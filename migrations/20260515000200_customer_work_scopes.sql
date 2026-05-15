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
            'ALTER TABLE %I.customers ADD COLUMN IF NOT EXISTS work_scopes TEXT[] NOT NULL DEFAULT ARRAY[''INFO'',''ADJUST'',''FORM'',''VALIDATE'',''PRINT'']::TEXT[]',
            tenant_schema
        );
    END LOOP;
END $$;
