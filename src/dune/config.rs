//! Register Dune query IDs after you publish queries on Dune.
//!
//! ## Data source
//!
//! The 1inch Router Sankey reads `dune.flashbots.result_overall_of`.
//! Default query ID: `QUERY_1INCH_SANKEY` (7428851) — see dune/queries/07_1inch_sankey.sql.
//!
//! This default is always registered when no `DUNE_QUERY_1INCH_SANKEY` override
//! is set, so `fetch` works out of the box with just `DUNE_API_KEY`.
//!
//! Env vars:
//! - `DUNE_QUERY_1INCH_SANKEY` — override the default sankey query ID

use crate::model::QueryKind;

/// 1inch router Sankey edge query (reads dune.flashbots.result_overall_of).
/// See dune/queries/07_1inch_sankey.sql
pub const QUERY_1INCH_SANKEY: u64 = 7_428_851;

pub struct QueryEntry {
    pub id: u64,
    pub kind: QueryKind,
}

pub struct QueryRegistry {
    entries: Vec<QueryEntry>,
}

impl QueryRegistry {
    pub fn from_env() -> Self {
        let mut entries = Vec::new();
        if let Some(id) = env_u64("DUNE_QUERY_FRONTEND_RESOLVER") {
            entries.push(QueryEntry { id, kind: QueryKind::FrontendResolver });
        }
        if let Some(id) = env_u64("DUNE_QUERY_TOPN") {
            entries.push(QueryEntry { id, kind: QueryKind::TopN });
        }
        if let Some(id) = env_u64("DUNE_QUERY_TIMESERIES") {
            entries.push(QueryEntry { id, kind: QueryKind::Timeseries });
        }
        if let Some(id) = env_u64("DUNE_QUERY_WALLET_APP") {
            entries.push(QueryEntry { id, kind: QueryKind::WalletApp });
        }

        // The 1inch sankey query is the only data source the dashboard needs,
        // so register the Flashbots default when no override is supplied.
        let sankey_id = env_u64("DUNE_QUERY_1INCH_SANKEY").unwrap_or(QUERY_1INCH_SANKEY);
        if sankey_id == QUERY_1INCH_SANKEY {
            tracing::info!("Using default sankey query Q{QUERY_1INCH_SANKEY}");
        } else {
            tracing::info!("Using overridden sankey query Q{sankey_id}");
        }
        entries.push(QueryEntry {
            id: sankey_id,
            kind: QueryKind::OneinchSankey,
        });

        Self { entries }
    }

    pub fn entries(&self) -> &[QueryEntry] {
        &self.entries
    }
}

fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|s| s.parse().ok())
}
