# oneinch-orderflow

Local dashboard and data pipeline for analyzing **1inch Router orderflow** — who sends orders where, and how they get routed through the MEV stack.

## What it shows

The main view is a 6-layer interactive Sankey:

```
User Class → Frontend → Solver → Mempool → OFA → Builder
```

This lets you answer questions like:

- Which frontends route volume through 1inch?
- What share of flow is private mempool vs public?
- Which OFA mechanisms (or none) are used?
- Which builders construct the final blocks?

Additional pages break down user segments (labeled addresses, smart wallets, EOAs, EIP-7702 delegated accounts).

## Stack

- **Backend**: Rust (Axum, SQLite, async Dune API client)
- **Frontend**: Vanilla HTML/JS with ECharts Sankey — no build step
- **Data**: Dune Analytics queries over Flashbots-indexed 1inch transaction data

## Quick start

```bash
cargo build --release

export DUNE_API_KEY=<your key from dune.com/settings/api>
export DUNE_USE_FLASHBOTS_DEFAULTS=1

./target/release/orderflow fetch   # pull data from Dune → SQLite cache
./target/release/orderflow serve   # start server at http://127.0.0.1:3000
```

## Optional: run without fetching (download demo DB)

If you just want to **demo the UI** without running `fetch` (and without a Dune API key), you can download a prebuilt SQLite snapshot from **GitHub Releases** and start `serve` pointing at it.

1) Download `orderflow.db` from the latest release (Assets) and save it locally (example: `./demo/orderflow.db`).

2) Run the server using that DB:

```bash
./target/release/orderflow serve --db ./demo/orderflow.db
```

Notes:

- The demo DB is a **time-window snapshot**, not live data.
- If you want fresh data, run `orderflow fetch` to rebuild your own local cache.

## CLI commands

| Command | Description |
|---------|-------------|
| `orderflow fetch` | Execute Dune queries and write results to local SQLite cache |
| `orderflow serve` | Serve the web UI and `GET /api/summary` JSON endpoint |
| `orderflow export` | Write the current snapshot to a JSON file (same shape as `/api/summary`) |

**Common flags:**

```
--db <path>        SQLite database path (default: ~/.cache/oneinch-orderflow/orderflow.db)
--host <addr>      Bind address for serve (default: 127.0.0.1)
--port <n>         Port for serve (default: 3000)
--demo             Serve even if cache is empty (shows placeholder data)
```

All flags can be set via environment variables (see `orderflow --help`).

## Environment variables

| Variable | Required | Description |
|----------|----------|-------------|
| `DUNE_API_KEY` | Yes | API key from dune.com/settings/api |
| `DUNE_USE_FLASHBOTS_DEFAULTS` | No | Set to `1` to use built-in default query IDs |
| `DUNE_QUERY_1INCH_SANKEY` | No | Override the Sankey query ID (default: `7428851`) |
| `DUNE_QUERY_ORDERFLOW` | No | Override the per-tx orderflow query ID |
| `ORDERFLOW_DB` | No | Override the SQLite database path |
| `DUNE_HTTP_TIMEOUT_SECS` | No | HTTP timeout in seconds (default: 600) |
| `DUNE_HTTP_RETRIES` | No | Retry attempts on transient errors (default: 5) |
| `DUNE_MAX_WAIT_SECS` | No | Max seconds to wait for a Dune execution (default: 3600) |
| `ORDERFLOW_WEB_ROOT` | No | Override the web static files directory |

## Data model

### Sankey edges (`/api/summary`)

Edges come from the cached `1inch_sankey` query rows. Each row must have:

| Field | Type | Description |
|-------|------|-------------|
| `edge_level` | string | e.g. `"L1>L2"` through `"L5>L6"` |
| `source` | string | Source node label |
| `target` | string | Target node label |
| `tx_count` | number | Transaction count |
| `volume_m_usd` | number | Volume in millions USD |

### User classification

The Sankey query classifies `user` addresses into three buckets:

- **Labeled** — best-effort join to a public labels table
- **Smart wallet** — Safe or other known contract wallets
- **EOA** — unlabeled externally owned accounts, further bucketed by activity pattern

EIP-7702 delegated accounts are enriched locally from `eoa_7702_resolved.csv`.

### SQLite cache

One table: `raw_rows(kind, payload, ingested_at)`. Each `fetch` overwrites all rows for the given kind.

## Dune queries

| File | Query ID | Purpose |
|------|----------|---------|
| `dune/queries/07_1inch_sankey.sql` | `7428851` | Main Sankey: 6-layer edges with user classification |
| `dune/queries/01_orderflow_view.sql` | `3184593` | Per-transaction trade details |
| `dune/queries/00_flashbots_reference.sql` | — | Reference: Flashbots source table schema |
| `dune/queries/03_frontend_resolver.sql` | — | Maps addresses to frontend names |
| `dune/queries/04_topn_pairs.sql` | — | High-volume trading pairs |
| `dune/queries/05_volume_timeseries.sql` | — | Time-bucketed volume trends |
| `dune/queries/06_wallet_app.sql` | — | Wallet integration metrics |
| `dune/queries/08_integrator_txs.sql` | — | 1inch integrator/partner transactions |

The main query reads from `dune.flashbots.result_overall_of` — a materialized table maintained by Flashbots. If `max(block_time)` looks stale, check whether that upstream table is still updating.

## Acknowledgements

Inspired by [Orderflow.art](https://github.com/flashbots/Orderflow.art) by Flashbots and Barter — the original work that made orderflow routing visible and legible across the Ethereum block-building stack. The flow model, layer taxonomy, and general framing used here are directly informed by their approach. This project applies the same lens to 1inch-specific flow.

## License

MIT
