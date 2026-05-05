-- Q1 · Orderflow view (one row per user trade, deduped notional) — align with
-- [orderflow.barterswap.xyz](https://orderflow.barterswap.xyz/) / [methodology](https://orderflow.art/methodology).
--
-- **Use Flashbots’ public 1inch Fusion orderflow (same as their Orderflow view):**
--   - Dune query: https://dune.com/queries/3184593
--   - Materialized table: dune.flashbots.result_1inch_fusion_orderflow_view
-- In the Dune app, “Run” that query (or save a copy). Point `DUNE_QUERY_ORDERFLOW=3184593`
-- at your fork, or set `DUNE_USE_FLASHBOTS_DEFAULTS=1` to hit query 3184593 via the API.
--
-- If you author SQL here instead, you can read the materialized result directly:
/*
SELECT *
FROM dune.flashbots.result_1inch_fusion_orderflow_view
WHERE block_time >= now() - interval '7' day
*/

-- Parser in `src/model/orderflow.rs` expects: hash, frontend, trade_usd (and optional user, trade_pair, …)

SELECT CAST(NULL AS VARCHAR) AS hash, CAST(NULL AS VARCHAR) AS frontend, CAST(NULL AS DOUBLE) AS trade_usd
WHERE 1 = 0;
