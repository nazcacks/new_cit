CREATE TABLE IF NOT EXISTS tax_forms (
    form_id      BIGSERIAL PRIMARY KEY,
    form_code    VARCHAR(50) UNIQUE NOT NULL,
    form_name    VARCHAR(200) NOT NULL,
    form_group   VARCHAR(100),
    description  TEXT,
    active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS form_templates (
    template_id     BIGSERIAL PRIMARY KEY,
    form_version_id BIGINT NOT NULL REFERENCES form_versions(form_version_id) ON DELETE CASCADE,
    template_type   VARCHAR(30) NOT NULL DEFAULT 'JSON',
    template_json   JSONB NOT NULL DEFAULT '{}'::jsonb,
    checksum        VARCHAR(80) NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(form_version_id, template_type)
);

CREATE TABLE IF NOT EXISTS form_validations (
    validation_id   BIGSERIAL PRIMARY KEY,
    form_version_id BIGINT NOT NULL REFERENCES form_versions(form_version_id) ON DELETE CASCADE,
    field_path      VARCHAR(200) NOT NULL,
    rule_code       VARCHAR(80) NOT NULL,
    severity        VARCHAR(20) NOT NULL DEFAULT 'ERROR',
    message         TEXT NOT NULL,
    rule_json       JSONB NOT NULL DEFAULT '{}'::jsonb,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS form_field_migration (
    migration_rule_id BIGSERIAL PRIMARY KEY,
    form_code         VARCHAR(50) NOT NULL,
    from_version_no   VARCHAR(30) NOT NULL,
    to_version_no     VARCHAR(30) NOT NULL,
    source_field      VARCHAR(100) NOT NULL,
    target_field      VARCHAR(100),
    operation         VARCHAR(30) NOT NULL DEFAULT 'COPY',
    rule_json         JSONB NOT NULL DEFAULT '{}'::jsonb,
    active            BOOLEAN NOT NULL DEFAULT TRUE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO tax_forms (form_code, form_name, form_group, description)
SELECT DISTINCT form_code, form_name, 'CIT', 'seeded from form_versions'
FROM form_versions
ON CONFLICT (form_code) DO UPDATE
SET form_name = EXCLUDED.form_name,
    updated_at = NOW();

INSERT INTO form_templates (form_version_id, template_type, template_json, checksum)
SELECT form_version_id, 'JSON', template_json, md5(template_json::TEXT)
FROM form_versions
ON CONFLICT (form_version_id, template_type) DO UPDATE
SET template_json = EXCLUDED.template_json,
    checksum = EXCLUDED.checksum;

INSERT INTO form_validations (form_version_id, field_path, rule_code, severity, message, rule_json)
SELECT form_version_id, field_name, 'REQUIRED', 'ERROR', field_name || ' is required', '{"required":true}'::jsonb
FROM form_versions
CROSS JOIN LATERAL jsonb_array_elements_text(template_json->'fields') AS fields(field_name)
ON CONFLICT DO NOTHING;

INSERT INTO form_field_migration (form_code, from_version_no, to_version_no, source_field, target_field, operation, rule_json)
VALUES
    ('FORM3', '2025.1', '2026.1', 'taxable_income', 'taxable_income', 'COPY', '{"description":"carry forward taxable income"}'),
    ('FORM15', '2025.1', '2026.1', 'accounting_income', 'accounting_income', 'COPY', '{"description":"carry forward accounting income"}'),
    ('FORM22', '2025.1', '2026.1', 'donations', 'donations', 'COPY', '{"description":"carry forward donation amount"}')
ON CONFLICT DO NOTHING;
