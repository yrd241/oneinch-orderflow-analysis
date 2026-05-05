pub mod sankey;

/// Cached table kinds matching Dune query outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QueryKind {
    FrontendResolver,
    TopN,
    Timeseries,
    WalletApp,
    /// Flashbots orderflow view (one row per user trade, includes block_time + block_number).
    OrderflowView,
    /// Flashbots liquidity view (per-hop legs, includes block_time + block_number).
    LiquidityView,
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
            QueryKind::LiquidityView => "liquidity_view",
            QueryKind::OneinchSankey => "1inch_sankey",
        }
    }
}
