INSERT INTO validation_rules (
    rule_code, severity, area, message_template, applies_to, active
) VALUES
    (
        'CHK_STDFS_CONFIRMED',
        'ERROR',
        'std-fs',
        'Standard financial statements must be CONFIRMED before entering adjustment step. Confirmed: {std_fs_confirmed}.',
        '#/workspace/ws/info/consistency',
        TRUE
    )
ON CONFLICT (rule_code) DO UPDATE
SET severity = EXCLUDED.severity,
    area = EXCLUDED.area,
    message_template = EXCLUDED.message_template,
    applies_to = EXCLUDED.applies_to,
    active = EXCLUDED.active;
