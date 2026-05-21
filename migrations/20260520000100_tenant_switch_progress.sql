ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS plan VARCHAR(30) NOT NULL DEFAULT 'STANDARD',
    ADD COLUMN IF NOT EXISTS suspended_at TIMESTAMPTZ;

UPDATE tenants
SET plan = COALESCE(NULLIF(plan, ''), 'STANDARD')
WHERE plan IS NULL OR plan = '';

CREATE INDEX IF NOT EXISTS idx_tenants_status_code
    ON tenants(status, tenant_code);
