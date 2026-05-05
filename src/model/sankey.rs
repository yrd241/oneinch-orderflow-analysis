//! Sankey graph types: layered demo flow + real edge rows from the Dune cache.

use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FlowEdge {
    pub from: String,
    pub to: String,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct LayeredFlow {
    /// layer index -> node id -> total value passing through the node
    pub layers: Vec<HashMap<String, f64>>,
    pub edges: Vec<FlowEdge>,
}

impl LayeredFlow {
    pub fn from_pairs(layers: Vec<Vec<(String, f64)>>, edges: Vec<FlowEdge>) -> Self {
        let mut built = Vec::new();
        for layer in layers {
            let mut m = HashMap::new();
            for (name, v) in layer {
                *m.entry(name).or_insert(0.0) += v;
            }
            built.push(m);
        }
        Self {
            layers: built,
            edges,
        }
    }
}

/// Demo: Frontend → Product → Liquidity (three layers).
pub fn demo_flow() -> LayeredFlow {
    let frontends: Vec<(String, f64)> = vec![
        ("1inch dApp".into(), 432e6),
        ("MetaMask Swaps".into(), 851e6),
        ("Matcha".into(), 120e6),
        ("Rabby".into(), 95e6),
        ("Other".into(), 210e6),
    ];
    let products: Vec<(String, f64)> = vec![
        ("Aggregation Router".into(), 1208e6),
        ("1inch Fusion".into(), 350e6),
        ("Limit Orders".into(), 150e6),
    ];
    let liquidity: Vec<(String, f64)> = vec![
        ("Uniswap v3".into(), 620e6),
        ("Curve".into(), 180e6),
        ("RFQ / PMM".into(), 908e6),
    ];

    let total: f64 = frontends.iter().map(|(_, v)| *v).sum();
    let liq_total: f64 = liquidity.iter().map(|(_, v)| *v).sum();

    let mut edges = Vec::new();
    for (fe, v) in &frontends {
        edges.push(FlowEdge { from: fe.clone(), to: "Aggregation Router".into(), value: v * 0.85 });
        edges.push(FlowEdge { from: fe.clone(), to: "1inch Fusion".into(),        value: v * 0.10 });
        edges.push(FlowEdge { from: fe.clone(), to: "Limit Orders".into(),        value: v * 0.05 });
    }

    // Product inflows from the 85/10/5 split above.
    let product_inflows = [
        ("Aggregation Router", total * 0.85),
        ("1inch Fusion",       total * 0.10),
        ("Limit Orders",       total * 0.05),
    ];
    for (p_name, p_inflow) in product_inflows {
        for (liq_name, liq_val) in &liquidity {
            edges.push(FlowEdge {
                from: p_name.into(),
                to: liq_name.clone(),
                value: p_inflow * liq_val / liq_total,

            });
        }
    }

    LayeredFlow::from_pairs(vec![frontends, products, liquidity], edges)
}

/// One row from the 1inch Sankey edge query (Q7 / QueryKind::OneinchSankey).
///
/// `edge_level` encodes the layer transition: "L1>L2", "L2>L3", "L3>L4", "L4>L5".
#[derive(Debug, Clone)]
pub struct SankeyEdgeRow {
    pub edge_level: String,
    pub source: String,
    pub target: String,
    pub tx_count: f64,
    pub volume_m_usd: f64,
}

impl SankeyEdgeRow {
    /// Returns `None` for META rows (time-range metadata) and malformed rows.
    pub fn from_value(v: &Value) -> Option<Self> {
        let edge_level = v.get("edge_level")?.as_str()?;
        if edge_level == "META" {
            return None;
        }
        Some(Self {
            edge_level: edge_level.to_string(),
            source: v.get("source")?.as_str()?.to_string(),
            target: v.get("target")?.as_str()?.to_string(),
            tx_count: json_f64(v, "tx_count"),
            volume_m_usd: json_f64(v, "volume_m_usd"),
        })
    }

    /// Depth of the source node (0-based layer index).
    pub fn source_depth(&self) -> u32 {
        self.layer_from().saturating_sub(1)
    }

    /// Depth of the target node (0-based layer index).
    pub fn target_depth(&self) -> u32 {
        self.layer_to().saturating_sub(1)
    }

    fn layer_from(&self) -> u32 {
        // edge_level prefix: "L1>L2", "L2>L3", …
        self.edge_level
            .split('>')
            .next()
            .and_then(|s| s.trim_start_matches('L').parse().ok())
            .unwrap_or(1)
    }

    fn layer_to(&self) -> u32 {
        self.edge_level
            .split('>')
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.trim_start_matches('L').parse().ok())
            .unwrap_or(2)
    }
}

/// Extract the block_time range from the META row produced by Q7.
/// Returns `(min_block_time, max_block_time)` as UTC strings, or `None`.
pub fn extract_time_range(rows: &[Value]) -> Option<(String, String)> {
    rows.iter().find_map(|v| {
        if v.get("edge_level")?.as_str()? != "META" {
            return None;
        }
        let min_t = v.get("source")?.as_str()?.to_string();
        let max_t = v.get("target")?.as_str()?.to_string();
        Some((min_t, max_t))
    })
}

fn json_f64(v: &Value, key: &str) -> f64 {
    v.get(key)
        .and_then(|x| x.as_f64())
        .or_else(|| {
            v.get(key)
                .and_then(|x| x.as_str())
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(0.0)
}
