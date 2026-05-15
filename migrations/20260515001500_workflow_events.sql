DO $$
DECLARE
    tenant_schema text;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name FROM tenants WHERE to_regnamespace(schema_name) IS NOT NULL
    LOOP
        EXECUTE format($sql$
            CREATE TABLE IF NOT EXISTS %I.workflow_events (
                event_id    BIGSERIAL PRIMARY KEY,
                by_id       BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                from_status VARCHAR(30),
                to_status   VARCHAR(30) NOT NULL,
                action      VARCHAR(50) NOT NULL,
                actor       VARCHAR(100) NOT NULL DEFAULT 'system',
                comment     TEXT,
                metadata    JSONB NOT NULL DEFAULT '{}'::jsonb,
                created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_workflow_events_by
                ON %I.workflow_events(by_id, created_at DESC);

            CREATE TABLE IF NOT EXISTS %I.approval_lines (
                line_id           BIGSERIAL PRIMARY KEY,
                by_id             BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                step_order        INT NOT NULL DEFAULT 1,
                approver_login_id VARCHAR(100) NOT NULL,
                status            VARCHAR(30) NOT NULL DEFAULT 'PENDING',
                acted_at          TIMESTAMPTZ,
                comment           TEXT,
                created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_approval_lines_by
                ON %I.approval_lines(by_id, step_order);
        $sql$, tenant_schema, tenant_schema, tenant_schema, tenant_schema, tenant_schema, tenant_schema);
    END LOOP;
END $$;
