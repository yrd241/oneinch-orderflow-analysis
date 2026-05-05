use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::Value;
use tokio::time::sleep;
use tracing::info;

const DUNE_API_BASE: &str = "https://api.dune.com/api/v1";
/// Dune uses this header, not `Authorization: Bearer` (see Dune API authentication docs).
static DUNE_API_KEY_HEADER: HeaderName = HeaderName::from_static("x-dune-api-key");

pub struct DuneClient {
    http: reqwest::Client,
}

impl DuneClient {
    pub fn new(api_key: String) -> Self {
        let mut headers = HeaderMap::new();
        let mut key = HeaderValue::from_str(api_key.trim())
            .expect("invalid API key characters");
        key.set_sensitive(true);
        headers.insert(DUNE_API_KEY_HEADER.clone(), key);

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest client");

        Self { http }
    }

    /// POST /query/{id}/execute then poll GET /execution/{id}/results until success.
    pub async fn execute_and_poll_results(&self, query_id: u64) -> Result<Vec<Value>> {
        let exec_url = format!("{}/query/{}/execute", DUNE_API_BASE, query_id);
        let response = self
            .http
            .post(&exec_url)
            .json(&serde_json::json!({}))
            .send()
            .await
            .context("execute query")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(anyhow!(
                    "Dune API returned 401 Unauthorized. \
                     (1) Set DUNE_API_KEY to a key from https://dune.com/settings/api \
                     (2) The key must allow **query execution** (not read-only, if your org splits scopes). \
                     (3) If executing someone else's query, fork it to your account and use the new query id. \
                     Response: {}",
                    body
                ));
            }
            return Err(anyhow!("Dune execute failed: {} — {}", status, body));
        }

        let exec: ExecuteResponse = response
            .json()
            .await
            .context("parse execute response")?;

        let execution_id = exec
            .execution_id
            .ok_or_else(|| anyhow!("missing execution_id"))?;

        info!(
            query_id,
            %execution_id,
            "Dune accepted execution; polling status (large queries can take many minutes on the free tier)"
        );

        let max_wait = std::env::var("DUNE_MAX_WAIT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600u64);
        let t0 = Instant::now();
        let deadline = t0 + Duration::from_secs(max_wait);

        let mut delay = Duration::from_secs(2);
        let mut last_state = String::new();
        let mut poll_n: u32 = 0;

        while Instant::now() < deadline {
            poll_n += 1;
            let row = self.execution_state(&execution_id).await?;

            if row.state != last_state {
                info!(
                    query_id,
                    %execution_id,
                    state = %row.state,
                    "Dune execution status"
                );
                last_state = row.state.clone();
            } else if poll_n.is_multiple_of(5) {
                // Heartbeat: not hung — big queries can sit in PENDING/EXECUTING for a long time
                info!(
                    query_id,
                    %execution_id,
                    state = %row.state,
                    polls = poll_n,
                    elapsed_sec = t0.elapsed().as_secs(),
                    "still waiting on Dune"
                );
            }

            match row.state.as_str() {
                "QUERY_STATE_COMPLETED"
                | "QUERY_STATE_COMPLETE"
                | "QUERY_STATE_COMPLETED_PARTIAL" => {
                    return self.execution_results(&execution_id).await;
                }
                "QUERY_STATE_FAILED" | "QUERY_STATE_EXPIRED" | "QUERY_STATE_CANCELED" => {
                    return Err(anyhow!(
                        "Dune execution ended with {} (execution_id={}): error={:?}",
                        row.state,
                        execution_id,
                        row.error
                    ));
                }
                _ => {}
            }

            sleep(delay).await;
            delay = (delay * 120 / 100).min(Duration::from_secs(15));
        }

        Err(anyhow!(
            "Dune execution timed out after {}s (execution still {}). \
             Large Flashbots queries often exceed a few minutes — raise DUNE_MAX_WAIT_SECS or run a smaller forked query.",
            max_wait,
            last_state
        ))
    }

    async fn execution_state(&self, execution_id: &str) -> Result<ExecutionStateRow> {
        let url = format!("{}/execution/{}/status", DUNE_API_BASE, execution_id);
        let row: ExecutionStateRow = self
            .http
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(row)
    }

    /// Fetch all result pages for a completed execution, following `next_uri` pagination.
    async fn execution_results(&self, execution_id: &str) -> Result<Vec<Value>> {
        const MAX_PAGES: usize = 200;

        let first_url = format!("{}/execution/{}/results", DUNE_API_BASE, execution_id);
        let mut next: Option<String> = Some(first_url);
        let mut all_rows: Vec<Value> = Vec::new();
        let mut pages_fetched: usize = 0;

        while let Some(url) = next {
            pages_fetched += 1;
            if pages_fetched > MAX_PAGES {
                return Err(anyhow!(
                    "Dune results for execution {execution_id} exceeded {MAX_PAGES} pages; \
                     aborting to prevent runaway pagination"
                ));
            }

            let page: ResultsPage = self
                .http
                .get(&url)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            let page_len = page.result.rows.len();
            all_rows.extend(page.result.rows);

            next = page.next_uri;
            if next.is_some() {
                info!(
                    execution_id,
                    fetched = all_rows.len(),
                    page_rows = page_len,
                    pages = pages_fetched,
                    "paginating Dune results"
                );
            }
        }

        Ok(all_rows)
    }
}

#[derive(Debug, Deserialize)]
struct ExecuteResponse {
    execution_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExecutionStateRow {
    state: String,
    #[serde(default)]
    error: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ResultsPage {
    result: ResultsInner,
    /// Present when more pages are available; fetch this URL for the next batch.
    next_uri: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResultsInner {
    rows: Vec<Value>,
}
