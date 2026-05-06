-- Q8 · Minimal tx list for 1inch Classic Integrators (cheap to run on Dune)
--
-- Goal: provide per-tx hashes (and trade_usd) so local code can decode `feeRecipient`
-- via RPC without doing calldata decoding on Dune.
--
-- Output columns (used by local cache / api):
--   - hash (VARBINARY/STRING)
--   - trade_usd (DOUBLE)
--   - block_time (optional)
--
-- Tune the time window to control cost/size.
WITH deduped AS (
  SELECT hash, trade_usd, block_time,
         ROW_NUMBER() OVER (PARTITION BY hash ORDER BY block_time) AS rn
  FROM dune.flashbots.result_overall_of
  WHERE solver LIKE '%1inch%'
    AND frontend = '1inch Integrators'
)
SELECT hash, trade_usd, block_time
FROM deduped
WHERE rn = 1
LIMIT 200000;

