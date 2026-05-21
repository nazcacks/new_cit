CREATE TABLE IF NOT EXISTS leaf_records (
    record_id BIGSERIAL PRIMARY KEY,
    tenant_code VARCHAR(50) NOT NULL REFERENCES tenants(tenant_code) ON DELETE CASCADE,
    leaf_key VARCHAR(120) NOT NULL,
    data JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_leaf_records_tenant_leaf_active
    ON leaf_records (tenant_code, leaf_key, record_id)
    WHERE deleted_at IS NULL;
