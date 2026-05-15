DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM tenants
        WHERE EXISTS (
            SELECT 1
            FROM information_schema.schemata
            WHERE schema_name = tenants.schema_name
        )
    LOOP
        EXECUTE format($sql$
            CREATE TABLE IF NOT EXISTS %I.valuation_positions (
                valuation_position_id BIGSERIAL PRIMARY KEY,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                module_code     VARCHAR(20) NOT NULL,
                item_code       VARCHAR(80) NOT NULL,
                item_name       VARCHAR(200) NOT NULL,
                position_type   VARCHAR(40) NOT NULL DEFAULT 'GENERAL',
                monetary        BOOLEAN NOT NULL DEFAULT TRUE,
                valuation_method VARCHAR(40) NOT NULL DEFAULT 'CLOSING_RATE',
                book_amount     BIGINT NOT NULL,
                tax_amount      BIGINT NOT NULL,
                adjustment_amount BIGINT NOT NULL,
                metadata        JSONB NOT NULL DEFAULT '{}'::jsonb,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $sql$, tenant_schema, tenant_schema);

        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.valuation_positions(by_id, module_code, item_code)',
            'idx_' || tenant_schema || '_valuation_positions_by',
            tenant_schema
        );

        EXECUTE format(
            'ALTER TABLE %I.carryforward_loss ADD COLUMN IF NOT EXISTS used_amount BIGINT NOT NULL DEFAULT 0',
            tenant_schema
        );
        EXECUTE format(
            'ALTER TABLE %I.carryforward_loss ADD COLUMN IF NOT EXISTS expired_amount BIGINT NOT NULL DEFAULT 0',
            tenant_schema
        );
        EXECUTE format(
            'ALTER TABLE %I.carryforward_loss ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()',
            tenant_schema
        );

        EXECUTE format($sql$
            CREATE TABLE IF NOT EXISTS %I.capital_changes (
                capital_change_id BIGSERIAL PRIMARY KEY,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                change_date     DATE NOT NULL,
                change_type     VARCHAR(40) NOT NULL,
                amount          BIGINT NOT NULL,
                description     TEXT,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $sql$, tenant_schema, tenant_schema);

        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.capital_changes(by_id, change_date, capital_change_id)',
            'idx_' || tenant_schema || '_capital_changes_by',
            tenant_schema
        );
    END LOOP;
END $$;

INSERT INTO tax_limits (law_version_id, item_code, amount, effective_from, effective_to, metadata)
SELECT law.law_version_id, seed.item_code, seed.amount, law.effective_from, law.effective_to, seed.metadata
FROM tax_law_versions law
CROSS JOIN (
    VALUES
        ('LOSS_DEDUCTION_LIMIT_BPS_SME', 10000::BIGINT, '{"category":"LOSS_RULE","description":"SME loss deduction limit rate in bps"}'::jsonb),
        ('LOSS_DEDUCTION_LIMIT_BPS_GENERAL', 8000::BIGINT, '{"category":"LOSS_RULE","description":"general company loss deduction limit rate in bps"}'::jsonb)
) AS seed(item_code, amount, metadata)
WHERE law.version_code LIKE 'CIT-%'
  AND NOT EXISTS (
      SELECT 1
      FROM tax_limits existing
      WHERE existing.law_version_id = law.law_version_id
        AND existing.item_code = seed.item_code
        AND existing.effective_from = law.effective_from
  );
