//! Register Dune query IDs after you publish queries on Dune.
//!
//! ## Data source
//!
//! The 1inch Router Sankey reads `dune.flashbots.result_overall_of`.
//! Default query ID: `QUERY_1INCH_SANKEY` (7428851) — see dune/queries/07_1inch_sankey.sql.
//!
//! Set `DUNE_USE_FLASHBOTS_DEFAULTS=1` to use that query ID when `DUNE_QUERY_*` are unset.
//!
//! Env vars:
//! - `DUNE_QUERY_1INCH_SANKEY`
//! - `DUNE_USE_FLASHBOTS_DEFAULTS` — `1` = default sankey Q7428851

use crate::model::QueryKind;

/// 1inch router Sankey edge query (reads dune.flashbots.result_overall_of).
/// See dune/queries/07_1inch_sankey.sql
pub const QUERY_1INCH_SANKEY: u64 = 7_428_851;

/// Flashbots public orderflow view (one row per trade; includes `user`, `solver`, `frontend`).
/// See `dune/queries/01_orderflow_view.sql`.
pub const QUERY_ORDERFLOW_VIEW: u64 = 3_184_593;

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
        if let Some(id) = env_u64("DUNE_QUERY_1INCH_SANKEY") {
            entries.push(QueryEntry { id, kind: QueryKind::OneinchSankey });
        }

        if entries.is_empty() && use_flashbots_defaults() {
            tracing::info!("Using Flashbots defaults: sankey Q{}", QUERY_1INCH_SANKEY);
            entries.push(QueryEntry {
                id: QUERY_1INCH_SANKEY,
                kind: QueryKind::OneinchSankey,
            });
        }

        if entries.is_empty() {
            tracing::warn!(
                "No DUNE_QUERY_* env vars set; fetch will have nothing to run. \
                 Set DUNE_QUERY_1INCH_SANKEY or DUNE_USE_FLASHBOTS_DEFAULTS=1 (see README)."
            );
        }

        Self { entries }
    }

    pub fn entries(&self) -> &[QueryEntry] {
        &self.entries
    }
}

fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|s| s.parse().ok())
}

fn use_flashbots_defaults() -> bool {
    matches!(
        std::env::var("DUNE_USE_FLASHBOTS_DEFAULTS")
            .map(|s| s == "1" || s.eq_ignore_ascii_case("true")),
        Ok(true)
    )
}
