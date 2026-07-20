DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM information_schema.schemata
        WHERE schema_name LIKE 'tenant\_%' ESCAPE '\'
    LOOP
        IF to_regclass(format('%I.customers', tenant_schema)) IS NOT NULL THEN
            EXECUTE format(
                'ALTER TABLE %I.customers
                 ADD COLUMN IF NOT EXISTS corp_type VARCHAR(20) NOT NULL DEFAULT ''DOMESTIC''',
                tenant_schema
            );
            EXECUTE format(
                'UPDATE %I.customers
                 SET corp_type = ''DOMESTIC''
                 WHERE corp_type IS NULL OR TRIM(corp_type) = ''''',
                tenant_schema
            );
        END IF;
    END LOOP;
END $$;
