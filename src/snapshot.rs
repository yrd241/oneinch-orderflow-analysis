//! Load Sankey orderflow from SQLite cache or demo data.
//!
//! When `user_7702_map` + `delegated_7702_labels` exist in the same SQLite DB and cached
//! `orderflow_view` rows are present, **`L1>L2` edges whose source is `User: EOA (Unlabeled)`**
//! are split by per-transaction `user` → EIP-7702 label (`User: EOA (7702 …)`), keeping all
//! **`L2>L3` … `L5>L6`** edges unchanged from Dune.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;
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
        let mut edges_for_payload = sankey_edges.clone();
        if let Some(c) = cache.as_ref() {
            if let Ok(of_rows) = c.load_kind(QueryKind::OrderflowView) {
                let has_map = sqlite_has_table(db, "user_7702_map");
                if !of_rows.is_empty() && has_map {
                    match merge_l1_unlabeled_with_7702(&sankey_edges, db, &of_rows) {
                        Ok(Some(merged)) => {
                            tracing::info!(
                                dune_edges = sankey_edges.len(),
                                merged_edges = merged.len(),
                                "Applied local EIP-7702 split to L1 edges"
                            );
                            edges_for_payload = merged;
                        }
                        Ok(None) => {}
                        Err(e) => tracing::warn!("7702 L1 merge skipped: {e}"),
                    }
                }
            }
        }
        return Ok(OrderflowSnapshot {
            source: "cache".into(),
            block_time_range,
            block_number_range,
            sankey: edges_to_payload(&edges_for_payload),
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

fn sqlite_has_table(db: &Path, table: &str) -> bool {
    let Ok(conn) = Connection::open(db) else {
        return false;
    };
    conn.prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1")
        .ok()
        .and_then(|mut s| s.exists(rusqlite::params![table]).ok())
        .unwrap_or(false)
}

fn load_7702_maps(db: &Path) -> anyhow::Result<(HashMap<String, String>, HashMap<String, String>)> {
    let conn = Connection::open(db)?;

    let mut user_to_del: HashMap<String, String> = HashMap::new();
    if conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='user_7702_map'")?
        .exists([])?
    {
        let mut stmt = conn.prepare("SELECT user, delegated_to FROM user_7702_map")?;
        let rows = stmt.query_map([], |row| {
            let u: String = row.get(0)?;
            let d: String = row.get(1)?;
            Ok((_normalize_addr_sql(&u), _normalize_addr_sql(&d)))
        })?;
        for r in rows {
            let (u, d) = r?;
            user_to_del.insert(u, d);
        }
    }

    let mut del_to_label: HashMap<String, String> = HashMap::new();
    if conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='delegated_7702_labels'")?
        .exists([])?
    {
        let mut stmt = conn.prepare("SELECT delegated_to, label FROM delegated_7702_labels")?;
        let rows = stmt.query_map([], |row| {
            let d: String = row.get(0)?;
            let l: String = row.get(1)?;
            Ok((_normalize_addr_sql(&d), l))
        })?;
        for r in rows {
            let (d, l) = r?;
            del_to_label.insert(d, l);
        }
    }

    Ok((user_to_del, del_to_label))
}

fn _normalize_addr_sql(s: &str) -> String {
    let mut a = s.trim().to_string();
    if !a.starts_with("0x") {
        a = format!("0x{a}");
    }
    a.to_lowercase()
}

fn l1_user_bucket_eoa_7702(
    user_lower: &str,
    user_to_del: &HashMap<String, String>,
    del_to_label: &HashMap<String, String>,
) -> String {
    if let Some(del) = user_to_del.get(user_lower) {
        if let Some(lab) = del_to_label.get(del) {
            return format!("User: EOA (7702 {lab})");
        }
        return "User: EOA (7702)".into();
    }
    "User: EOA (Unlabeled)".into()
}

fn q7_frontend_bucket(frontend: Option<&str>) -> String {
    let f = frontend.unwrap_or("Unknown");
    let keep = matches!(
        f,
        "1inch Integrators"
            | "1inch Website: Default"
            | "Trust Wallet"
            | "MetaMask Swaps"
            | "Binance Wallet"
            | "deBridge Frontend"
            | "Li.Fi Integrators"
            | "Rainbow Wallet"
            | "Cowswap Integrators"
            | "Fluid Frontend"
    );
    if keep {
        format!("Frontend: {f}")
    } else {
        "Frontend: Other Frontends".into()
    }
}

/// `orderflow_view` often has `solver = 'UNSPECIFIED'`; still attribute to 1inch Router flow.
fn row_matches_1inch_router_orderflow(solver_raw: &str) -> bool {
    let lower = solver_raw.trim().to_lowercase();
    lower.contains("1inch") || lower == "unspecified"
}

const UNLABELED_EOA_L1: &str = "User: EOA (Unlabeled)";

/// Split only Dune **`L1>L2`** edges from **`User: EOA (Unlabeled)` → `Frontend:*`** using
/// per-tx `orderflow_view` + local `user_7702_map` / `delegated_7702_labels`. All **`L2>L3`…`L5>L6`**
/// edges stay exactly as from Dune.
fn merge_l1_unlabeled_with_7702(
    dune_edges: &[SankeyEdgeRow],
    db: &Path,
    orderflow_rows: &[Value],
) -> anyhow::Result<Option<Vec<SankeyEdgeRow>>> {
    if !sqlite_has_table(db, "user_7702_map") {
        return Ok(None);
    }
    let (user_to_del, del_to_label) = match load_7702_maps(db) {
        Ok(x) => x,
        Err(_) => return Ok(None),
    };

    let mut unlabeled_to_frontend: HashMap<String, (f64, f64)> = HashMap::new();
    let mut rest: Vec<SankeyEdgeRow> = Vec::with_capacity(dune_edges.len());

    for e in dune_edges {
        if e.edge_level == "L1>L2" && e.source == UNLABELED_EOA_L1 {
            let ent = unlabeled_to_frontend
                .entry(e.target.clone())
                .or_insert((0.0, 0.0));
            ent.0 += e.tx_count;
            ent.1 += e.volume_m_usd;
        } else {
            rest.push(e.clone());
        }
    }

    if unlabeled_to_frontend.is_empty() {
        return Ok(None);
    }

    let mut agg: HashMap<(String, String), (f64, f64)> = HashMap::new();
    let mut seen_hash: HashSet<String> = HashSet::new();

    for v in orderflow_rows {
        let solver = v.get("solver").and_then(|x| x.as_str()).unwrap_or("");
        if !row_matches_1inch_router_orderflow(solver) {
            continue;
        }
        if let Some(h) = v.get("hash").and_then(|x| x.as_str()) {
            if !h.is_empty() && !seen_hash.insert(h.to_string()) {
                continue;
            }
        }
        let user_raw = match v.get("user").and_then(|x| x.as_str()) {
            Some(u) if !u.trim().is_empty() => _normalize_addr_sql(u),
            _ => continue,
        };
        let l1 = l1_user_bucket_eoa_7702(&user_raw, &user_to_del, &del_to_label);
        let l2 = q7_frontend_bucket(v.get("frontend").and_then(|x| x.as_str()));
        let trade_usd = json_f64_orderflow(v, "trade_usd").max(0.0);

        let key = (l1, l2);
        let ent = agg.entry(key).or_insert((0.0, 0.0));
        ent.0 += 1.0;
        ent.1 += trade_usd;
    }

    let mut new_l1l2: Vec<SankeyEdgeRow> = Vec::new();

    for (frontend_f, (old_tx, old_vol_m)) in unlabeled_to_frontend {
        let mut subs: Vec<(String, f64, f64)> = Vec::new();
        for ((l1, fe), (tx, vol_usd)) in &agg {
            if *fe != frontend_f {
                continue;
            }
            subs.push((l1.clone(), *tx, vol_usd / 1e6));
        }

        if subs.is_empty() {
            new_l1l2.push(SankeyEdgeRow {
                edge_level: "L1>L2".into(),
                source: UNLABELED_EOA_L1.into(),
                target: frontend_f,
                tx_count: old_tx,
                volume_m_usd: old_vol_m,
            });
            continue;
        }

        let total_vol_m: f64 = subs.iter().map(|(_, _, vm)| vm).sum();
        let total_tx: f64 = subs.iter().map(|(_, t, _)| t).sum();
        let equal_w = 1.0 / (subs.len() as f64);

        for (bucket, tx_b, vol_b_m) in subs {
            let w = if total_vol_m > f64::EPSILON {
                vol_b_m / total_vol_m
            } else if total_tx > f64::EPSILON {
                tx_b / total_tx
            } else {
                equal_w
            };
            let nv = old_vol_m * w;
            let nt = old_tx * w;
            if nv <= f64::EPSILON && nt <= f64::EPSILON {
                continue;
            }
            new_l1l2.push(SankeyEdgeRow {
                edge_level: "L1>L2".into(),
                source: bucket,
                target: frontend_f.clone(),
                tx_count: nt,
                volume_m_usd: nv,
            });
        }
    }

    let mut out = rest;
    out.extend(new_l1l2);
    Ok(Some(out))
}

fn json_f64_orderflow(v: &Value, key: &str) -> f64 {
    v.get(key)
        .and_then(|x| x.as_f64())
        .or_else(|| {
            v.get(key)
                .and_then(|x| x.as_i64())
                .map(|n| n as f64)
        })
        .or_else(|| {
            v.get(key)
                .and_then(|x| x.as_str())
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(0.0)
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
