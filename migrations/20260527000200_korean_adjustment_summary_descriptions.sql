DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM tenants
        WHERE to_regnamespace(schema_name) IS NOT NULL
          AND to_regclass(format('%I.tax_adjustments', schema_name)) IS NOT NULL
    LOOP
        EXECUTE format($sql$
            UPDATE %I.tax_adjustments
            SET description = CASE description
                WHEN 'Transaction based tax adjustment' THEN '거래 기반 세무조정'
                WHEN 'Evaluation and carryforward adjustment' THEN '평가 및 이월 세무조정'
                WHEN 'Tax amount adjustment' THEN '세액 조정'
                WHEN 'Special tax adjustment' THEN '특수 세무조정'
                ELSE description
            END
            WHERE description IN (
                'Transaction based tax adjustment',
                'Evaluation and carryforward adjustment',
                'Tax amount adjustment',
                'Special tax adjustment'
            );
        $sql$, tenant_schema);
    END LOOP;
END $$;
