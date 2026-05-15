DO $$
DECLARE
    tenant_schema text;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM tenants
        WHERE to_regnamespace(schema_name) IS NOT NULL
          AND to_regclass(format('%I.business_years', schema_name)) IS NOT NULL
    LOOP
        EXECUTE format($sql$
            CREATE TABLE IF NOT EXISTS %I.audit_logs (
                audit_id        BIGSERIAL PRIMARY KEY,
                table_name      VARCHAR(100) NOT NULL,
                record_id       VARCHAR(100) NOT NULL,
                action          VARCHAR(20) NOT NULL,
                old_data        JSONB,
                new_data        JSONB,
                changed_by      VARCHAR(100) NOT NULL DEFAULT 'system',
                changed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            ALTER TABLE %I.audit_logs
                ADD COLUMN IF NOT EXISTS event_date DATE NOT NULL DEFAULT CURRENT_DATE,
                ADD COLUMN IF NOT EXISTS prev_hash VARCHAR(64),
                ADD COLUMN IF NOT EXISTS hash_current VARCHAR(64);
            CREATE INDEX IF NOT EXISTS idx_audit_logs_event_date
                ON %I.audit_logs(event_date, audit_id);

            CREATE TABLE IF NOT EXISTS %I.notifications (
                notification_id BIGSERIAL PRIMARY KEY,
                by_id           BIGINT REFERENCES %I.business_years(by_id),
                title           VARCHAR(200) NOT NULL,
                message         TEXT NOT NULL,
                severity        VARCHAR(20) NOT NULL DEFAULT 'INFO',
                status          VARCHAR(20) NOT NULL DEFAULT 'UNREAD',
                metadata        JSONB NOT NULL DEFAULT '{}'::jsonb,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                read_at         TIMESTAMPTZ
            );
            CREATE INDEX IF NOT EXISTS idx_notifications_status
                ON %I.notifications(status, created_at DESC);
        $sql$, tenant_schema, tenant_schema, tenant_schema, tenant_schema, tenant_schema, tenant_schema);
    END LOOP;
END $$;
