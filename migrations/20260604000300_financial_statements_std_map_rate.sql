DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM information_schema.schemata
        WHERE schema_name LIKE 'tenant\_%' ESCAPE '\'
    LOOP
        IF to_regclass(format('%I.financial_statements', tenant_schema)) IS NOT NULL THEN
            EXECUTE format(
                'ALTER TABLE %I.financial_statements
                 ADD COLUMN IF NOT EXISTS std_map_rate DOUBLE PRECISION NOT NULL DEFAULT 0',
                tenant_schema
            );
        END IF;
    END LOOP;
END $$;
