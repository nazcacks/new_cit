CREATE TABLE IF NOT EXISTS function_codes (
    function_code VARCHAR(50) PRIMARY KEY,
    function_name VARCHAR(100) NOT NULL,
    description   TEXT,
    sort_order    INT NOT NULL DEFAULT 0,
    active        BOOLEAN NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS menu_functions (
    menu_key      VARCHAR(80) NOT NULL REFERENCES menu_nodes(menu_key) ON DELETE CASCADE,
    function_code VARCHAR(50) NOT NULL REFERENCES function_codes(function_code),
    enabled       BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (menu_key, function_code)
);

CREATE TABLE IF NOT EXISTS role_menu_functions (
    role_code     VARCHAR(50) NOT NULL REFERENCES roles(role_code) ON DELETE CASCADE,
    menu_key      VARCHAR(80) NOT NULL REFERENCES menu_nodes(menu_key) ON DELETE CASCADE,
    function_code VARCHAR(50) NOT NULL REFERENCES function_codes(function_code),
    effect        VARCHAR(10) NOT NULL DEFAULT 'ALLOW' CHECK (effect IN ('ALLOW', 'DENY')),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_code, menu_key, function_code)
);

CREATE TABLE IF NOT EXISTS role_data_scopes (
    role_code   VARCHAR(50) NOT NULL REFERENCES roles(role_code) ON DELETE CASCADE,
    module_code VARCHAR(100) NOT NULL DEFAULT '*',
    data_scope  VARCHAR(20) NOT NULL CHECK (data_scope IN ('ALL', 'ASSIGNED', 'OWNED', 'NONE')),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_code, module_code)
);

CREATE TABLE IF NOT EXISTS field_masking_policies (
    policy_id     BIGSERIAL PRIMARY KEY,
    module_code   VARCHAR(100) NOT NULL,
    field_path    VARCHAR(200) NOT NULL,
    mask_type     VARCHAR(30) NOT NULL DEFAULT 'PARTIAL',
    active        BOOLEAN NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(module_code, field_path)
);

CREATE TABLE IF NOT EXISTS access_delegations (
    delegation_id     BIGSERIAL PRIMARY KEY,
    tenant_id         BIGINT NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    grantor_user_id   BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    delegatee_user_id BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    customer_id       BIGINT NOT NULL,
    work_scope        VARCHAR(30) NOT NULL,
    valid_from        DATE,
    valid_to          DATE,
    status            VARCHAR(20) NOT NULL DEFAULT 'ACTIVE'
        CHECK (status IN ('ACTIVE', 'REVOKED', 'EXPIRED')),
    reason            TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_access_delegations_delegatee
    ON access_delegations(tenant_id, delegatee_user_id, customer_id, status);

INSERT INTO function_codes (function_code, function_name, description, sort_order)
VALUES
    ('READ', 'Read', 'View records and screens', 10),
    ('CREATE', 'Create', 'Create records', 20),
    ('UPDATE', 'Update', 'Update records', 30),
    ('DELETE', 'Delete', 'Delete records', 40),
    ('IMPORT', 'Import', 'Import files or rows', 50),
    ('EXPORT', 'Export', 'Export files or rows', 60),
    ('CALCULATE', 'Calculate', 'Run tax calculations', 70),
    ('APPROVE', 'Approve', 'Approve workflow items', 80),
    ('EFILE', 'E-file', 'Create e-filing files', 90),
    ('PRINT', 'Print', 'Generate PDF/print output', 100),
    ('MASK_OFF', 'Unmask', 'View unmasked sensitive fields', 110),
    ('DELEGATE', 'Delegate', 'Delegate assigned access', 120)
ON CONFLICT (function_code) DO UPDATE
SET function_name = EXCLUDED.function_name,
    description = EXCLUDED.description,
    sort_order = EXCLUDED.sort_order,
    active = TRUE;

INSERT INTO menu_functions (menu_key, function_code, enabled)
SELECT DISTINCT menu_key, function_code, TRUE
FROM (
    SELECT menu_key, COALESCE(required_perm_function, 'READ') AS function_code
    FROM menu_nodes
    UNION ALL
    SELECT menu_key, 'READ'
    FROM menu_nodes
) seed
WHERE EXISTS (
    SELECT 1 FROM function_codes fc WHERE fc.function_code = seed.function_code
)
ON CONFLICT (menu_key, function_code) DO UPDATE
SET enabled = TRUE,
    updated_at = NOW();

INSERT INTO role_data_scopes (role_code, module_code, data_scope)
VALUES
    ('SUPER_ADMIN', '*', 'ALL'),
    ('TENANT_ADMIN', '*', 'ALL'),
    ('TAX_EXPERT', '*', 'ASSIGNED'),
    ('TAX_REVIEWER', '*', 'ASSIGNED'),
    ('ASSISTANT', '*', 'ASSIGNED'),
    ('VIEWER', '*', 'ASSIGNED')
ON CONFLICT (role_code, module_code) DO UPDATE
SET data_scope = EXCLUDED.data_scope,
    updated_at = NOW();

INSERT INTO field_masking_policies (module_code, field_path, mask_type)
VALUES
    ('customers', 'biz_reg_no', 'PARTIAL'),
    ('customers', 'corp_reg_no', 'PARTIAL'),
    ('users', 'email', 'PARTIAL'),
    ('users', 'phone', 'PARTIAL')
ON CONFLICT (module_code, field_path) DO UPDATE
SET mask_type = EXCLUDED.mask_type,
    active = TRUE;

INSERT INTO role_permissions (role_code, module_code, function_code, effect)
VALUES
    ('SUPER_ADMIN', '*', '*', 'ALLOW'),
    ('TENANT_ADMIN', 'customers', 'MASK_OFF', 'ALLOW'),
    ('TENANT_ADMIN', 'permissions', 'DELEGATE', 'ALLOW'),
    ('TAX_EXPERT', 'adjustment', 'CALCULATE', 'ALLOW'),
    ('TAX_EXPERT', 'forms', 'PRINT', 'ALLOW'),
    ('TAX_REVIEWER', 'workflow', 'APPROVE', 'ALLOW'),
    ('ASSISTANT', 'customers', 'MASK_OFF', 'DENY'),
    ('VIEWER', 'customers', 'MASK_OFF', 'DENY')
ON CONFLICT (role_code, module_code, function_code) DO UPDATE
SET effect = EXCLUDED.effect,
    updated_at = NOW();
