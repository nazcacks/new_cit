DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM tenants
        WHERE to_regnamespace(schema_name) IS NOT NULL
          AND to_regclass(format('%I.business_years', schema_name)) IS NOT NULL
    LOOP
        EXECUTE format(
            'ALTER TABLE %I.business_years
                ADD COLUMN IF NOT EXISTS lock_mode VARCHAR(30) NOT NULL DEFAULT ''OPEN''',
            tenant_schema
        );
        EXECUTE format(
            'UPDATE %I.business_years
                SET lock_mode = CASE
                    WHEN status = ''FILED'' OR locked_at IS NOT NULL THEN ''FILED_LOCK''
                    WHEN status = ''AMENDED'' THEN ''AMENDMENT_UNLOCK''
                    ELSE COALESCE(NULLIF(lock_mode, ''''), ''OPEN'')
                END',
            tenant_schema
        );
    END LOOP;
END $$;

UPDATE users
SET pwd_fail_count = 0
WHERE locked = FALSE AND status = 'ACTIVE';
