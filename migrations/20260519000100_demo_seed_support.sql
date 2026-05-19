UPDATE users u
SET password_hash = crypt('ChangeMe123!', gen_salt('bf')),
    use_2fa = FALSE,
    pwd_fail_count = 0,
    locked = FALSE,
    status = 'ACTIVE',
    pwd_changed_at = NOW()
FROM tenants t
WHERE t.tenant_id = u.tenant_id
  AND t.tenant_code = 'demo'
  AND u.login_id = 'admin';

INSERT INTO form_versions (form_code, form_name, version_no, effective_from, effective_to, template_json, status)
VALUES
    ('FORM32', 'Reserve rollforward statement', '2026.1', DATE '2026-01-01', NULL, '{"fields":["taxable_income","addbacks","deductions","reserve_basis"]}', 'APPROVED'),
    ('FORM50', 'E-filing summary statement', '2026.1', DATE '2026-01-01', NULL, '{"fields":["taxable_income","corporate_tax","local_income_tax","total_tax_due","efile_ready"]}', 'APPROVED'),
    ('ATT01', 'Financial statement attachment', '2026.1', DATE '2026-01-01', NULL, '{"fields":["attachment_code","taxable_income","total_tax_due","amount"]}', 'APPROVED'),
    ('ATT02', 'Asset register attachment', '2026.1', DATE '2026-01-01', NULL, '{"fields":["attachment_code","taxable_income","total_tax_due","amount"]}', 'APPROVED'),
    ('ATT03', 'Transaction detail attachment', '2026.1', DATE '2026-01-01', NULL, '{"fields":["attachment_code","taxable_income","total_tax_due","amount"]}', 'APPROVED'),
    ('ATT04', 'Vehicle usage attachment', '2026.1', DATE '2026-01-01', NULL, '{"fields":["attachment_code","taxable_income","total_tax_due","amount"]}', 'APPROVED'),
    ('ATT05', 'Workflow approval attachment', '2026.1', DATE '2026-01-01', NULL, '{"fields":["attachment_code","taxable_income","total_tax_due","amount"]}', 'APPROVED'),
    ('ATT06', 'Validation result attachment', '2026.1', DATE '2026-01-01', NULL, '{"fields":["attachment_code","taxable_income","total_tax_due","amount"]}', 'APPROVED'),
    ('ATT07', 'Tax credit attachment', '2026.1', DATE '2026-01-01', NULL, '{"fields":["attachment_code","taxable_income","total_tax_due","amount"]}', 'APPROVED'),
    ('ATT08', 'Loss carryforward attachment', '2026.1', DATE '2026-01-01', NULL, '{"fields":["attachment_code","taxable_income","total_tax_due","amount"]}', 'APPROVED'),
    ('ATT09', 'Foreign income attachment', '2026.1', DATE '2026-01-01', NULL, '{"fields":["attachment_code","taxable_income","total_tax_due","amount"]}', 'APPROVED'),
    ('ATT10', 'Consolidated tax attachment', '2026.1', DATE '2026-01-01', NULL, '{"fields":["attachment_code","taxable_income","total_tax_due","amount"]}', 'APPROVED')
ON CONFLICT (form_code, version_no) DO UPDATE
SET form_name = EXCLUDED.form_name,
    template_json = EXCLUDED.template_json,
    status = EXCLUDED.status;

INSERT INTO tax_forms (form_code, form_name, form_group, description)
SELECT DISTINCT form_code, form_name, 'CIT', 'seeded attachment form'
FROM form_versions
WHERE form_code IN (
    'FORM32', 'FORM50',
    'ATT01', 'ATT02', 'ATT03', 'ATT04', 'ATT05',
    'ATT06', 'ATT07', 'ATT08', 'ATT09', 'ATT10'
)
ON CONFLICT (form_code) DO UPDATE
SET form_name = EXCLUDED.form_name,
    active = TRUE,
    updated_at = NOW();

INSERT INTO form_templates (form_version_id, template_type, template_json, checksum)
SELECT form_version_id, 'JSON', template_json, md5(template_json::TEXT)
FROM form_versions
WHERE form_code IN (
    'FORM32', 'FORM50',
    'ATT01', 'ATT02', 'ATT03', 'ATT04', 'ATT05',
    'ATT06', 'ATT07', 'ATT08', 'ATT09', 'ATT10'
)
ON CONFLICT (form_version_id, template_type) DO UPDATE
SET template_json = EXCLUDED.template_json,
    checksum = EXCLUDED.checksum;

INSERT INTO form_validations (form_version_id, field_path, rule_code, severity, message, rule_json)
SELECT form_version_id, field_name, 'REQUIRED', 'ERROR', field_name || ' is required', '{"required":true}'::jsonb
FROM form_versions
CROSS JOIN LATERAL jsonb_array_elements_text(template_json->'fields') AS fields(field_name)
WHERE form_code IN (
    'FORM32', 'FORM50',
    'ATT01', 'ATT02', 'ATT03', 'ATT04', 'ATT05',
    'ATT06', 'ATT07', 'ATT08', 'ATT09', 'ATT10'
)
ON CONFLICT DO NOTHING;
