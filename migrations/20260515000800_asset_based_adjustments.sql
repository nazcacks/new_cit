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
            'CREATE TABLE IF NOT EXISTS %I.vehicle_usage_logs (
                usage_log_id BIGSERIAL PRIMARY KEY,
                by_id BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                asset_id BIGINT NOT NULL REFERENCES %I.assets(asset_id),
                usage_month DATE NOT NULL,
                total_distance_km DOUBLE PRECISION NOT NULL DEFAULT 0,
                business_distance_km DOUBLE PRECISION NOT NULL DEFAULT 0,
                business_use_bps INT NOT NULL DEFAULT 10000,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE(by_id, asset_id, usage_month)
            )',
            tenant_schema,
            tenant_schema,
            tenant_schema
        );
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS idx_vehicle_usage_logs_by
             ON %I.vehicle_usage_logs(by_id, asset_id, usage_month)',
            tenant_schema
        );
    END LOOP;
END $$;
