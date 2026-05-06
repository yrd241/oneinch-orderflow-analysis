-- Q7 · 1inch Router solver orderflow Sankey edges
-- Filters on solver LIKE '%1inch%' to capture 1inch Router + 1inch Labs variants.
-- Flow (trade_usd-based, deduped):
--   User class → Frontend → 1inch Router (solver) → Mempool → OFA → Builder
-- Returns (edge_level, source, target, tx_count, volume_m_usd) tuples.
-- edge_level format: "L1>L2", "L2>L3", "L3>L4", "L4>L5", "L5>L6"
-- One extra row with edge_level = 'META' carries the block_time range in source/target.
-- Used by snapshot::edges_to_payload() to build the web Sankey.
--
-- Dune query ID: 7428851

-- Deduplicate multihop transactions: keep one row per tx hash (earliest block_time).
-- This matches the orderflow.art methodology to avoid double-counting multihop volume.
WITH deduped AS (
  SELECT hash, frontend, solver, trade_usd, mempool, ofa, builder, "user" AS user_addr, block_time
  FROM (
    SELECT
      hash, frontend, solver, trade_usd, mempool, ofa, builder, "user", block_time,
      ROW_NUMBER() OVER (PARTITION BY hash ORDER BY block_time) AS rn
    FROM dune.flashbots.result_overall_of
    WHERE solver LIKE '%1inch%'
  ) t
  WHERE rn = 1
),
-- --- User classification helpers ---
--
-- 1) Address labels (manual, maintained in your fork).
--    We start with a VALUES-based table so the query runs everywhere.
--    If you have a working labels dataset, replace this CTE with a SELECT from that dataset.
user_labels AS (
  SELECT *
  FROM (
    VALUES
      -- ('0x...', 'Some Label', 'Category')
      (CAST(NULL AS VARCHAR), CAST(NULL AS VARCHAR), CAST(NULL AS VARCHAR))
  ) AS t(address, label_name, label_category)
  WHERE address IS NOT NULL
),
-- 2) Smart wallets (optional, also manual by default).
--    If you have a reliable Safe table in your environment, replace this with that table.
safe_wallets AS (
  SELECT *
  FROM (
    VALUES
      -- ('0x...') -- Safe address
      (CAST(NULL AS VARCHAR))
  ) AS t(address)
  WHERE address IS NOT NULL
),
-- 3) Other contract accounts (optional, manual by default).
contract_accounts AS (
  SELECT *
  FROM (
    VALUES
      -- ('0x...') -- contract account address
      (CAST(NULL AS VARCHAR))
  ) AS t(address)
  WHERE address IS NOT NULL
),
-- 4) EIP-7702: delegated implementation addresses -> human label (maintain in your fork).
--    Sync with local lists (e.g. delegated_7702_labels / 7702_delegated_unique.txt workflows).
eip7702_delegated_labels AS (
  SELECT *
  FROM (
    VALUES
      -- ('0xDELEGATED_IMPLEMENTATION', 'metamask')
      (CAST(NULL AS VARCHAR), CAST(NULL AS VARCHAR))
  ) AS t(delegated_to, label)
  WHERE delegated_to IS NOT NULL
),
--    Map sending EOA (user) -> delegated_to code address from eth_getCode / your pipeline.
eip7702_user_map AS (
  SELECT *
  FROM (
    VALUES
      -- ('0xUSER_EOA', '0xDELEGATED_IMPLEMENTATION')
      (CAST(NULL AS VARCHAR), CAST(NULL AS VARCHAR))
  ) AS t(user_addr, delegated_to)
  WHERE user_addr IS NOT NULL
),
-- Build a per-tx user class label for the first Sankey layer.
user_classified AS (
  SELECT
    d.*,
    CASE
      WHEN ul.address IS NOT NULL THEN
        'User: Labeled (' || COALESCE(ul.label_category, 'Uncategorized') || ') ' || COALESCE(ul.label_name, 'Unknown')
      WHEN sw.address IS NOT NULL THEN 'User: Smart Wallet (Safe)'
      WHEN ca.address IS NOT NULL THEN 'User: Smart Wallet (Other)'
      WHEN e7u.user_addr IS NOT NULL THEN
        CASE
          WHEN e7l.label IS NOT NULL AND TRIM(e7l.label) <> '' THEN
            'User: EOA (7702 ' || e7l.label || ')'
          ELSE 'User: EOA (7702)'
        END
      ELSE 'User: EOA (Unlabeled)'
    END AS user_class
  FROM deduped d
  LEFT JOIN user_labels ul
    ON LOWER(ul.address) = LOWER(CAST(d.user_addr AS VARCHAR))
  LEFT JOIN safe_wallets sw
    ON LOWER(sw.address) = LOWER(CAST(d.user_addr AS VARCHAR))
  LEFT JOIN contract_accounts ca
    ON LOWER(ca.address) = LOWER(CAST(d.user_addr AS VARCHAR))
  LEFT JOIN eip7702_user_map e7u
    ON LOWER(e7u.user_addr) = LOWER(CAST(d.user_addr AS VARCHAR))
  LEFT JOIN eip7702_delegated_labels e7l
    ON LOWER(e7l.delegated_to) = LOWER(e7u.delegated_to)
),
base AS (
  SELECT
    user_class                                             AS l1,
    'Frontend: ' || CASE
      WHEN frontend IN ('1inch Integrators','1inch Website: Default','Trust Wallet',
                        'MetaMask Swaps','Binance Wallet','deBridge Frontend',
                        'Li.Fi Integrators','Rainbow Wallet','Cowswap Integrators',
                        'Fluid Frontend')
      THEN frontend
      ELSE 'Other Frontends'
    END                                                    AS l2,
    'Solver: 1inch Router'                                 AS l3,
    CASE mempool
      WHEN 'private mempool' THEN 'Mempool: Private'
      WHEN 'public mempool'  THEN 'Mempool: Public'
      ELSE 'Mempool: Unknown'
    END                                                    AS l4,
    'OFA: ' || COALESCE(ofa, 'None')                      AS l5,
    'Builder: ' || CASE
      WHEN builder IN ('Titan','BuilderNet','beaverbuild','Quasar','rsync-builder','BTCS')
      THEN builder
      ELSE 'Others'
    END                                                    AS l6,
    trade_usd
  FROM user_classified
),
edges_l1_l2 AS (
  SELECT l1 AS source, l2 AS target,
         COUNT(*) AS tx_count, ROUND(SUM(trade_usd)/1e6,4) AS volume_m_usd
  FROM base GROUP BY 1,2
),
edges_l2_l3 AS (
  SELECT l2 AS source, l3 AS target,
         COUNT(*) AS tx_count, ROUND(SUM(trade_usd)/1e6,4) AS volume_m_usd
  FROM base GROUP BY 1,2
),
edges_l3_l4 AS (
  SELECT l3 AS source, l4 AS target,
         COUNT(*) AS tx_count, ROUND(SUM(trade_usd)/1e6,4) AS volume_m_usd
  FROM base GROUP BY 1,2
),
edges_l4_l5 AS (
  SELECT l4 AS source, l5 AS target,
         COUNT(*) AS tx_count, ROUND(SUM(trade_usd)/1e6,4) AS volume_m_usd
  FROM base GROUP BY 1,2
),
edges_l5_l6 AS (
  SELECT l5 AS source, l6 AS target,
         COUNT(*) AS tx_count, ROUND(SUM(trade_usd)/1e6,4) AS volume_m_usd
  FROM base GROUP BY 1,2
)
SELECT 'L1>L2' AS edge_level, source, target, tx_count, volume_m_usd FROM edges_l1_l2
UNION ALL
SELECT 'L2>L3', source, target, tx_count, volume_m_usd FROM edges_l2_l3
UNION ALL
SELECT 'L3>L4', source, target, tx_count, volume_m_usd FROM edges_l3_l4
UNION ALL
SELECT 'L4>L5', source, target, tx_count, volume_m_usd FROM edges_l4_l5
UNION ALL
SELECT 'L5>L6', source, target, tx_count, volume_m_usd FROM edges_l5_l6
UNION ALL
-- META row: block_time range of all included transactions (UTC).
-- source = MIN(block_time), target = MAX(block_time), both as VARCHAR.
SELECT 'META'                              AS edge_level,
       CAST(MIN(block_time) AS VARCHAR)    AS source,
       CAST(MAX(block_time) AS VARCHAR)    AS target,
       COUNT(*)                            AS tx_count,
       ROUND(SUM(trade_usd)/1e6, 4)        AS volume_m_usd
FROM deduped
ORDER BY edge_level, tx_count DESC
