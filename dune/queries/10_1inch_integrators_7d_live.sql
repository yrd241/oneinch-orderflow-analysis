-- Q10 · 1inch Integrators — last 7 days (live pipeline, no stale overall_of MV)
--
-- Mirrors Orderflow.art labeling:
--   - Volume / pair: deduped fills (same logic as Flashbots query 3100100)
--   - Frontend: Router Labels (query 3004150) where frontend = '1inch Integrators'
--   - Does NOT include 1inch Fusion (separate frontend in their stack)
--
-- Prerequisites on Dune (public Flashbots team queries — readable when logged in):
--   query_3100100  [orderflow_version] dex dedup table
--   query_3004150  Router Labels
--
-- Cost: dominated by scanning query_3100100 (~100–200 credits/run). For repeated
-- analysis, save this query → Materialize → refresh daily on your own team.
--
-- Output: one row per tx (deduped by hash), newest first.

WITH deduped AS (
  SELECT
    t.blockchain,
    t.tx_hash,
    t.tx_from,
    t.tx_to,
    t.block_time,
    t.token_pair,
    t.amount_usd AS trade_usd,
    ROW_NUMBER() OVER (PARTITION BY t.tx_hash ORDER BY t.block_time) AS rn
  FROM query_3100100 t
  WHERE t.blockchain = 'ethereum'
    AND t.block_time > now() - interval '7' day
),
integrator_txs AS (
  SELECT
    d.blockchain,
    d.tx_hash,
    d.tx_from,
    d.tx_to,
    d.block_time,
    d.token_pair,
    d.trade_usd,
    r.router        AS router_label,
    r.contract_name AS router_contract
  FROM deduped d
  INNER JOIN query_3004150 r
    ON d.tx_to = r.address
   AND r.blockchain = 'ethereum'
   AND r.frontend = '1inch Integrators'
  WHERE d.rn = 1
),
agg AS (
  SELECT
    tx_hash,
    MAX(project)  AS agg_project,
    MAX(version)  AS agg_version
  FROM dex_aggregator.trades
  WHERE blockchain = 'ethereum'
    AND block_time > now() - interval '7' day
    AND project LIKE '%1inch%'
    AND project NOT IN ('1inch Limit Order Protocol')  -- liquidity leg; optional tighten
  GROUP BY 1
)
SELECT
  CAST(t.tx_hash AS VARCHAR)     AS hash,
  t.block_time,
  CAST(t.tx_from AS VARCHAR)     AS user_addr,
  CAST(t.tx_to AS VARCHAR)       AS router,
  r.router                       AS router_label,
  r.contract_name                AS router_contract,
  t.token_pair,
  t.trade_usd,
  a.agg_project,
  a.agg_version
FROM integrator_txs t
LEFT JOIN agg a ON a.tx_hash = t.tx_hash
ORDER BY t.block_time DESC
