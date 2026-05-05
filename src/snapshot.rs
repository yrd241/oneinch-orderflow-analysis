//! Load Sankey orderflow from SQLite cache or demo data.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use crate::dune::Cache;
use crate::model::sankey::{demo_flow, extract_time_range, LayeredFlow, SankeyEdgeRow};
use crate::model::QueryKind;

#[derive(Debug, Clone, Serialize)]
pub struct OrderflowSnapshot {
    pub source: String,
    /// UTC block_time range of the included transactions: [min, max].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_time_range: Option<[String; 2]>,
    /// Block number range of the included transactions: [min, max].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number_range: Option<[u64; 2]>,
    pub sankey: SankeyPayload,
}

#[derive(Debug, Clone, Serialize)]
pub struct SankeyPayload {
    pub nodes: Vec<SankeyNode>,
    pub links: Vec<SankeyLink>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SankeyNode {
    pub name: String,
    pub depth: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SankeyLink {
    pub source: String,
    pub target: String,
    /// Sankey flow width (tx count from real data, or USD volume from demo).
    pub value: f64,
    /// USD volume — shown in tooltip when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_usd: Option<f64>,
}

pub fn load_snapshot(db: &Path, allow_demo: bool) -> Result<OrderflowSnapshot> {
    let cache = Cache::open(db)
        .inspect_err(|e| tracing::warn!("cache open failed: {e}"))
        .ok();

    let raw_rows = cache
        .as_ref()
        .and_then(|c| c.load_kind(QueryKind::OneinchSankey).ok())
        .unwrap_or_default();

    let sankey_edges: Vec<SankeyEdgeRow> = raw_rows
        .iter()
        .filter_map(SankeyEdgeRow::from_value)
        .collect();

    // META row: UTC block_time range (min/max) for all included transactions.
    // Keep this independent from whether edge rows successfully parsed.
    let mut block_time_range = extract_time_range(&raw_rows).map(|(min, max)| [min, max]);
    let mut block_number_range: Option<[u64; 2]> = None;

    // Fallback: if the sankey query didn't emit the META row, derive the time/block range
    // from the cached orderflow view, which includes per-tx block_time + block_number.
    if block_time_range.is_none() {
        if let Some(c) = cache.as_ref() {
            if let Ok(rows) = c.load_kind(QueryKind::OrderflowView) {
                let (t, b) = extract_time_and_block_range(&rows);
                block_time_range = t;
                block_number_range = b;
            }
        }
    }

    if !sankey_edges.is_empty() {
        return Ok(OrderflowSnapshot {
            source: "cache".into(),
            block_time_range,
            block_number_range,
            sankey: edges_to_payload(&sankey_edges),
        });
    }

    // If we at least have time-range metadata from cache, show it even when
    // edge rows are empty (parse mismatch or other unexpected cache shape).
    if block_time_range.is_some() {
        return Ok(OrderflowSnapshot {
            source: "cache".into(),
            block_time_range,
            block_number_range,
            sankey: SankeyPayload { nodes: vec![], links: vec![] },
        });
    }

    if allow_demo {
        return Ok(OrderflowSnapshot {
            source: "demo".into(),
            block_time_range: None,
            block_number_range: None,
            sankey: layered_flow_to_payload(&demo_flow()),
        });
    }

    Ok(OrderflowSnapshot {
        source: "empty".into(),
        block_time_range: None,
        block_number_range: None,
        sankey: SankeyPayload { nodes: vec![], links: vec![] },
    })
}

fn extract_time_and_block_range(rows: &[Value]) -> (Option<[String; 2]>, Option<[u64; 2]>) {
    let mut min_block: Option<u64> = None;
    let mut max_block: Option<u64> = None;
    let mut min_time: Option<String> = None;
    let mut max_time: Option<String> = None;

    for v in rows {
        let b = json_u64(v, "block_number");
        let t = v
            .get("block_time")
            .and_then(|x| x.as_str())
            .map(normalize_block_time);

        if let (Some(bn), Some(bt)) = (b, t) {
            let is_new_min = min_block.map(|x| bn < x).unwrap_or(true);
            if is_new_min {
                min_block = Some(bn);
                min_time = Some(bt.clone());
            }
            let is_new_max = max_block.map(|x| bn > x).unwrap_or(true);
            if is_new_max {
                max_block = Some(bn);
                max_time = Some(bt);
            }
        }
    }

    let time_range = match (min_time, max_time) {
        (Some(a), Some(b)) => Some([a, b]),
        _ => None,
    };
    let block_range = match (min_block, max_block) {
        (Some(a), Some(b)) => Some([a, b]),
        _ => None,
    };
    (time_range, block_range)
}

fn normalize_block_time(s: &str) -> String {
    // Dune result strings often look like: "2025-09-26 23:52:35.000 UTC"
    // Normalize to "YYYY-MM-DD HH:MM:SS" (UTC) so the UI can parse reliably.
    let mut out = s.trim().to_string();
    if let Some(stripped) = out.strip_suffix(" UTC") {
        out = stripped.to_string();
    }
    if let Some((a, _frac)) = out.split_once('.') {
        out = a.to_string();
    }
    out
}

fn json_u64(v: &Value, key: &str) -> Option<u64> {
    v.get(key)
        .and_then(|x| x.as_u64())
        .or_else(|| v.get(key).and_then(|x| x.as_i64()).and_then(|n| u64::try_from(n).ok()))
        .or_else(|| v.get(key).and_then(|x| x.as_str()).and_then(|s| s.parse().ok()))
}

pub fn layered_flow_to_payload(flow: &LayeredFlow) -> SankeyPayload {
    let mut depth: HashMap<String, u32> = HashMap::new();
    for (i, layer) in flow.layers.iter().enumerate() {
        let i = i as u32;
        for name in layer.keys() {
            depth.entry(name.clone()).or_insert(i);
        }
    }

    let mut names = HashSet::new();
    for e in &flow.edges {
        names.insert(e.from.clone());
        names.insert(e.to.clone());
    }

    let mut nodes: Vec<SankeyNode> = names
        .into_iter()
        .map(|name| SankeyNode {
            depth: depth.get(&name).copied(),
            name,
        })
        .collect();
    nodes.sort_by(|a, b| {
        (a.depth.unwrap_or(999), &a.name).cmp(&(b.depth.unwrap_or(999), &b.name))
    });

    let links: Vec<SankeyLink> = flow
        .edges
        .iter()
        .map(|e| SankeyLink {
            source: e.from.clone(),
            target: e.to.clone(),
            value: e.value,
            volume_usd: None,
        })
        .collect();

    SankeyPayload { nodes, links }
}

pub fn edges_to_payload(edges: &[SankeyEdgeRow]) -> SankeyPayload {
    let mut depth: HashMap<String, u32> = HashMap::new();
    for e in edges {
        depth.entry(e.source.clone()).or_insert(e.source_depth());
        depth.entry(e.target.clone()).or_insert(e.target_depth());
    }

    let mut nodes: Vec<SankeyNode> = depth
        .iter()
        .map(|(name, &d)| SankeyNode {
            name: name.clone(),
            depth: Some(d),
        })
        .collect();
    nodes.sort_by(|a, b| {
        (a.depth.unwrap_or(999), &a.name).cmp(&(b.depth.unwrap_or(999), &b.name))
    });

    let links: Vec<SankeyLink> = edges
        .iter()
        .map(|e| SankeyLink {
            source: e.source.clone(),
            target: e.target.clone(),
            value: e.tx_count,
            volume_usd: Some(e.volume_m_usd * 1e6),
        })
        .collect();

    SankeyPayload { nodes, links }
}
