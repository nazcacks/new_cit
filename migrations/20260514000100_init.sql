CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS tenants (
    tenant_id       BIGSERIAL PRIMARY KEY,
    tenant_code     VARCHAR(20) UNIQUE NOT NULL CHECK (tenant_code ~ '^[a-z][a-z0-9_]*$'),
    tenant_name     VARCHAR(200) NOT NULL,
    biz_reg_no      VARCHAR(13) NOT NULL,
    contract_start  DATE NOT NULL,
    contract_end    DATE,
    schema_name     VARCHAR(50) UNIQUE NOT NULL CHECK (schema_name ~ '^tenant_[a-z0-9_]+$'),
    status          VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    allowed_ips     TEXT,
    max_users       INT NOT NULL DEFAULT 10,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS users (
    user_id         BIGSERIAL PRIMARY KEY,
    tenant_id       BIGINT NOT NULL REFERENCES tenants(tenant_id),
    login_id        VARCHAR(50) NOT NULL,
    password_hash   VARCHAR(255) NOT NULL,
    user_name       VARCHAR(100) NOT NULL,
    email           VARCHAR(200),
    phone           VARCHAR(20),
    totp_secret     VARCHAR(255),
    use_2fa         BOOLEAN NOT NULL DEFAULT TRUE,
    pwd_changed_at  TIMESTAMPTZ,
    pwd_fail_count  INT NOT NULL DEFAULT 0,
    locked          BOOLEAN NOT NULL DEFAULT FALSE,
    last_login_at   TIMESTAMPTZ,
    last_login_ip   VARCHAR(45),
    status          VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, login_id)
);

CREATE TABLE IF NOT EXISTS login_history (
    history_id      BIGSERIAL PRIMARY KEY,
    user_id         BIGINT REFERENCES users(user_id),
    login_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address      VARCHAR(45),
    user_agent      VARCHAR(500),
    success         BOOLEAN NOT NULL,
    fail_reason     VARCHAR(200),
    session_id      VARCHAR(100)
);
CREATE INDEX IF NOT EXISTS idx_login_history_user ON login_history(user_id, login_at DESC);

CREATE TABLE IF NOT EXISTS tax_law_versions (
    law_version_id  BIGSERIAL PRIMARY KEY,
    version_code    VARCHAR(50) UNIQUE NOT NULL,
    law_name        VARCHAR(200) NOT NULL,
    effective_from  DATE NOT NULL,
    effective_to    DATE,
    status          VARCHAR(20) NOT NULL DEFAULT 'APPROVED',
    metadata        JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tax_rates (
    tax_rate_id            BIGSERIAL PRIMARY KEY,
    law_version_id         BIGINT NOT NULL REFERENCES tax_law_versions(law_version_id),
    item_code              VARCHAR(50) NOT NULL,
    taxable_from           BIGINT NOT NULL,
    taxable_to             BIGINT,
    base_tax               BIGINT NOT NULL DEFAULT 0,
    rate_bps               INT NOT NULL CHECK (rate_bps >= 0),
    progressive_deduction  BIGINT NOT NULL DEFAULT 0,
    effective_from         DATE NOT NULL,
    effective_to           DATE,
    metadata               JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE(law_version_id, item_code, taxable_from)
);
CREATE INDEX IF NOT EXISTS idx_tax_rates_effective ON tax_rates(item_code, effective_from, effective_to);

CREATE TABLE IF NOT EXISTS tax_limits (
    tax_limit_id    BIGSERIAL PRIMARY KEY,
    law_version_id  BIGINT NOT NULL REFERENCES tax_law_versions(law_version_id),
    item_code       VARCHAR(50) NOT NULL,
    amount          BIGINT NOT NULL,
    effective_from  DATE NOT NULL,
    effective_to    DATE,
    metadata        JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE IF NOT EXISTS form_versions (
    form_version_id BIGSERIAL PRIMARY KEY,
    form_code       VARCHAR(50) NOT NULL,
    form_name       VARCHAR(200) NOT NULL,
    version_no      VARCHAR(30) NOT NULL,
    effective_from  DATE NOT NULL,
    effective_to    DATE,
    template_json   JSONB NOT NULL DEFAULT '{}'::jsonb,
    status          VARCHAR(20) NOT NULL DEFAULT 'APPROVED',
    UNIQUE(form_code, version_no)
);

CREATE TABLE IF NOT EXISTS form_relationships (
    relationship_id BIGSERIAL PRIMARY KEY,
    source_form     VARCHAR(50) NOT NULL,
    source_field    VARCHAR(100) NOT NULL,
    target_form     VARCHAR(50) NOT NULL,
    target_field    VARCHAR(100) NOT NULL,
    rule_json       JSONB NOT NULL DEFAULT '{}'::jsonb,
    effective_from  DATE NOT NULL,
    effective_to    DATE
);

CREATE TABLE IF NOT EXISTS efile_masters (
    efile_master_id BIGSERIAL PRIMARY KEY,
    master_code     VARCHAR(50) UNIQUE NOT NULL,
    master_name     VARCHAR(200) NOT NULL,
    version_no      VARCHAR(30) NOT NULL,
    encoding        VARCHAR(30) NOT NULL DEFAULT 'windows-949',
    effective_from  DATE NOT NULL,
    effective_to    DATE,
    status          VARCHAR(20) NOT NULL DEFAULT 'APPROVED',
    metadata        JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE IF NOT EXISTS efile_record_layouts (
    layout_id        BIGSERIAL PRIMARY KEY,
    efile_master_id  BIGINT NOT NULL REFERENCES efile_masters(efile_master_id),
    record_type      VARCHAR(10) NOT NULL,
    record_name      VARCHAR(100) NOT NULL,
    sort_order       INT NOT NULL,
    fixed_length     INT NOT NULL
);

CREATE TABLE IF NOT EXISTS efile_record_fields (
    field_id         BIGSERIAL PRIMARY KEY,
    layout_id        BIGINT NOT NULL REFERENCES efile_record_layouts(layout_id),
    field_name       VARCHAR(100) NOT NULL,
    start_pos        INT NOT NULL,
    byte_length      INT NOT NULL,
    align            VARCHAR(10) NOT NULL DEFAULT 'LEFT',
    pad_char         VARCHAR(1) NOT NULL DEFAULT ' ',
    required         BOOLEAN NOT NULL DEFAULT TRUE,
    source_path      VARCHAR(200)
);

CREATE TABLE IF NOT EXISTS law_amendment_history (
    amendment_id    BIGSERIAL PRIMARY KEY,
    law_version_id  BIGINT NOT NULL REFERENCES tax_law_versions(law_version_id),
    change_summary  TEXT NOT NULL,
    approved_by     VARCHAR(100) NOT NULL DEFAULT 'system',
    approved_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS jobs (
    job_id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_type         VARCHAR(80) NOT NULL,
    payload          JSONB NOT NULL,
    status           VARCHAR(30) NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'succeeded', 'failed', 'dead_letter')),
    attempts         INT NOT NULL DEFAULT 0,
    max_attempts     INT NOT NULL DEFAULT 3,
    next_run_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locked_at        TIMESTAMPTZ,
    last_error       TEXT,
    result           JSONB,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at     TIMESTAMPTZ,
    dead_lettered_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_jobs_ready ON jobs(status, next_run_at, created_at);
CREATE INDEX IF NOT EXISTS idx_jobs_type ON jobs(job_type, status);

INSERT INTO tax_law_versions (version_code, law_name, effective_from, effective_to, status, metadata)
VALUES
    ('CIT-2024', 'Corporate Income Tax Act 2024', DATE '2024-01-01', DATE '2024-12-31', 'APPROVED', '{"source":"seed"}'),
    ('CIT-2025', 'Corporate Income Tax Act 2025', DATE '2025-01-01', DATE '2025-12-31', 'APPROVED', '{"source":"seed"}'),
    ('CIT-2026', 'Corporate Income Tax Act 2026', DATE '2026-01-01', NULL, 'APPROVED', '{"source":"seed"}')
ON CONFLICT (version_code) DO NOTHING;

INSERT INTO tax_rates (
    law_version_id,
    item_code,
    taxable_from,
    taxable_to,
    base_tax,
    rate_bps,
    progressive_deduction,
    effective_from,
    effective_to,
    metadata
)
SELECT
    tax_law_versions.law_version_id,
    seed.item_code,
    seed.taxable_from,
    seed.taxable_to,
    seed.base_tax,
    seed.rate_bps,
    seed.progressive_deduction,
    seed.effective_from,
    seed.effective_to,
    seed.metadata
FROM (
    VALUES
        ('CIT-2024', 'CORPORATE_TAX', 0::BIGINT, 200000000::BIGINT, 0::BIGINT, 900, 0::BIGINT, DATE '2024-01-01', DATE '2024-12-31', '{"bracket":"0-200M"}'::jsonb),
        ('CIT-2024', 'CORPORATE_TAX', 200000001::BIGINT, 20000000000::BIGINT, 0::BIGINT, 1900, 20000000::BIGINT, DATE '2024-01-01', DATE '2024-12-31', '{"bracket":"200M-20B"}'::jsonb),
        ('CIT-2024', 'CORPORATE_TAX', 20000000001::BIGINT, 300000000000::BIGINT, 0::BIGINT, 2100, 420000000::BIGINT, DATE '2024-01-01', DATE '2024-12-31', '{"bracket":"20B-300B"}'::jsonb),
        ('CIT-2024', 'CORPORATE_TAX', 300000000001::BIGINT, NULL::BIGINT, 0::BIGINT, 2400, 9420000000::BIGINT, DATE '2024-01-01', DATE '2024-12-31', '{"bracket":"300B+"}'::jsonb),
        ('CIT-2025', 'CORPORATE_TAX', 0::BIGINT, 200000000::BIGINT, 0::BIGINT, 900, 0::BIGINT, DATE '2025-01-01', DATE '2025-12-31', '{"bracket":"0-200M"}'::jsonb),
        ('CIT-2025', 'CORPORATE_TAX', 200000001::BIGINT, 20000000000::BIGINT, 0::BIGINT, 1900, 20000000::BIGINT, DATE '2025-01-01', DATE '2025-12-31', '{"bracket":"200M-20B"}'::jsonb),
        ('CIT-2025', 'CORPORATE_TAX', 20000000001::BIGINT, 300000000000::BIGINT, 0::BIGINT, 2100, 420000000::BIGINT, DATE '2025-01-01', DATE '2025-12-31', '{"bracket":"20B-300B"}'::jsonb),
        ('CIT-2025', 'CORPORATE_TAX', 300000000001::BIGINT, NULL::BIGINT, 0::BIGINT, 2400, 9420000000::BIGINT, DATE '2025-01-01', DATE '2025-12-31', '{"bracket":"300B+"}'::jsonb),
        ('CIT-2026', 'CORPORATE_TAX', 0::BIGINT, 200000000::BIGINT, 0::BIGINT, 900, 0::BIGINT, DATE '2026-01-01', NULL::DATE, '{"bracket":"0-200M"}'::jsonb),
        ('CIT-2026', 'CORPORATE_TAX', 200000001::BIGINT, 20000000000::BIGINT, 0::BIGINT, 1900, 20000000::BIGINT, DATE '2026-01-01', NULL::DATE, '{"bracket":"200M-20B"}'::jsonb),
        ('CIT-2026', 'CORPORATE_TAX', 20000000001::BIGINT, 300000000000::BIGINT, 0::BIGINT, 2100, 420000000::BIGINT, DATE '2026-01-01', NULL::DATE, '{"bracket":"20B-300B"}'::jsonb),
        ('CIT-2026', 'CORPORATE_TAX', 300000000001::BIGINT, NULL::BIGINT, 0::BIGINT, 2400, 9420000000::BIGINT, DATE '2026-01-01', NULL::DATE, '{"bracket":"300B+"}'::jsonb)
) AS seed(version_code, item_code, taxable_from, taxable_to, base_tax, rate_bps, progressive_deduction, effective_from, effective_to, metadata)
JOIN tax_law_versions USING (version_code)
ON CONFLICT (law_version_id, item_code, taxable_from) DO NOTHING;

INSERT INTO tax_limits (law_version_id, item_code, amount, effective_from, effective_to, metadata)
SELECT
    tax_law_versions.law_version_id,
    seed.item_code,
    seed.amount,
    seed.effective_from,
    seed.effective_to,
    seed.metadata
FROM (
    VALUES
        ('CIT-2026', 'ENTERTAINMENT_BASE_LIMIT', 12000000::BIGINT, DATE '2026-01-01', NULL::DATE, '{"description":"base entertainment expense limit"}'::jsonb),
        ('CIT-2026', 'DONATION_LIMIT_BPS', 1000::BIGINT, DATE '2026-01-01', NULL::DATE, '{"description":"10 percent of pre-donation income"}'::jsonb)
) AS seed(version_code, item_code, amount, effective_from, effective_to, metadata)
JOIN tax_law_versions USING (version_code);

INSERT INTO form_versions (form_code, form_name, version_no, effective_from, effective_to, template_json, status)
VALUES
    ('FORM3', '법인세 과세표준 및 세액조정계산서', '2026.1', DATE '2026-01-01', NULL, '{"fields":["taxable_income","corporate_tax","local_income_tax","tax_credits","total_tax_due"]}', 'APPROVED'),
    ('FORM15', '소득금액조정명세서', '2026.1', DATE '2026-01-01', NULL, '{"fields":["accounting_income","addbacks","deductions","taxable_income"]}', 'APPROVED'),
    ('FORM22', '기부금 조정명세서', '2026.1', DATE '2026-01-01', NULL, '{"fields":["donations","deductible_donations","non_deductible_donations"]}', 'APPROVED')
ON CONFLICT (form_code, version_no) DO NOTHING;

INSERT INTO form_relationships (source_form, source_field, target_form, target_field, rule_json, effective_from, effective_to)
VALUES
    ('FORM15', 'taxable_income', 'FORM3', 'taxable_income', '{"operation":"copy_latest"}', DATE '2026-01-01', NULL),
    ('FORM22', 'non_deductible_donations', 'FORM15', 'addbacks', '{"operation":"add"}', DATE '2026-01-01', NULL);

INSERT INTO efile_masters (master_code, master_name, version_no, encoding, effective_from, effective_to, status, metadata)
VALUES
    ('CIT-EFILE-2026', '홈택스 법인세 전자신고 2026', '2026.1', 'windows-949', DATE '2026-01-01', NULL, 'APPROVED', '{"record_end":"CRLF"}')
ON CONFLICT (master_code) DO NOTHING;

INSERT INTO efile_record_layouts (efile_master_id, record_type, record_name, sort_order, fixed_length)
SELECT efile_master_id, record_type, record_name, sort_order, fixed_length
FROM efile_masters
CROSS JOIN (
    VALUES
        ('H', 'Header', 1, 80),
        ('D', 'Detail', 2, 80),
        ('T', 'Trailer', 3, 80)
) AS layout(record_type, record_name, sort_order, fixed_length)
WHERE master_code = 'CIT-EFILE-2026'
ON CONFLICT DO NOTHING;
