-- Q2 · Liquidity view (per hop / leg; may overcount multihops vs orderflow) — same stack as
-- [orderflow.barterswap.xyz](https://orderflow.barterswap.xyz/).
--
-- **Flashbots 1inch Fusion liquidity view:**
--   - Dune query: https://dune.com/queries/3179125
--   - Materialized: dune.flashbots.result_1inch_fusion_liquidity_view
-- All-time variant: Q3179123 → dune.flashbots.result_alltime_1inch_fusion_liquidity_view
--
-- Use `DUNE_QUERY_LIQUIDITY=3179125` or `DUNE_USE_FLASHBOTS_DEFAULTS=1`.
/*
SELECT *
FROM dune.flashbots.result_1inch_fusion_liquidity_view
WHERE block_time >= now() - interval '7' day
*/

-- `src/snapshot.rs` aggregates: liquidity_src + amount_usd (fallback: project).

SELECT CAST(NULL AS VARCHAR) AS hash, CAST(NULL AS VARCHAR) AS liquidity_src, CAST(NULL AS DOUBLE) AS amount_usd
WHERE 1 = 0;
