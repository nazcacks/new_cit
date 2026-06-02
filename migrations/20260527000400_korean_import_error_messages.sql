DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM tenants
        WHERE to_regnamespace(schema_name) IS NOT NULL
          AND to_regclass(format('%I.import_errors', schema_name)) IS NOT NULL
    LOOP
        EXECUTE format($sql$
            UPDATE %I.import_errors
            SET message = regexp_replace(
                message,
                '^debit total (.+) does not match credit total (.+)$',
                '차변 합계 \1와 대변 합계 \2가 일치하지 않습니다.'
            )
            WHERE message ~ '^debit total .+ does not match credit total .+$';
        $sql$, tenant_schema);
    END LOOP;
END $$;
