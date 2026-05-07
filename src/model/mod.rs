pub mod sankey;

/// Cached table kinds matching Dune query outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QueryKind {
    FrontendResolver,
    TopN,
    Timeseries,
    WalletApp,
    /// Legacy Flashbots orderflow view (per-tx). Kept so snapshot fallbacks
    /// can still read older caches that predate the inline `USER_ADDR` rows.
    OrderflowView,
    /// 1inch router Sankey edge data (source/target/tx_count/volume_m_usd per layer)
    OneinchSankey,
}

impl QueryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            QueryKind::FrontendResolver => "frontend_resolver",
            QueryKind::TopN => "topn",
            QueryKind::Timeseries => "timeseries",
            QueryKind::WalletApp => "wallet_app",
            QueryKind::OrderflowView => "orderflow_view",
            QueryKind::OneinchSankey => "1inch_sankey",
        }
    }
}
