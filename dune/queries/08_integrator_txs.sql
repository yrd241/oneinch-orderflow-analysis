-- Q8 · 1inch Integrators tx list for local feeRecipient decode (Classic Router only)
--
-- Superseded detail / full comments: dune/queries/11_integrators_classic_router_hashes.sql
-- Run Q11 on Dune and download CSV, or use scripts/fetch_sample_txs_by_frontend.py

WITH deduped AS (
  SELECT
    CAST(hash AS VARCHAR) AS hash,
    trade_usd,
    block_time,
    ROW_NUMBER() OVER (PARTITION BY hash ORDER BY block_time) AS rn
  FROM dune.flashbots.result_overall_of
  WHERE solver LIKE '%1inch%'
    AND frontend IN ('1inch Integrators', '1inch Website: Default')
),
classic AS (
  SELECT d.hash, d.trade_usd, d.block_time
  FROM deduped d
  WHERE d.rn = 1
    AND EXISTS (
      SELECT 1
      FROM ethereum.transactions t
      WHERE CAST(t.hash AS VARCHAR) = d.hash
        AND t.to IN (
          0x111111125434b319222cdbf8c261674adb56f3ae,
          0x11111112542d85b3ef69ae05771c2dccff4faa26,
          0x1111111254fb6c44bac0bed2854e76f90643097d,
          0x1111111254eeb25477b68fb85ed929f73a960582,
          0x111111125421ca6dc452d289314280a0f8842a65
        )
    )
)
SELECT hash, trade_usd, block_time
FROM classic
ORDER BY block_time DESC
