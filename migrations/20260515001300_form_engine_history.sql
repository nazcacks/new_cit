DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM tenants
        WHERE EXISTS (
            SELECT 1
            FROM information_schema.schemata
            WHERE schema_name = tenants.schema_name
        )
    LOOP
        EXECUTE format($sql$
            CREATE TABLE IF NOT EXISTS %I.form_data_history (
                history_id      BIGSERIAL PRIMARY KEY,
                form_data_id    BIGINT NOT NULL REFERENCES %I.form_data(form_data_id) ON DELETE CASCADE,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                form_code       VARCHAR(50) NOT NULL,
                change_type     VARCHAR(30) NOT NULL,
                changed_by      VARCHAR(100) NOT NULL DEFAULT 'system',
                reason          TEXT,
                old_data        JSONB,
                new_data        JSONB NOT NULL,
                changed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $sql$, tenant_schema, tenant_schema, tenant_schema);
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.form_data_history(by_id, form_code, changed_at DESC)',
            'idx_' || tenant_schema || '_form_data_history_by',
            tenant_schema
        );
    END LOOP;
END $$;
