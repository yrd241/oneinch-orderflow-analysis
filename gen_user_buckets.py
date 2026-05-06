#!/usr/bin/env python3
"""
Generate web/user_buckets.json: maps display bucket name → sorted list of user addresses.
Bucket names match what cleanName() in eoa.js produces (EOA stripped).
Run this whenever the DB is refreshed.
"""

import sqlite3, json, os, sys

DB   = os.path.expanduser("~/.cache/oneinch-orderflow/orderflow.db")
OUT  = os.path.join(os.path.dirname(__file__), "web", "user_buckets.json")

def main():
    if not os.path.exists(DB):
        sys.exit(f"DB not found: {DB}")

    conn = sqlite3.connect(DB)
    cur  = conn.cursor()

    # All active users in 1inch Router orderflow (tx-hash deduped)
    cur.execute("""
        SELECT DISTINCT LOWER(json_extract(payload, '$.user'))
        FROM raw_rows
        WHERE kind = 'orderflow_view'
          AND json_extract(payload, '$.hash') IS NOT NULL
          AND (LOWER(json_extract(payload, '$.solver')) LIKE '%1inch%'
               OR LOWER(json_extract(payload, '$.solver')) = 'unspecified')
          AND json_extract(payload, '$.user') IS NOT NULL
          AND json_extract(payload, '$.user') != ''
    """)
    active_users = {r[0] for r in cur.fetchall() if r[0]}

    # user_7702_map: user → delegated_to
    cur.execute("SELECT LOWER(user), LOWER(delegated_to) FROM user_7702_map")
    user_to_del = {r[0]: r[1] for r in cur.fetchall()}

    # delegated_7702_labels: delegated_to → label
    cur.execute("SELECT LOWER(delegated_to), label FROM delegated_7702_labels")
    del_to_label = {r[0]: r[1] for r in cur.fetchall()}

    conn.close()

    buckets: dict[str, list[str]] = {}

    for addr in active_users:
        if addr in user_to_del:
            delegated = user_to_del[addr]
            label = del_to_label.get(delegated)
            # "User: EOA (7702 tokenPocket)" → cleanName → "User (7702 tokenPocket)"
            bucket = f"User (7702 {label})" if label else "User (7702)"
        else:
            bucket = "User (Unlabeled)"

        buckets.setdefault(bucket, []).append(addr)

    for addrs in buckets.values():
        addrs.sort()

    with open(OUT, "w") as f:
        json.dump(buckets, f, separators=(",", ":"))

    total = sum(len(v) for v in buckets.values())
    for name, addrs in sorted(buckets.items()):
        print(f"  {name}: {len(addrs)}")
    print(f"Total: {total}  →  {OUT}")

if __name__ == "__main__":
    main()
