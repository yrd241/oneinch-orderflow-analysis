# oneinch-orderflow-analysis

Local dashboard + data pipeline for **deep analysis of 1inch orderflow**.

The goal is to make “who sent orders where” concrete and measurable:

- **Which frontends** route through 1inch
- **How solvers source flow** (public vs private mempool)
- **Whether orders go through OFA / which OFA**
- **Which builders** ultimately build the blocks

The UI renders this as a **multi-layer Sankey** so you can inspect the dominant paths at a glance.

## Quick start

```bash
cargo build --release

export DUNE_API_KEY=your_key
export DUNE_USE_FLASHBOTS_DEFAULTS=1

./target/release/orderflow fetch
./target/release/orderflow serve
```

Open `http://127.0.0.1:3000`.

## CLI

- `orderflow fetch`: execute Dune queries → refresh local SQLite cache
- `orderflow serve`: serve UI + `GET /api/summary` (defaults: `--host 127.0.0.1 --port 3000 --demo true`)
- `orderflow export`: export current snapshot JSON (same payload as `/api/summary`)

DB path override: `--db <path>` or `ORDERFLOW_DB=<path>` (default `~/.cache/oneinch-orderflow/orderflow.db`).

## Query IDs

`fetch` decides what to execute from env vars:

- `DUNE_QUERY_1INCH_SANKEY` → cache kind `1inch_sankey`
- (optional) `DUNE_QUERY_ORDERFLOW`, `DUNE_QUERY_LIQUIDITY`, … can be cached as well; the UI currently focuses on the Sankey snapshot.

If no `DUNE_QUERY_*` are set and `DUNE_USE_FLASHBOTS_DEFAULTS=1`, the code executes the built-in default:

- `QUERY_1INCH_SANKEY = 7428851`

## Sankey (what the UI renders)

The UI reads `GET /api/summary` and renders `data.sankey`.

Real sankey edges come from cached rows of kind `1inch_sankey` and must include:

- `edge_level`: `"L1>L2"` … `"L5>L6"`
- `source`, `target`
- `tx_count`
- `volume_m_usd` (millions of USD; converted to USD in API payload as `volume_usd`)

### Flow model (layers)

The reference Sankey models a 6-layer path:

`User class → Frontend → Solver (1inch Router) → Mempool → OFA → Builder`

This lets you answer questions like:

- Is flow coming from a few major frontends, or long tail?
- What share is **private** vs **public** mempool?
- Which OFA endpoints (or “None”) are dominant?
- Which builders dominate for the flow that matters?

### Reference SQL

`dune/queries/07_1inch_sankey.sql` is a **reference** Dune SQL that emits the expected shape.

The reference query classifies `user` addresses into a small number of buckets:

- labeled address (best-effort join to a public labels table)
- smart wallet (Safe / other contracts)
- unlabeled EOA (bucketed)

Note: label and smart-wallet joins are **best-effort** and may require adjusting the source tables
to what’s available in your Dune environment.

## Storage (SQLite cache)

The cache stores raw JSON rows per kind:

- `raw_rows(kind, payload, ingested_at)` (overwritten per kind on each fetch)

## Notes on “freshness”

If your Dune sources are materialized results (e.g. `dune.flashbots.result_*`), they may have ingestion delays or stop updating.
If you see stale `max(block_time)`, confirm the upstream table/view is still updating, or fork the query onto a newer dataset.

## License

MIT
