INSERT INTO standard_accounts (
    code, name_ko, fs_type, account_class, normal_balance, tax_relevance, sub_class, sort_order
)
VALUES (
    'ACCUM_DEPR',
    'Accumulated depreciation',
    'BS',
    'CONTRA',
    'CREDIT',
    'PPE',
    'CONTRA_ASSET',
    55
)
ON CONFLICT (code) DO UPDATE
SET name_ko = EXCLUDED.name_ko,
    fs_type = EXCLUDED.fs_type,
    account_class = EXCLUDED.account_class,
    normal_balance = EXCLUDED.normal_balance,
    tax_relevance = EXCLUDED.tax_relevance,
    sub_class = EXCLUDED.sub_class,
    sort_order = EXCLUDED.sort_order,
    is_active = TRUE;

INSERT INTO validation_rules (rule_code, severity, area, message_template, applies_to)
VALUES
    (
        'CHK_PPE_COST',
        'ERROR',
        'tax-data',
        'Asset register PPE cost must match BS and standard BS PPE cost. Difference: {ppe_cost_diff}',
        '#/workspace/ws-info/assets'
    ),
    (
        'CHK_ACCUM_DEPR',
        'ERROR',
        'tax-data',
        'Asset register accumulated depreciation must match BS accumulated depreciation. Difference: {accum_depr_diff}',
        '#/workspace/ws-info/assets'
    ),
    (
        'CHK_INTANGIBLE',
        'ERROR',
        'tax-data',
        'Asset register intangible assets must match BS and standard BS intangible assets. Difference: {intangible_diff}',
        '#/workspace/ws-info/assets'
    )
ON CONFLICT (rule_code) DO UPDATE
SET severity = EXCLUDED.severity,
    area = EXCLUDED.area,
    message_template = EXCLUDED.message_template,
    applies_to = EXCLUDED.applies_to,
    active = TRUE;

DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM information_schema.schemata
        WHERE schema_name LIKE 'tenant\_%' ESCAPE '\'
    LOOP
        IF to_regclass(format('%I.assets', tenant_schema)) IS NOT NULL THEN
            EXECUTE format('ALTER TABLE %I.assets ADD COLUMN IF NOT EXISTS depr_method VARCHAR(20) NOT NULL DEFAULT ''SL''', tenant_schema);
            EXECUTE format('ALTER TABLE %I.assets ADD COLUMN IF NOT EXISTS residual_value BIGINT NOT NULL DEFAULT 0', tenant_schema);
            EXECUTE format('ALTER TABLE %I.assets ADD COLUMN IF NOT EXISTS accumulated_depr_prior BIGINT NOT NULL DEFAULT 0', tenant_schema);
            EXECUTE format('ALTER TABLE %I.assets ADD COLUMN IF NOT EXISTS acct_depr_current BIGINT NOT NULL DEFAULT 0', tenant_schema);
            EXECUTE format('ALTER TABLE %I.assets ADD COLUMN IF NOT EXISTS tax_depr_rate_bps INT', tenant_schema);
            EXECUTE format('ALTER TABLE %I.assets ADD COLUMN IF NOT EXISTS tax_depr_limit BIGINT NOT NULL DEFAULT 0', tenant_schema);
            EXECUTE format('ALTER TABLE %I.assets ADD COLUMN IF NOT EXISTS depr_excess BIGINT NOT NULL DEFAULT 0', tenant_schema);
            EXECUTE format('ALTER TABLE %I.assets ADD COLUMN IF NOT EXISTS depr_shortfall BIGINT NOT NULL DEFAULT 0', tenant_schema);
            EXECUTE format('ALTER TABLE %I.assets ADD COLUMN IF NOT EXISTS prev_year_asset_id BIGINT REFERENCES %I.assets(asset_id)', tenant_schema, tenant_schema);
            EXECUTE format('ALTER TABLE %I.assets ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()', tenant_schema);
            EXECUTE format('CREATE INDEX IF NOT EXISTS idx_assets_prev_year ON %I.assets(prev_year_asset_id)', tenant_schema);
        END IF;

        IF to_regclass(format('%I.depreciation', tenant_schema)) IS NOT NULL THEN
            EXECUTE format('ALTER TABLE %I.depreciation ADD COLUMN IF NOT EXISTS shortfall_amount BIGINT NOT NULL DEFAULT 0', tenant_schema);
            EXECUTE format('ALTER TABLE %I.depreciation ADD COLUMN IF NOT EXISTS metadata JSONB NOT NULL DEFAULT ''{}''::jsonb', tenant_schema);
        END IF;
    END LOOP;
END $$;
