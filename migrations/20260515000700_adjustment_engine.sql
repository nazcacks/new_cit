DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM information_schema.schemata
        WHERE schema_name LIKE 'tenant\_%' ESCAPE '\'
    LOOP
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I.adjustment_items (
                adjustment_item_id BIGSERIAL PRIMARY KEY,
                by_id BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                adjustment_id BIGINT REFERENCES %I.tax_adjustments(adjustment_id) ON DELETE SET NULL,
                section VARCHAR(50) NOT NULL,
                item_code VARCHAR(80) NOT NULL,
                item_name VARCHAR(200) NOT NULL,
                amount BIGINT NOT NULL,
                direction VARCHAR(20) NOT NULL CHECK (direction IN (''ADD'', ''DEDUCT'', ''INFO'')),
                disposition VARCHAR(40) NOT NULL DEFAULT ''OTHER'',
                source_module VARCHAR(50) NOT NULL DEFAULT ''B1'',
                law_ref VARCHAR(100),
                metadata JSONB NOT NULL DEFAULT ''{}''::jsonb,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )',
            tenant_schema,
            tenant_schema,
            tenant_schema
        );
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS idx_adjustment_items_by
             ON %I.adjustment_items(by_id, source_module, section, item_code)',
            tenant_schema
        );
        EXECUTE format(
            'ALTER TABLE %I.reserves
             ADD COLUMN IF NOT EXISTS source_module VARCHAR(50) NOT NULL DEFAULT ''MANUAL''',
            tenant_schema
        );
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS idx_reserves_by
             ON %I.reserves(by_id, source_module, reserve_code)',
            tenant_schema
        );
    END LOOP;
END $$;
