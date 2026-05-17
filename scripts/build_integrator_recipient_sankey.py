#!/usr/bin/env python3
"""
Build web payload: Recipient → Frontend: 1inch Integrators (Sankey + table).

Inputs (from your Dune + RPC pipeline):
  - Tx list CSV (hash, trade_usd, block_time) — e.g. Q11 download
  - Fee decode CSV — decode_fee_recipient.py output

Usage:
  python3 scripts/build_integrator_recipient_sankey.py \\
    --txs data/1inch_Integrators.csv \\
    --fees data/integrator_fee_recipients.csv \\
    --out web/data/integrator_recipients.json
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from collections import defaultdict
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from aggregate_integrator_recipients import all_fee_recipients, primary_max_fee  # noqa: E402

FRONTEND_NODE = "Frontend: 1inch Integrators"
RECIPIENT_PREFIX = "Recipient: "
OTHER_LABEL = f"{RECIPIENT_PREFIX}…other"
UNK_LABEL = f"{RECIPIENT_PREFIX}(no fee in calldata)"
DEFAULT_TOP_N = 10

# Known integrator feeRecipient wallets (lowercase keys).
KNOWN_FEE_RECIPIENT_LABELS: dict[str, str] = {
    "0x8d413db42d6901de42b2c481cc0f6d0fd1c52828": "Coinbase Wallet",
    "0x39041f1b366fe33f9a5a79de5120f2aee2577ebc": "Rabby Wallet",
    "0x4a183b7ed67b9e14b3f45abfb2cf44ed22c29e54": "Zerion",
}


def _norm_hash(h: str) -> str:
    h = (h or "").strip().lower()
    return h if h.startswith("0x") else "0x" + h


def _norm_addr(addr: str) -> str:
    a = (addr or "").strip().lower()
    if not a:
        return ""
    return a if a.startswith("0x") else "0x" + a


def _wallet_name(addr: str) -> str:
    return KNOWN_FEE_RECIPIENT_LABELS.get(_norm_addr(addr), "")


def _recipient_label(addr: str) -> str:
    name = _wallet_name(addr)
    if name:
        return f"{RECIPIENT_PREFIX}{name}"
    a = _norm_addr(addr)
    if len(a) >= 14:
        short = a[:6] + "…" + a[-4:]
    else:
        short = a
    return f"{RECIPIENT_PREFIX}{short}"


def _parse_time(s: str) -> datetime | None:
    if not s or not str(s).strip():
        return None
    raw = str(s).strip().replace(" UTC", "").split(".")[0]
    for fmt in ("%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"):
        try:
            return datetime.strptime(raw, fmt)
        except ValueError:
            continue
    return None


def load_txs(path: Path) -> dict[str, dict]:
    by_hash: dict[str, dict] = {}
    with path.open(newline="") as f:
        for row in csv.DictReader(f):
            h = _norm_hash(row.get("hash") or "")
            if not h:
                continue
            try:
                usd = float(row.get("trade_usd") or 0)
            except ValueError:
                usd = 0.0
            by_hash[h] = {
                "trade_usd": usd,
                "block_time": row.get("block_time") or "",
            }
    return by_hash


def load_fees(path: Path) -> dict[str, dict]:
    by_hash: dict[str, dict] = {}
    with path.open(newline="") as f:
        for row in csv.DictReader(f):
            h = _norm_hash(row.get("hash") or "")
            if not h or row.get("error"):
                continue
            recipients = all_fee_recipients(row)
            if not recipients:
                continue
            primary = primary_max_fee(row) or recipients[0]
            by_hash[h] = {
                "primary": primary,
                "recipients": recipients,
                "fee_recipients": row.get("fee_recipients") or row.get("fee_recipient") or "",
            }
    return by_hash


def _limit_recipients_for_ui(
    agg: dict[str, dict], *, top_n: int, rollup_other: bool
) -> dict[str, dict]:
    """Keep top N parsed recipients by trade_usd.

    rollup_other=False (default): strict top N only — no …other bucket.
    rollup_other=True: merge rank > N into Recipient: …other.
    """
    if top_n <= 0 or len(agg) <= top_n:
        return dict(agg)
    ranked = sorted(agg.items(), key=lambda x: (-x[1]["trade_usd"], -x[1]["tx_count"], x[0]))
    shown = dict(ranked[:top_n])
    if not rollup_other:
        return shown
    rest = ranked[top_n:]
    if not rest:
        return shown
    other: dict = {"addr": "", "tx_count": 0, "trade_usd": 0.0, "hashes": []}
    for _label, rec in rest:
        other["tx_count"] += rec["tx_count"]
        other["trade_usd"] += rec["trade_usd"]
        for h in rec["hashes"]:
            if h not in other["hashes"]:
                other["hashes"].append(h)
    shown[OTHER_LABEL] = other
    return shown


def _ui_recipient_order(agg: dict[str, dict]) -> list[tuple[str, dict]]:
    """Top recipients by volume, then …other last (never sorted into the middle)."""
    other_rec = agg.get(OTHER_LABEL)
    main = sorted(
        ((label, rec) for label, rec in agg.items() if label != OTHER_LABEL),
        key=lambda x: (-x[1]["trade_usd"], -x[1]["tx_count"], x[0]),
    )
    if other_rec is not None:
        main.append((OTHER_LABEL, other_rec))
    return main


def build(
    txs: dict[str, dict],
    fees: dict[str, dict],
    *,
    top_n: int = DEFAULT_TOP_N,
    rollup_other: bool = False,
) -> dict:
    agg: dict[str, dict] = defaultdict(
        lambda: {"tx_count": 0, "trade_usd": 0.0, "hashes": [], "addr": ""}
    )
    unknown = {"tx_count": 0, "trade_usd": 0.0, "hashes": []}
    times: list[datetime] = []

    for h, tx in txs.items():
        bt = _parse_time(tx.get("block_time", ""))
        if bt:
            times.append(bt)
        usd = tx["trade_usd"]
        fee = fees.get(h)
        if not fee:
            unknown["tx_count"] += 1
            unknown["trade_usd"] += usd
            unknown["hashes"].append(h)
            continue
        for addr in fee["recipients"]:
            key = _recipient_label(addr)
            rec = agg[key]
            rec["addr"] = addr
            rec["tx_count"] += 1
            rec["trade_usd"] += usd
            if h not in rec["hashes"]:
                rec["hashes"].append(h)

    total_parsed_recipients = len(agg)
    agg = _limit_recipients_for_ui(agg, top_n=top_n, rollup_other=rollup_other)

    links = []
    nodes_map: dict[str, int] = {}

    def add_node(name: str, depth: int) -> None:
        if name not in nodes_map:
            nodes_map[name] = depth

    for label, rec in _ui_recipient_order(agg):
        add_node(label, 0)
        add_node(FRONTEND_NODE, 1)
        links.append(
            {
                "source": label,
                "target": FRONTEND_NODE,
                "value": float(rec["tx_count"]),
                "volume_usd": rec["trade_usd"],
            }
        )

    if unknown["tx_count"]:
        add_node(UNK_LABEL, 0)
        add_node(FRONTEND_NODE, 1)
        links.append(
            {
                "source": UNK_LABEL,
                "target": FRONTEND_NODE,
                "value": float(unknown["tx_count"]),
                "volume_usd": unknown["trade_usd"],
            }
        )
        agg[UNK_LABEL] = {
            "addr": "",
            "tx_count": unknown["tx_count"],
            "trade_usd": unknown["trade_usd"],
            "hashes": unknown["hashes"],
        }

    nodes = [{"name": n, "depth": d} for n, d in sorted(nodes_map.items(), key=lambda x: (x[1], x[0]))]

    block_time_range = None
    if times:
        block_time_range = [
            min(times).strftime("%Y-%m-%d %H:%M:%S") + " UTC",
            max(times).strftime("%Y-%m-%d %H:%M:%S") + " UTC",
        ]

    recipients_detail = {}
    for label, rec in _ui_recipient_order(agg):
        addr = rec.get("addr") or ""
        wallet = _wallet_name(addr)
        recipients_detail[label] = {
            "address": addr,
            "wallet": wallet,
            "tx_count": rec["tx_count"],
            "trade_usd": round(rec["trade_usd"], 4),
            "hashes": rec["hashes"],
        }

    return {
        "source": "integrator_recipients",
        "block_time_range": block_time_range,
        "sankey": {"nodes": nodes, "links": links},
        "recipients_detail": recipients_detail,
        "meta": {
            "tx_rows": len(txs),
            "tx_with_fee": sum(1 for h in txs if h in fees),
            "unique_recipients_total": total_parsed_recipients,
            "unique_recipients_shown": len(
                [k for k in agg if k not in (UNK_LABEL, OTHER_LABEL)]
            )
            + (1 if OTHER_LABEL in agg else 0),
            "top_n": top_n,
            "rollup_other": rollup_other,
            "count_all_recipients": True,
            "primary_by_max_fee": True,
        },
    }


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--txs", type=Path, required=True, help="Dune Q11 / fetch CSV (hash, trade_usd)")
    p.add_argument("--fees", type=Path, required=True, help="decode_fee_recipient.py output")
    p.add_argument(
        "--out",
        type=Path,
        default=Path("web/data/integrator_recipients.json"),
    )
    p.add_argument(
        "--top",
        type=int,
        default=DEFAULT_TOP_N,
        help="max parsed fee recipients in UI (by trade_usd)",
    )
    mode = p.add_mutually_exclusive_group()
    mode.add_argument(
        "--no-other",
        dest="rollup_other",
        action="store_false",
        help="strict top N only, drop the rest (default)",
    )
    mode.add_argument(
        "--rollup-other",
        dest="rollup_other",
        action="store_true",
        help="merge recipients below top N into Recipient: …other",
    )
    p.set_defaults(rollup_other=False)
    args = p.parse_args()
    rollup_other = args.rollup_other
    if not args.txs.is_file():
        print(f"Missing {args.txs}", file=sys.stderr)
        return 1
    if not args.fees.is_file():
        print(f"Missing {args.fees}", file=sys.stderr)
        return 1

    payload = build(
        load_txs(args.txs),
        load_fees(args.fees),
        top_n=max(1, args.top),
        rollup_other=rollup_other,
    )
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(f"Wrote {args.out}", file=sys.stderr)
    print(f"Meta: {payload['meta']}", file=sys.stderr)
    m = payload["meta"]
    print(
        f"Recipients: {m.get('unique_recipients_shown', '?')} shown "
        f"({m.get('unique_recipients_total', '?')} parsed, top {m.get('top_n', '?')})",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
