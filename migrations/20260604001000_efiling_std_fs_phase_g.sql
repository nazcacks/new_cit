INSERT INTO efile_validation_rules (
    efile_master_id, rule_code, severity, field_path, message, rule_json
)
SELECT efile_master_id, rule_code, severity, field_path, message, rule_json
FROM efile_masters
CROSS JOIN (
    VALUES
        (
            'EFILE_STDFS_CONFIRMED',
            'ERROR',
            'std_fs_statements.status',
            'Standard financial statements must be CONFIRMED before e-filing XML generation.',
            '{"type":"std_fs_confirmed"}'::jsonb
        ),
        (
            'EFILE_STDFS_XML_FIELD',
            'ERROR',
            'std_fs_items.xml_field_id',
            'Confirmed standard BS/IS statement lines must have xml_field_id values for XML generation.',
            '{"type":"std_fs_xml_field"}'::jsonb
        ),
        (
            'EFILE_STDFS_TOTALS',
            'ERROR',
            'std_fs_statements.total_check',
            'Confirmed standard BS/IS totals must match current source financial statement totals before e-filing.',
            '{"type":"std_fs_totals"}'::jsonb
        )
) AS rule(rule_code, severity, field_path, message, rule_json)
WHERE master_code = 'CIT-EFILE-2026'
ON CONFLICT (efile_master_id, rule_code) DO UPDATE
SET severity = EXCLUDED.severity,
    field_path = EXCLUDED.field_path,
    message = EXCLUDED.message,
    rule_json = EXCLUDED.rule_json,
    active = TRUE;
