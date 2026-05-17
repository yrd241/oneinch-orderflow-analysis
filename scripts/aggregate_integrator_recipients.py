#!/usr/bin/env python3
"""
Aggregate 1inch Integrators volume by fee recipient (integrator wallet).

Pipeline:
  1. fetch_sample_txs_by_frontend.py --full  →  all hashes + trade_usd
  2. decode_fee_recipient.py                 →  fee_recipient(s) + on-chain fee amounts
  3. This script                             →  rank recipients by volume

Primary recipient (max fee per tx — for labeling / primary_tx_count only):
  - max_fee (default): largest on-chain fee (Transfer sum; fallback calldata amount8).
  - corpus_dominant / tag_leg / first: legacy primary pickers.

Volume / tx_count stats: every address in fee_recipients is counted (multi-fee txs
contribute to each leg). Total trade_usd across recipients can exceed chain total.

Usage:
  python3 scripts/fetch_sample_txs_by_frontend.py --frontend "1inch Integrators" --full

  python3 scripts/decode_fee_recipient.py \\
    --csv data/1inch_Integrators.csv --out data/integrator_fee_recipients.csv

  python3 scripts/aggregate_integrator_recipients.py \\
    --fees data/integrator_fee_recipients.csv
"""

from __future__ import annotations

import argparse
import csv
import sys
from collections import Counter, defaultdict
from pathlib import Path


def _split_addrs(s: str) -> list[str]:
    if not s or not str(s).strip():
        return []
    return [a.strip().lower() for a in str(s).split(";") if a.strip()]


def _split_amounts(s: str) -> list[int]:
    if not s or not str(s).strip():
        return []
    out: list[int] = []
    for part in str(s).split(";"):
        part = part.strip()
        if not part:
            continue
        try:
            out.append(int(part))
        except ValueError:
            out.append(0)
    return out


def all_fee_recipients(row: dict) -> list[str]:
    """Every integrator fee wallet in this tx (deduped, lowercased)."""
    fees = _split_addrs(row.get("fee_recipients") or "")
    if not fees:
        one = (row.get("fee_recipient") or row.get("primary_fee_recipient") or "").strip().lower()
        return [one] if one else []
    seen: set[str] = set()
    out: list[str] = []
    for f in fees:
        if f not in seen:
            seen.add(f)
            out.append(f)
    return out


def primary_max_fee(row: dict) -> str | None:
    fees = all_fee_recipients(row)
    if not fees:
        return None
    if len(fees) == 1:
        return fees[0]

    methods = [m.strip() for m in (row.get("decode_method") or "").split(";") if m.strip()]
    amts = _split_amounts(row.get("fee_amounts_raw") or "")
    if len(amts) < len(fees):
        amts = amts + [0] * (len(fees) - len(amts))

    # Real on-chain fee in CSV (post-receipt decode): pick largest raw amount
    if any(a > 1_000_000 for a in amts):
        best_i = max(range(len(fees)), key=lambda i: (amts[i], -i))
        return fees[best_i]

    # Multi-fee 25b0d0 + amount8: amount8 field is usually NOT token wei (e.g. 22 vs 4e17)
    if "amount8" in (row.get("decode_method") or ""):
        for i, addr in enumerate(fees):
            m = methods[i] if i < len(methods) else ""
            if "amount8" not in m and "token+fee" in m:
                return addr

    primary = (row.get("primary_fee_recipient") or row.get("fee_recipient") or "").strip().lower()
    if primary and primary in fees:
        return primary

    best_i = max(range(len(fees)), key=lambda i: (amts[i], -i))
    return fees[best_i]


def primary_tag_leg(fees: list[str], methods: str) -> str | None:
    if not fees:
        return None
    if len(fees) == 1:
        return fees[0]
    mlist = [m.strip() for m in (methods or "").split(";") if m.strip()]
    for i, addr in enumerate(fees):
        m = mlist[i] if i < len(mlist) else ""
        if "amount8" not in m:
            return addr
    return fees[0]


def primary_first(fees: list[str], _methods: str) -> str | None:
    return fees[0] if fees else None


def build_corpus_dominant(single_fee_counts: Counter[str]) -> callable:
    def pick(fees: list[str], _methods: str) -> str | None:
        if not fees:
            return None
        if len(fees) == 1:
            return fees[0]
        return max(fees, key=lambda f: (single_fee_counts.get(f, 0), -fees.index(f)))

    return pick


def load_fee_rows(path: Path) -> list[dict]:
    with path.open(newline="") as f:
        return list(csv.DictReader(f))


def aggregate(
    rows: list[dict],
    *,
    primary_mode: str,
    min_trade_usd: float,
) -> tuple[list[dict], dict]:
    if primary_mode == "max_fee":

        def pick_row(r: dict) -> str | None:
            return primary_max_fee(r)

    else:
        usable = [
            r
            for r in rows
            if not r.get("error")
            and _split_addrs(r.get("fee_recipients") or r.get("fee_recipient") or "")
        ]
        single_counts: Counter[str] = Counter()
        for r in usable:
            fees = _split_addrs(r.get("fee_recipients") or r.get("fee_recipient") or "")
            if len(fees) == 1:
                single_counts[fees[0]] += 1

        if primary_mode == "corpus_dominant":
            pick_fees = build_corpus_dominant(single_counts)
        elif primary_mode == "tag_leg":
            pick_fees = primary_tag_leg
        elif primary_mode == "first":
            pick_fees = primary_first
        else:
            raise ValueError(f"unknown primary_mode: {primary_mode}")

        def pick_row(r: dict) -> str | None:
            fees = _split_addrs(r.get("fee_recipients") or r.get("fee_recipient") or "")
            return pick_fees(fees, r.get("decode_method") or "")

    by_recipient: dict[str, dict] = defaultdict(
        lambda: {
            "tx_count": 0,
            "primary_tx_count": 0,
            "trade_usd": 0.0,
            "multi_fee_tx_count": 0,
        }
    )
    skipped_no_fee = 0
    skipped_small = 0
    multi_fee = 0
    chain_txs_attributed = 0

    for r in rows:
        if r.get("error"):
            continue
        try:
            usd = float(r.get("trade_usd") or 0)
        except ValueError:
            usd = 0.0
        if min_trade_usd and usd < min_trade_usd:
            skipped_small += 1
            continue

        fees = all_fee_recipients(r)
        if not fees:
            skipped_no_fee += 1
            continue

        chain_txs_attributed += 1
        is_multi = len(fees) > 1
        if is_multi:
            multi_fee += 1

        primary = pick_row(r)
        for addr in fees:
            rec = by_recipient[addr]
            rec["tx_count"] += 1
            rec["trade_usd"] += usd
            if is_multi:
                rec["multi_fee_tx_count"] += 1
            if addr == primary:
                rec["primary_tx_count"] += 1

    ranked = []
    for addr, m in by_recipient.items():
        ranked.append(
            {
                "fee_recipient": addr,
                "tx_count": m["tx_count"],
                "primary_tx_count": m["primary_tx_count"],
                "trade_usd": round(m["trade_usd"], 4),
                "volume_m_usd": round(m["trade_usd"] / 1e6, 4),
                "multi_fee_tx_count": m["multi_fee_tx_count"],
            }
        )
    ranked.sort(key=lambda x: (-x["trade_usd"], -x["tx_count"], x["fee_recipient"]))

    meta = {
        "input_rows": len(rows),
        "chain_txs_with_fee": chain_txs_attributed,
        "recipient_attribution_slots": sum(x["tx_count"] for x in ranked),
        "unique_recipients": len(ranked),
        "multi_fee_txs": multi_fee,
        "skipped_no_fee": skipped_no_fee,
        "skipped_below_min_usd": skipped_small,
        "primary_mode": primary_mode,
        "count_all_recipients": True,
    }
    return ranked, meta


def main() -> int:
    p = argparse.ArgumentParser(description="Rank 1inch Integrator fee recipients by volume")
    p.add_argument(
        "--fees",
        type=Path,
        default=Path("samples/integrator_fee_recipients.csv"),
        help="CSV from decode_fee_recipient.py",
    )
    p.add_argument(
        "--out",
        type=Path,
        default=Path("samples/integrator_recipients_ranked.csv"),
    )
    p.add_argument(
        "--primary",
        choices=("max_fee", "corpus_dominant", "tag_leg", "first"),
        default="max_fee",
        help="how to pick primary when a tx has multiple (all legs still counted)",
    )
    p.add_argument("--min-trade-usd", type=float, default=0.0)
    p.add_argument("--top", type=int, default=20, help="print top N to stderr")
    args = p.parse_args()

    if not args.fees.is_file():
        print(f"Missing {args.fees}; run decode_fee_recipient.py first.", file=sys.stderr)
        return 1

    rows = load_fee_rows(args.fees)
    ranked, meta = aggregate(rows, primary_mode=args.primary, min_trade_usd=args.min_trade_usd)

    args.out.parent.mkdir(parents=True, exist_ok=True)
    fields = [
        "rank",
        "fee_recipient",
        "tx_count",
        "primary_tx_count",
        "trade_usd",
        "volume_m_usd",
        "multi_fee_tx_count",
    ]
    with args.out.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        for i, row in enumerate(ranked, start=1):
            w.writerow({"rank": i, **row})

    print(f"Wrote {len(ranked)} recipients -> {args.out}", file=sys.stderr)
    print(f"Meta: {meta}", file=sys.stderr)
    print(f"\nTop {args.top} by trade_usd (primary={args.primary}):", file=sys.stderr)
    for i, row in enumerate(ranked[: args.top], 1):
        print(
            f"  #{i} {row['fee_recipient']}  "
            f"tx={row['tx_count']}  ${row['trade_usd']:,.2f}  "
            f"({row['volume_m_usd']} M)",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
