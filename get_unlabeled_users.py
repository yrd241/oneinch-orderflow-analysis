#!/usr/bin/env python3
"""
Export all unique Unlabeled user addresses from the orderflow database.
Unlabeled = appeared in orderflow_view (1inch Router txs) but NOT in user_7702_map.

Output: unlabeled_users.txt (one address per line)
"""

import sqlite3
import os
import sys

DB_PATH = os.path.expanduser("~/.cache/oneinch-orderflow/orderflow.db")
OUT_FILE = os.path.join(os.path.dirname(__file__), "unlabeled_users.txt")

def main():
    if not os.path.exists(DB_PATH):
        print(f"DB not found: {DB_PATH}", file=sys.stderr)
        sys.exit(1)

    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()

    cur.execute("""
        SELECT DISTINCT LOWER(json_extract(payload, '$.user')) AS user
        FROM raw_rows
        WHERE kind = 'orderflow_view'
          AND json_extract(payload, '$.hash') IS NOT NULL
          AND (
            LOWER(json_extract(payload, '$.solver')) LIKE '%1inch%'
            OR LOWER(json_extract(payload, '$.solver')) = 'unspecified'
          )
          AND json_extract(payload, '$.user') IS NOT NULL
          AND json_extract(payload, '$.user') != ''
          AND LOWER(json_extract(payload, '$.user')) NOT IN (
            SELECT LOWER(user) FROM user_7702_map
          )
        ORDER BY user
    """)

    rows = cur.fetchall()
    conn.close()

    addrs = [r[0] for r in rows if r[0]]
    print(f"Found {len(addrs)} unlabeled user addresses")

    with open(OUT_FILE, "w") as f:
        f.write("\n".join(addrs) + "\n")

    print(f"Written to {OUT_FILE}")

if __name__ == "__main__":
    main()
