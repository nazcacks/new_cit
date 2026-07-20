INSERT INTO validation_rules (rule_code, severity, area, message_template, applies_to, active)
VALUES
    (
        'CHK_DONATION_TXN',
        'ERROR',
        'tax-data',
        'Donation transactions must match source IS and STD_IS totals. Difference: {donation_txn_is_diff}.',
        '#/workspace/ws/info/transactions',
        TRUE
    ),
    (
        'CHK_ENTERTAIN_TXN',
        'WARN',
        'tax-data',
        'Entertainment transactions should match source IS and STD_IS totals. Difference: {entertain_txn_is_diff}.',
        '#/workspace/ws/info/transactions',
        TRUE
    ),
    (
        'CHK_INTEREST_TXN',
        'ERROR',
        'tax-data',
        'Interest expense transactions must match source IS and STD_IS totals. Difference: {interest_txn_is_diff}.',
        '#/workspace/ws/info/transactions',
        TRUE
    ),
    (
        'CHK_VEHICLE_USAGE_BPS',
        'WARN',
        'tax-data',
        'Business vehicle mileage logs are missing for {vehicle_usage_default_count} vehicle(s); 70% business-use default is applied.',
        '#/workspace/ws/info/vehicle',
        TRUE
    ),
    (
        'CHK_B10_LINK',
        'ERROR',
        'tax-data',
        'B10 business vehicle adjustment must match mileage-based addback. Difference: {b10_link_diff}.',
        '#/workspace/ws/info/vehicle',
        TRUE
    )
ON CONFLICT (rule_code) DO UPDATE
SET severity = EXCLUDED.severity,
    area = EXCLUDED.area,
    message_template = EXCLUDED.message_template,
    applies_to = EXCLUDED.applies_to,
    active = EXCLUDED.active;
