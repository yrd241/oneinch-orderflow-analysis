-- Reference only — no rows. Documents Flashbots / barterswap orderflow data layer on Dune.
-- Methodology: https://orderflow.art/methodology
--
-- === 1inch Fusion (matches barterswap “1inch” slices) ===
-- | Materialized view | Query ID (public) |
-- |---|---|
-- | dune.flashbots.result_1inch_fusion_txs_7d | 3079565 |
-- | dune.flashbots.result_1inch_fusion_txs_alltime | 3178977 |
-- | dune.flashbots.result_1inch_fusion_orderflow_view | 3184593 |
-- | dune.flashbots.result_1inch_fusion_liquidity_view | 3179125 |
-- | dune.flashbots.result_alltime_1inch_fusion_liquidity_view | 3179123 |
--
-- === Overall Ethereum Sankey (full Orderflow.art-style stack) ===
-- | dune.flashbots.result_orderflow_version_orderflow_sankey | 3100173 |
-- | dune.flashbots.result_overall_of | 3146056 |
-- | dune.flashbots.result_overall_lq | 3146096 |
--
-- Refresh windows on Flashbots jobs are typically ~00:00–00:30 UTC (see their dashboard).

SELECT 1 AS _reference_only WHERE 1 = 0;
