CREATE TABLE IF NOT EXISTS user_report_definitions (
    report_id    BIGSERIAL PRIMARY KEY,
    tenant_id    BIGINT NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    user_id      BIGINT REFERENCES users(user_id) ON DELETE SET NULL,
    report_name  VARCHAR(160) NOT NULL,
    source       VARCHAR(80) NOT NULL,
    columns      JSONB NOT NULL DEFAULT '[]'::jsonb,
    filters      JSONB NOT NULL DEFAULT '{}'::jsonb,
    active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_user_report_definitions_tenant
    ON user_report_definitions(tenant_id, active, created_at DESC);

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
        EXECUTE format($sql$
            CREATE TABLE IF NOT EXISTS %I.adjustment_items_history (
                history_id      BIGSERIAL PRIMARY KEY,
                adjustment_item_id BIGINT,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                source_module   VARCHAR(50) NOT NULL,
                action          VARCHAR(30) NOT NULL,
                old_data        JSONB,
                new_data        JSONB,
                changed_by      VARCHAR(100) NOT NULL DEFAULT 'system',
                changed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_adjustment_items_history_by
                ON %I.adjustment_items_history(by_id, source_module, changed_at DESC);

            CREATE TABLE IF NOT EXISTS %I.adjustment_item_attachments (
                attachment_id   BIGSERIAL PRIMARY KEY,
                adjustment_item_id BIGINT NOT NULL REFERENCES %I.adjustment_items(adjustment_item_id) ON DELETE CASCADE,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                file_name       VARCHAR(255) NOT NULL,
                content_type    VARCHAR(100) NOT NULL DEFAULT 'application/octet-stream',
                storage_url     TEXT,
                memo            TEXT,
                uploaded_by     VARCHAR(100) NOT NULL DEFAULT 'system',
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_adjustment_item_attachments_by
                ON %I.adjustment_item_attachments(by_id, adjustment_item_id, created_at DESC);

            CREATE TABLE IF NOT EXISTS %I.print_history (
                print_id        BIGSERIAL PRIMARY KEY,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                form_code       VARCHAR(50),
                file_name       VARCHAR(255) NOT NULL,
                content_type    VARCHAR(100) NOT NULL,
                watermark       VARCHAR(40) NOT NULL,
                status          VARCHAR(20) NOT NULL DEFAULT 'GENERATED',
                printed_by      VARCHAR(100) NOT NULL DEFAULT 'system',
                metadata        JSONB NOT NULL DEFAULT '{}'::jsonb,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_print_history_by
                ON %I.print_history(by_id, created_at DESC);
        $sql$,
            tenant_schema, tenant_schema, tenant_schema,
            tenant_schema, tenant_schema, tenant_schema, tenant_schema,
            tenant_schema, tenant_schema, tenant_schema
        );
    END LOOP;
END $$;
