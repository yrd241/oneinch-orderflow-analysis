-- Q11 · 1inch Integrators tx hashes — Classic Aggregation Router only (excludes Fusion)
--
-- Aligns with the same universe as dune/queries/07_1inch_sankey.sql (result_overall_of, all rows
-- in the MV — not a 7-day window). Filters OUT Fusion / LOP fills by requiring:
--   ethereum.transactions.to = 1inch Aggregation Router V2–V6
--
-- Use on Dune: Run → Download CSV → local decode_fee_recipient.py
--
-- Same frontend bucketing as Q7 / fetch_sample_txs_by_frontend.py:
--   1inch Website: Default → 1inch Integrators

WITH deduped AS (
  SELECT
    CASE
      WHEN frontend = '1inch Website: Default' THEN '1inch Integrators'
      ELSE frontend
    END AS frontend,
    CAST(hash AS VARCHAR) AS hash,
    block_time,
    CAST("user" AS VARCHAR) AS user_addr,
    solver,
    trade_usd,
    ROW_NUMBER() OVER (PARTITION BY hash ORDER BY block_time) AS rn
  FROM dune.flashbots.result_overall_of
  WHERE solver LIKE '%1inch%'
    AND frontend IN ('1inch Integrators', '1inch Website: Default')
),
classic AS (
  SELECT d.frontend, d.hash, d.block_time, d.user_addr, d.solver, d.trade_usd
  FROM deduped d
  WHERE d.rn = 1
    AND EXISTS (
      SELECT 1
      FROM ethereum.transactions t
      WHERE CAST(t.hash AS VARCHAR) = d.hash
        AND t.to IN (
          0x111111125434b319222cdbf8c261674adb56f3ae, -- Aggregation Router V2
          0x11111112542d85b3ef69ae05771c2dccff4faa26, -- V3
          0x1111111254fb6c44bac0bed2854e76f90643097d, -- V4
          0x1111111254eeb25477b68fb85ed929f73a960582, -- V5
          0x111111125421ca6dc452d289314280a0f8842a65  -- V6
        )
    )
)
SELECT
  frontend,
  hash,
  block_time,
  user_addr,
  solver,
  trade_usd
FROM classic
ORDER BY block_time DESC

-- Optional: cap for testing
-- LIMIT 1000
