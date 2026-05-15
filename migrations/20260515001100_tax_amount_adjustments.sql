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
            CREATE TABLE IF NOT EXISTS %I.tax_credit_claims (
                credit_claim_id BIGSERIAL PRIMARY KEY,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                credit_type     VARCHAR(50) NOT NULL,
                base_amount     BIGINT NOT NULL,
                rate_bps        BIGINT NOT NULL,
                requested_amount BIGINT NOT NULL,
                allowed_amount  BIGINT NOT NULL,
                metadata        JSONB NOT NULL DEFAULT '{}'::jsonb,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $sql$, tenant_schema, tenant_schema);
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.tax_credit_claims(by_id, credit_type)',
            'idx_' || tenant_schema || '_tax_credit_claims_by',
            tenant_schema
        );

        EXECUTE format($sql$
            CREATE TABLE IF NOT EXISTS %I.minimum_tax_results (
                minimum_tax_result_id BIGSERIAL PRIMARY KEY,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                tax_base        BIGINT NOT NULL,
                regular_tax     BIGINT NOT NULL,
                minimum_tax     BIGINT NOT NULL,
                additional_tax  BIGINT NOT NULL,
                metadata        JSONB NOT NULL DEFAULT '{}'::jsonb,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $sql$, tenant_schema, tenant_schema);
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.minimum_tax_results(by_id, created_at DESC)',
            'idx_' || tenant_schema || '_minimum_tax_by',
            tenant_schema
        );

        EXECUTE format($sql$
            CREATE TABLE IF NOT EXISTS %I.penalty_tax_items (
                penalty_tax_item_id BIGSERIAL PRIMARY KEY,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                penalty_type    VARCHAR(50) NOT NULL,
                tax_base        BIGINT NOT NULL,
                rate_bps        BIGINT NOT NULL,
                days_late       INT,
                reduction_bps   BIGINT NOT NULL DEFAULT 0,
                penalty_amount  BIGINT NOT NULL,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $sql$, tenant_schema, tenant_schema);
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.penalty_tax_items(by_id, penalty_type)',
            'idx_' || tenant_schema || '_penalty_tax_by',
            tenant_schema
        );
    END LOOP;
END $$;

INSERT INTO tax_limits (law_version_id, item_code, amount, effective_from, effective_to, metadata)
SELECT law.law_version_id, seed.item_code, seed.amount, law.effective_from, law.effective_to, seed.metadata
FROM tax_law_versions law
CROSS JOIN (
    VALUES
        ('INTEGRATED_INVESTMENT_CREDIT_BPS', 1000::BIGINT, '{"category":"CREDIT","description":"integrated investment credit rate in bps"}'::jsonb),
        ('FOREIGN_TAX_CREDIT_LIMIT_BPS', 10000::BIGINT, '{"category":"CREDIT","description":"foreign tax credit maximum bps"}'::jsonb),
        ('DISASTER_CREDIT_BPS', 3000::BIGINT, '{"category":"CREDIT","description":"disaster tax credit rate in bps"}'::jsonb),
        ('SME_SPECIAL_REDUCTION_BPS', 1500::BIGINT, '{"category":"CREDIT","description":"SME special reduction rate in bps"}'::jsonb),
        ('STARTUP_REDUCTION_BPS', 5000::BIGINT, '{"category":"CREDIT","description":"startup reduction rate in bps"}'::jsonb),
        ('MINIMUM_TAX_RATE_BPS_SME', 1000::BIGINT, '{"category":"MINIMUM_TAX","description":"SME minimum tax rate in bps"}'::jsonb),
        ('MINIMUM_TAX_RATE_BPS_GENERAL', 1700::BIGINT, '{"category":"MINIMUM_TAX","description":"general minimum tax rate in bps"}'::jsonb)
) AS seed(item_code, amount, metadata)
WHERE law.version_code LIKE 'CIT-%'
  AND NOT EXISTS (
      SELECT 1
      FROM tax_limits existing
      WHERE existing.law_version_id = law.law_version_id
        AND existing.item_code = seed.item_code
        AND existing.effective_from = law.effective_from
  );
