CREATE TABLE IF NOT EXISTS roles (
    role_code   VARCHAR(50) PRIMARY KEY,
    role_name   VARCHAR(100) NOT NULL,
    description TEXT,
    system_role BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_roles (
    user_id    BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    role_code  VARCHAR(50) NOT NULL REFERENCES roles(role_code),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    granted_by VARCHAR(100) NOT NULL DEFAULT 'system',
    PRIMARY KEY (user_id, role_code)
);

CREATE TABLE IF NOT EXISTS role_permissions (
    role_code     VARCHAR(50) NOT NULL REFERENCES roles(role_code) ON DELETE CASCADE,
    module_code   VARCHAR(100) NOT NULL,
    function_code VARCHAR(50) NOT NULL,
    effect        VARCHAR(10) NOT NULL DEFAULT 'ALLOW' CHECK (effect IN ('ALLOW', 'DENY')),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_code, module_code, function_code)
);

CREATE TABLE IF NOT EXISTS user_customer_access (
    access_id    BIGSERIAL PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    tenant_id    BIGINT NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    customer_id  BIGINT NOT NULL,
    access_level VARCHAR(30) NOT NULL DEFAULT 'VIEWER'
        CHECK (access_level IN ('OWNER', 'CO_WORKER', 'REVIEWER', 'ASSISTANT', 'VIEWER', 'BLOCKED')),
    is_primary   BOOLEAN NOT NULL DEFAULT FALSE,
    valid_from   DATE,
    valid_to     DATE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, tenant_id, customer_id)
);
CREATE INDEX IF NOT EXISTS idx_user_customer_access_user ON user_customer_access(user_id);
CREATE INDEX IF NOT EXISTS idx_user_customer_access_customer ON user_customer_access(tenant_id, customer_id);

CREATE TABLE IF NOT EXISTS user_customer_work_scope (
    access_id  BIGINT NOT NULL REFERENCES user_customer_access(access_id) ON DELETE CASCADE,
    work_scope VARCHAR(30) NOT NULL
        CHECK (work_scope IN ('INFO', 'ADJUST', 'FORM', 'VALIDATE', 'APPROVE', 'PRINT', 'EFILE', 'POST')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (access_id, work_scope)
);

CREATE TABLE IF NOT EXISTS admin_audit_events (
    audit_id   BIGSERIAL PRIMARY KEY,
    actor      VARCHAR(100) NOT NULL DEFAULT 'system',
    action     VARCHAR(100) NOT NULL,
    target     VARCHAR(200) NOT NULL,
    metadata   JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO roles (role_code, role_name, description, system_role)
VALUES
    ('SUPER_ADMIN', '시스템 관리자', '전체 시스템 설정과 테넌트 관리', TRUE),
    ('TENANT_ADMIN', '테넌트 관리자', '소속 테넌트 사용자와 고객사 권한 관리', TRUE),
    ('TAX_EXPERT', '세무조정 담당자', '세무정보 입력과 세무조정 수행', TRUE),
    ('TAX_REVIEWER', '검토자', '검증과 결재 검토 수행', TRUE),
    ('ASSISTANT', '업무 보조자', '기초정보와 서식 입력 보조', TRUE),
    ('VIEWER', '조회 전용', '조회와 출력 제한 권한', TRUE)
ON CONFLICT (role_code) DO UPDATE
SET role_name = EXCLUDED.role_name,
    description = EXCLUDED.description,
    system_role = EXCLUDED.system_role;

INSERT INTO role_permissions (role_code, module_code, function_code, effect)
VALUES
    ('SUPER_ADMIN', 'admin', 'READ', 'ALLOW'),
    ('SUPER_ADMIN', 'admin', 'CREATE', 'ALLOW'),
    ('SUPER_ADMIN', 'admin', 'UPDATE', 'ALLOW'),
    ('SUPER_ADMIN', 'admin', 'DELETE', 'ALLOW'),
    ('TENANT_ADMIN', 'admin.users', 'READ', 'ALLOW'),
    ('TENANT_ADMIN', 'admin.users', 'CREATE', 'ALLOW'),
    ('TENANT_ADMIN', 'admin.users', 'UPDATE', 'ALLOW'),
    ('TENANT_ADMIN', 'admin.access', 'READ', 'ALLOW'),
    ('TENANT_ADMIN', 'admin.access', 'UPDATE', 'ALLOW'),
    ('TAX_EXPERT', 'adjustment', 'READ', 'ALLOW'),
    ('TAX_EXPERT', 'adjustment', 'CREATE', 'ALLOW'),
    ('TAX_EXPERT', 'forms', 'CREATE', 'ALLOW'),
    ('TAX_REVIEWER', 'adjustment', 'READ', 'ALLOW'),
    ('TAX_REVIEWER', 'workflow', 'APPROVE', 'ALLOW'),
    ('ASSISTANT', 'tax-data', 'CREATE', 'ALLOW'),
    ('VIEWER', 'dashboard', 'READ', 'ALLOW')
ON CONFLICT (role_code, module_code, function_code) DO UPDATE
SET effect = EXCLUDED.effect,
    updated_at = NOW();

INSERT INTO user_roles (user_id, role_code, granted_by)
SELECT u.user_id, 'SUPER_ADMIN', 'migration'
FROM users u
JOIN tenants t ON t.tenant_id = u.tenant_id
WHERE t.tenant_code = 'demo' AND u.login_id = 'admin'
ON CONFLICT (user_id, role_code) DO NOTHING;
