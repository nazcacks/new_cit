DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM tenants
        WHERE to_regnamespace(schema_name) IS NOT NULL
          AND to_regclass(format('%I.efiling_history', schema_name)) IS NOT NULL
    LOOP
        EXECUTE format(
            'ALTER TABLE %I.efiling_history ADD COLUMN IF NOT EXISTS receipt_no VARCHAR(80)',
            tenant_schema
        );
        EXECUTE format(
            'ALTER TABLE %I.efiling_history ADD COLUMN IF NOT EXISTS receipt_at TIMESTAMPTZ',
            tenant_schema
        );
    END LOOP;
END $$;
