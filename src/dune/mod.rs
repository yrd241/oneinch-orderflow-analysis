pub mod cache;
mod client;
pub mod config;

pub use cache::Cache;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::task::JoinSet;
use tracing::info;

use crate::cli::FetchArgs;
use crate::paths;
use crate::dune::client::DuneClient;
use crate::dune::config::QueryRegistry;

pub async fn run_fetch(args: FetchArgs) -> Result<()> {
    let db_path = resolve_db_path(args.db);
    let registry = QueryRegistry::from_env();
    if registry.entries().is_empty() {
        anyhow::bail!(
            "No Dune queries registered. This shouldn't happen — please report a bug."
        );
    }

    let api_key = std::env::var("DUNE_API_KEY").context(
        "Set DUNE_API_KEY (https://dune.com/settings/api) to fetch from Dune",
    )?;

    let client = Arc::new(DuneClient::new(api_key));

    // Execute all Dune queries concurrently.
    let mut set: JoinSet<Result<_>> = JoinSet::new();
    for entry in registry.entries() {
        let client = Arc::clone(&client);
        let id = entry.id;
        let kind = entry.kind;
        set.spawn(async move {
            info!("Executing Dune query {id}");
            let rows = client.execute_and_poll_results(id).await?;
            info!("Fetched {} rows for query {id} ({kind:?})", rows.len());
            Ok((kind, rows))
        });
    }

    // Collect results then write to cache sequentially (Connection is not Sync).
    let mut results = Vec::new();
    while let Some(res) = set.join_next().await {
        results.push(res??);
    }

    let mut cache = Cache::open(&db_path)?;
    for (kind, rows) in results {
        cache.insert_rows(kind, &rows)?;
        info!("Cached {} rows for {kind:?}", rows.len());
    }

    Ok(())
}

fn resolve_db_path(override_path: Option<PathBuf>) -> PathBuf {
    override_path.unwrap_or_else(paths::default_cache_db)
}
