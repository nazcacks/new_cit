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
            'ALTER TABLE %I.business_years
                ADD COLUMN IF NOT EXISTS original_by_id BIGINT REFERENCES %I.business_years(by_id),
                ADD COLUMN IF NOT EXISTS amendment_sequence INT NOT NULL DEFAULT 0,
                ADD COLUMN IF NOT EXISTS amendment_reason TEXT,
                ADD COLUMN IF NOT EXISTS version_mode VARCHAR(30)',
            tenant_schema,
            tenant_schema
        );
        EXECUTE format(
            'ALTER TABLE %I.business_years
                DROP CONSTRAINT IF EXISTS business_years_customer_id_year_label_key',
            tenant_schema
        );
        EXECUTE format(
            'CREATE UNIQUE INDEX IF NOT EXISTS idx_business_years_customer_year_sequence
                ON %I.business_years(customer_id, year_label, amendment_sequence)',
            tenant_schema
        );
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS idx_business_years_original
                ON %I.business_years(original_by_id, amendment_sequence)',
            tenant_schema
        );
    END LOOP;
END $$;
