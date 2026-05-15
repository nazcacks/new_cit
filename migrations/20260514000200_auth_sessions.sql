CREATE TABLE IF NOT EXISTS auth_sessions (
    session_token UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       BIGINT NOT NULL REFERENCES users(user_id),
    tenant_id     BIGINT NOT NULL REFERENCES tenants(tenant_id),
    issued_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '8 hours',
    revoked_at    TIMESTAMPTZ,
    last_seen_at  TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_auth_sessions_user ON auth_sessions(user_id, issued_at DESC);
CREATE INDEX IF NOT EXISTS idx_auth_sessions_active ON auth_sessions(session_token, expires_at)
WHERE revoked_at IS NULL;

INSERT INTO tenants (
    tenant_code,
    tenant_name,
    biz_reg_no,
    contract_start,
    schema_name,
    status,
    max_users
)
VALUES (
    'demo',
    'Demo Tax Firm',
    '1234567890',
    DATE '2026-01-01',
    'tenant_demo',
    'ACTIVE',
    20
)
ON CONFLICT (tenant_code) DO NOTHING;

INSERT INTO users (
    tenant_id,
    login_id,
    password_hash,
    user_name,
    email,
    use_2fa,
    status,
    pwd_changed_at
)
SELECT
    tenant_id,
    'admin',
    crypt('admin123!', gen_salt('bf')),
    '시스템 관리자',
    'admin@example.local',
    FALSE,
    'ACTIVE',
    NOW()
FROM tenants
WHERE tenant_code = 'demo'
ON CONFLICT (tenant_id, login_id) DO UPDATE
SET
    password_hash = EXCLUDED.password_hash,
    user_name = EXCLUDED.user_name,
    email = EXCLUDED.email,
    use_2fa = EXCLUDED.use_2fa,
    status = EXCLUDED.status,
    locked = FALSE,
    pwd_fail_count = 0;
