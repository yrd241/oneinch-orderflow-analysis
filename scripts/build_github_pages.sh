#!/usr/bin/env bash
# Build static JSON for GitHub Pages (publish the web/ directory).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Cursor sandbox may set CARGO_TARGET_DIR to a cache dir without a fresh binary.
unset CARGO_TARGET_DIR

DB="${ORDERFLOW_DB:-$ROOT/database/orderflow.db}"
BIN="${ORDERFLOW_BIN:-$ROOT/target/release/orderflow}"

if [[ ! -x "$BIN" ]]; then
  echo "Building orderflow…"
  cargo build --release
fi

if [[ ! -f "$DB" ]]; then
  echo "Missing DB: $DB (run: orderflow fetch, or use database/orderflow.db)" >&2
  exit 1
fi

"$BIN" export-web --db "$DB" --out-dir "$ROOT/web/data"

INTEGRATOR_OUT="$ROOT/web/data/integrator_recipients.json"
if [[ -f "$ROOT/results.csv" && -f "$ROOT/integrator_fee_recipients_mini.csv" ]]; then
  echo "Rebuilding integrator_recipients.json (full corpus)…"
  python3 "$ROOT/scripts/build_integrator_recipient_sankey.py" \
    --txs "$ROOT/results.csv" \
    --fees "$ROOT/integrator_fee_recipients_mini.csv" \
    --out "$INTEGRATOR_OUT"
elif [[ -f "$ROOT/samples/1inch_Integrators.csv" && -f "$ROOT/samples/integrator_fee_recipients.csv" ]]; then
  echo "Rebuilding integrator_recipients.json (samples/)…"
  python3 "$ROOT/scripts/build_integrator_recipient_sankey.py" \
    --txs "$ROOT/samples/1inch_Integrators.csv" \
    --fees "$ROOT/samples/integrator_fee_recipients.csv" \
    --out "$INTEGRATOR_OUT"
elif [[ ! -f "$INTEGRATOR_OUT" ]]; then
  echo "Warning: integrator_recipients.json missing; integrators page will be empty." >&2
fi

echo "Done. Publish directory: $ROOT/web"
ls -lh "$ROOT/web/data/"*.json 2>/dev/null || true
