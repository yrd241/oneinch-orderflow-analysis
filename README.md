# orderflow_cli

Local orderflow dashboard for **1inch router-style flows**.

This Rust CLI:

- executes configured **Dune query id(s)** via Dune HTTP API,
- stores returned **JSON rows** in a local **SQLite** cache,
- serves a static UI (`web/`) and `GET /api/summary`,
- renders an **ECharts Sankey**.

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

- `orderflow fetch`: run configured Dune query ids → refresh SQLite cache
- `orderflow serve`: serve UI + `GET /api/summary` (defaults: `--host 127.0.0.1 --port 3000 --demo true`)
- `orderflow export`: write current snapshot to `export.json`

DB path override: `--db <path>` or `ORDERFLOW_DB=<path>` (default `~/.cache/orderflow_cli/orderflow.db`).

## Query IDs

`fetch` decides what to execute from env vars:

- `DUNE_QUERY_1INCH_SANKEY` → cache kind `1inch_sankey`
- (optional) `DUNE_QUERY_ORDERFLOW`, `DUNE_QUERY_LIQUIDITY`, … are supported by the code, but the current UI only renders the Sankey.

If no `DUNE_QUERY_*` are set and `DUNE_USE_FLASHBOTS_DEFAULTS=1`, the code executes the built-in default:

- `QUERY_1INCH_SANKEY = 7428851`

## Sankey (what the UI renders)

The UI reads `GET /api/summary` and renders `data.sankey`.

Real sankey edges come from cached rows of kind `1inch_sankey` and must include:

- `edge_level`: `"L1>L2"` … `"L5>L6"`
- `source`, `target`
- `tx_count`
- `volume_m_usd` (millions of USD; converted to USD in API payload as `volume_usd`)

### Reference SQL

`dune/queries/07_1inch_sankey.sql` is a **reference** Dune SQL that emits the expected shape.
It models a 6-layer flow:

`User class → Frontend → Solver → Mempool → OFA → Builder`

The reference query classifies `user` addresses into a small number of buckets:

- labeled address (best-effort join to a public labels table)
- smart wallet (Safe / other contracts)
- unlabeled EOA (bucketed)

Note: label and smart-wallet joins are **best-effort** and may require adjusting the source tables
to what’s available in your Dune environment.

## Storage (SQLite cache)

The cache stores raw JSON rows per kind:

- `raw_rows(kind, payload, ingested_at)` (overwritten per kind on each fetch)

## License

MIT
