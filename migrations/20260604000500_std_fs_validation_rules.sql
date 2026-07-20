INSERT INTO validation_rules (
    rule_code, severity, area, message_template, applies_to, active
) VALUES
    (
        'CHK_STDBS_BALANCE',
        'ERROR',
        'std-fs',
        'STD_BS assets must equal liabilities plus equity. Difference: {std_bs_balance_diff}.',
        '#/workspace/ws/info/std-fs',
        TRUE
    ),
    (
        'CHK_STDBS_VS_FS',
        'ERROR',
        'std-fs',
        'STD_BS asset total must equal source BS asset total. Difference: {std_bs_vs_fs_diff}.',
        '#/workspace/ws/info/std-fs',
        TRUE
    ),
    (
        'CHK_STDIS_VS_FS',
        'ERROR',
        'std-fs',
        'STD_IS profit/loss must equal source IS profit/loss. Difference: {std_is_vs_fs_diff}.',
        '#/workspace/ws/info/std-fs',
        TRUE
    ),
    (
        'CHK_STDFS_UNMAPPED',
        'ERROR',
        'std-fs',
        'All source FS lines must be mapped to active leaf standard FS items. Count: {std_fs_unmapped_count}.',
        '#/workspace/ws/info/std-fs',
        TRUE
    )
ON CONFLICT (rule_code) DO UPDATE
SET severity = EXCLUDED.severity,
    area = EXCLUDED.area,
    message_template = EXCLUDED.message_template,
    applies_to = EXCLUDED.applies_to,
    active = EXCLUDED.active;
