INSERT INTO tax_law_versions (version_code, law_name, effective_from, effective_to, status, metadata)
VALUES
    ('CIT-2021', 'Corporate Income Tax Act 2021', DATE '2021-01-01', DATE '2021-12-31', 'APPROVED', '{"source":"seed","seed_range":"2021-2026"}'),
    ('CIT-2022', 'Corporate Income Tax Act 2022', DATE '2022-01-01', DATE '2022-12-31', 'APPROVED', '{"source":"seed","seed_range":"2021-2026"}'),
    ('CIT-2023', 'Corporate Income Tax Act 2023', DATE '2023-01-01', DATE '2023-12-31', 'APPROVED', '{"source":"seed","seed_range":"2021-2026"}')
ON CONFLICT (version_code) DO NOTHING;

INSERT INTO tax_rates (
    law_version_id,
    item_code,
    taxable_from,
    taxable_to,
    base_tax,
    rate_bps,
    progressive_deduction,
    effective_from,
    effective_to,
    metadata
)
SELECT
    law.law_version_id,
    seed.item_code,
    seed.taxable_from,
    seed.taxable_to,
    seed.base_tax,
    seed.rate_bps,
    seed.progressive_deduction,
    seed.effective_from,
    seed.effective_to,
    seed.metadata
FROM (
    VALUES
        ('CIT-2021', 'CORPORATE_TAX', 0::BIGINT, 200000000::BIGINT, 0::BIGINT, 1000, 0::BIGINT, DATE '2021-01-01', DATE '2021-12-31', '{"bracket":"0-200M","source":"seed"}'::jsonb),
        ('CIT-2021', 'CORPORATE_TAX', 200000001::BIGINT, 20000000000::BIGINT, 0::BIGINT, 2000, 20000000::BIGINT, DATE '2021-01-01', DATE '2021-12-31', '{"bracket":"200M-20B","source":"seed"}'::jsonb),
        ('CIT-2021', 'CORPORATE_TAX', 20000000001::BIGINT, 300000000000::BIGINT, 0::BIGINT, 2200, 420000000::BIGINT, DATE '2021-01-01', DATE '2021-12-31', '{"bracket":"20B-300B","source":"seed"}'::jsonb),
        ('CIT-2021', 'CORPORATE_TAX', 300000000001::BIGINT, NULL::BIGINT, 0::BIGINT, 2500, 9420000000::BIGINT, DATE '2021-01-01', DATE '2021-12-31', '{"bracket":"300B+","source":"seed"}'::jsonb),
        ('CIT-2022', 'CORPORATE_TAX', 0::BIGINT, 200000000::BIGINT, 0::BIGINT, 1000, 0::BIGINT, DATE '2022-01-01', DATE '2022-12-31', '{"bracket":"0-200M","source":"seed"}'::jsonb),
        ('CIT-2022', 'CORPORATE_TAX', 200000001::BIGINT, 20000000000::BIGINT, 0::BIGINT, 2000, 20000000::BIGINT, DATE '2022-01-01', DATE '2022-12-31', '{"bracket":"200M-20B","source":"seed"}'::jsonb),
        ('CIT-2022', 'CORPORATE_TAX', 20000000001::BIGINT, 300000000000::BIGINT, 0::BIGINT, 2200, 420000000::BIGINT, DATE '2022-01-01', DATE '2022-12-31', '{"bracket":"20B-300B","source":"seed"}'::jsonb),
        ('CIT-2022', 'CORPORATE_TAX', 300000000001::BIGINT, NULL::BIGINT, 0::BIGINT, 2500, 9420000000::BIGINT, DATE '2022-01-01', DATE '2022-12-31', '{"bracket":"300B+","source":"seed"}'::jsonb),
        ('CIT-2023', 'CORPORATE_TAX', 0::BIGINT, 200000000::BIGINT, 0::BIGINT, 900, 0::BIGINT, DATE '2023-01-01', DATE '2023-12-31', '{"bracket":"0-200M","source":"seed"}'::jsonb),
        ('CIT-2023', 'CORPORATE_TAX', 200000001::BIGINT, 20000000000::BIGINT, 0::BIGINT, 1900, 20000000::BIGINT, DATE '2023-01-01', DATE '2023-12-31', '{"bracket":"200M-20B","source":"seed"}'::jsonb),
        ('CIT-2023', 'CORPORATE_TAX', 20000000001::BIGINT, 300000000000::BIGINT, 0::BIGINT, 2100, 420000000::BIGINT, DATE '2023-01-01', DATE '2023-12-31', '{"bracket":"20B-300B","source":"seed"}'::jsonb),
        ('CIT-2023', 'CORPORATE_TAX', 300000000001::BIGINT, NULL::BIGINT, 0::BIGINT, 2400, 9420000000::BIGINT, DATE '2023-01-01', DATE '2023-12-31', '{"bracket":"300B+","source":"seed"}'::jsonb)
) AS seed(version_code, item_code, taxable_from, taxable_to, base_tax, rate_bps, progressive_deduction, effective_from, effective_to, metadata)
JOIN tax_law_versions law ON law.version_code = seed.version_code
ON CONFLICT (law_version_id, item_code, taxable_from) DO NOTHING;

INSERT INTO tax_limits (law_version_id, item_code, amount, effective_from, effective_to, metadata)
SELECT law.law_version_id, seed.item_code, seed.amount, seed.effective_from, seed.effective_to, seed.metadata
FROM (
    VALUES
        ('CIT-2021', 'ENTERTAINMENT_BASE_LIMIT', 12000000::BIGINT, DATE '2021-01-01', DATE '2021-12-31', '{"category":"LIMIT","description":"base entertainment expense limit"}'::jsonb),
        ('CIT-2022', 'ENTERTAINMENT_BASE_LIMIT', 12000000::BIGINT, DATE '2022-01-01', DATE '2022-12-31', '{"category":"LIMIT","description":"base entertainment expense limit"}'::jsonb),
        ('CIT-2023', 'ENTERTAINMENT_BASE_LIMIT', 12000000::BIGINT, DATE '2023-01-01', DATE '2023-12-31', '{"category":"LIMIT","description":"base entertainment expense limit"}'::jsonb),
        ('CIT-2024', 'ENTERTAINMENT_BASE_LIMIT', 12000000::BIGINT, DATE '2024-01-01', DATE '2024-12-31', '{"category":"LIMIT","description":"base entertainment expense limit"}'::jsonb),
        ('CIT-2025', 'ENTERTAINMENT_BASE_LIMIT', 12000000::BIGINT, DATE '2025-01-01', DATE '2025-12-31', '{"category":"LIMIT","description":"base entertainment expense limit"}'::jsonb),
        ('CIT-2026', 'ENTERTAINMENT_BASE_LIMIT', 12000000::BIGINT, DATE '2026-01-01', NULL::DATE, '{"category":"LIMIT","description":"base entertainment expense limit"}'::jsonb),
        ('CIT-2021', 'RND_CREDIT_BPS', 2500::BIGINT, DATE '2021-01-01', DATE '2021-12-31', '{"category":"CREDIT","description":"R&D tax credit rate in bps"}'::jsonb),
        ('CIT-2022', 'RND_CREDIT_BPS', 2500::BIGINT, DATE '2022-01-01', DATE '2022-12-31', '{"category":"CREDIT","description":"R&D tax credit rate in bps"}'::jsonb),
        ('CIT-2023', 'RND_CREDIT_BPS', 2500::BIGINT, DATE '2023-01-01', DATE '2023-12-31', '{"category":"CREDIT","description":"R&D tax credit rate in bps"}'::jsonb),
        ('CIT-2024', 'RND_CREDIT_BPS', 2500::BIGINT, DATE '2024-01-01', DATE '2024-12-31', '{"category":"CREDIT","description":"R&D tax credit rate in bps"}'::jsonb),
        ('CIT-2025', 'RND_CREDIT_BPS', 2500::BIGINT, DATE '2025-01-01', DATE '2025-12-31', '{"category":"CREDIT","description":"R&D tax credit rate in bps"}'::jsonb),
        ('CIT-2026', 'RND_CREDIT_BPS', 2500::BIGINT, DATE '2026-01-01', NULL::DATE, '{"category":"CREDIT","description":"R&D tax credit rate in bps"}'::jsonb),
        ('CIT-2021', 'MACHINE_USEFUL_LIFE_YEARS', 5::BIGINT, DATE '2021-01-01', DATE '2021-12-31', '{"category":"DEPRECIATION_LIFE","asset_category":"MACHINE"}'::jsonb),
        ('CIT-2022', 'MACHINE_USEFUL_LIFE_YEARS', 5::BIGINT, DATE '2022-01-01', DATE '2022-12-31', '{"category":"DEPRECIATION_LIFE","asset_category":"MACHINE"}'::jsonb),
        ('CIT-2023', 'MACHINE_USEFUL_LIFE_YEARS', 5::BIGINT, DATE '2023-01-01', DATE '2023-12-31', '{"category":"DEPRECIATION_LIFE","asset_category":"MACHINE"}'::jsonb),
        ('CIT-2024', 'MACHINE_USEFUL_LIFE_YEARS', 5::BIGINT, DATE '2024-01-01', DATE '2024-12-31', '{"category":"DEPRECIATION_LIFE","asset_category":"MACHINE"}'::jsonb),
        ('CIT-2025', 'MACHINE_USEFUL_LIFE_YEARS', 5::BIGINT, DATE '2025-01-01', DATE '2025-12-31', '{"category":"DEPRECIATION_LIFE","asset_category":"MACHINE"}'::jsonb),
        ('CIT-2026', 'MACHINE_USEFUL_LIFE_YEARS', 5::BIGINT, DATE '2026-01-01', NULL::DATE, '{"category":"DEPRECIATION_LIFE","asset_category":"MACHINE"}'::jsonb),
        ('CIT-2021', 'SME_REVENUE_LIMIT', 12000000000::BIGINT, DATE '2021-01-01', DATE '2021-12-31', '{"category":"SME_CRITERIA","description":"SME revenue threshold"}'::jsonb),
        ('CIT-2022', 'SME_REVENUE_LIMIT', 12000000000::BIGINT, DATE '2022-01-01', DATE '2022-12-31', '{"category":"SME_CRITERIA","description":"SME revenue threshold"}'::jsonb),
        ('CIT-2023', 'SME_REVENUE_LIMIT', 12000000000::BIGINT, DATE '2023-01-01', DATE '2023-12-31', '{"category":"SME_CRITERIA","description":"SME revenue threshold"}'::jsonb),
        ('CIT-2024', 'SME_REVENUE_LIMIT', 12000000000::BIGINT, DATE '2024-01-01', DATE '2024-12-31', '{"category":"SME_CRITERIA","description":"SME revenue threshold"}'::jsonb),
        ('CIT-2025', 'SME_REVENUE_LIMIT', 12000000000::BIGINT, DATE '2025-01-01', DATE '2025-12-31', '{"category":"SME_CRITERIA","description":"SME revenue threshold"}'::jsonb),
        ('CIT-2026', 'SME_REVENUE_LIMIT', 12000000000::BIGINT, DATE '2026-01-01', NULL::DATE, '{"category":"SME_CRITERIA","description":"SME revenue threshold"}'::jsonb),
        ('CIT-2021', 'LOSS_CARRYFORWARD_YEARS', 10::BIGINT, DATE '2021-01-01', DATE '2021-12-31', '{"category":"LOSS_RULE","description":"loss carryforward years"}'::jsonb),
        ('CIT-2022', 'LOSS_CARRYFORWARD_YEARS', 10::BIGINT, DATE '2022-01-01', DATE '2022-12-31', '{"category":"LOSS_RULE","description":"loss carryforward years"}'::jsonb),
        ('CIT-2023', 'LOSS_CARRYFORWARD_YEARS', 15::BIGINT, DATE '2023-01-01', DATE '2023-12-31', '{"category":"LOSS_RULE","description":"loss carryforward years"}'::jsonb),
        ('CIT-2024', 'LOSS_CARRYFORWARD_YEARS', 15::BIGINT, DATE '2024-01-01', DATE '2024-12-31', '{"category":"LOSS_RULE","description":"loss carryforward years"}'::jsonb),
        ('CIT-2025', 'LOSS_CARRYFORWARD_YEARS', 15::BIGINT, DATE '2025-01-01', DATE '2025-12-31', '{"category":"LOSS_RULE","description":"loss carryforward years"}'::jsonb),
        ('CIT-2026', 'LOSS_CARRYFORWARD_YEARS', 15::BIGINT, DATE '2026-01-01', NULL::DATE, '{"category":"LOSS_RULE","description":"loss carryforward years"}'::jsonb)
) AS seed(version_code, item_code, amount, effective_from, effective_to, metadata)
JOIN tax_law_versions law ON law.version_code = seed.version_code
WHERE NOT EXISTS (
    SELECT 1
    FROM tax_limits existing
    WHERE existing.law_version_id = law.law_version_id
      AND existing.item_code = seed.item_code
      AND existing.effective_from = seed.effective_from
);
