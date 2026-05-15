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
            CREATE TABLE IF NOT EXISTS %I.donation_carryforwards (
                carryforward_id BIGSERIAL PRIMARY KEY,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                source_year     INT NOT NULL,
                donation_type   VARCHAR(30) NOT NULL,
                original_amount BIGINT NOT NULL,
                used_amount     BIGINT NOT NULL DEFAULT 0,
                expired_amount  BIGINT NOT NULL DEFAULT 0,
                remaining_amount BIGINT NOT NULL,
                expires_year    INT NOT NULL,
                adjustment_item_id BIGINT,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $sql$, tenant_schema, tenant_schema);

        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.donation_carryforwards(by_id, donation_type, expires_year)',
            'idx_' || tenant_schema || '_donation_carryforwards_by',
            tenant_schema
        );

        EXECUTE format($sql$
            CREATE TABLE IF NOT EXISTS %I.entertainment_revenue_breakdowns (
                revenue_breakdown_id BIGSERIAL PRIMARY KEY,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                revenue_category VARCHAR(80) NOT NULL,
                amount          BIGINT NOT NULL,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $sql$, tenant_schema, tenant_schema);

        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.entertainment_revenue_breakdowns(by_id, revenue_category)',
            'idx_' || tenant_schema || '_ent_revenue_by',
            tenant_schema
        );

        EXECUTE format($sql$
            CREATE TABLE IF NOT EXISTS %I.loan_interest_facts (
                loan_interest_fact_id BIGSERIAL PRIMARY KEY,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                weighted_average_loan_balance BIGINT NOT NULL DEFAULT 0,
                weighted_average_interest_rate_bps INT NOT NULL DEFAULT 0,
                deemed_interest BIGINT NOT NULL DEFAULT 0,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $sql$, tenant_schema, tenant_schema);

        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.loan_interest_facts(by_id, created_at DESC)',
            'idx_' || tenant_schema || '_loan_interest_by',
            tenant_schema
        );
    END LOOP;
END $$;

INSERT INTO tax_limits (law_version_id, item_code, amount, effective_from, effective_to, metadata)
SELECT law.law_version_id, seed.item_code, seed.amount, law.effective_from, law.effective_to, seed.metadata
FROM tax_law_versions law
CROSS JOIN (
    VALUES
        ('DONATION_SPECIAL_LIMIT_BPS', 5000::BIGINT, '{"category":"DONATION","description":"special donation limit rate in bps"}'::jsonb),
        ('DONATION_GENERAL_LIMIT_BPS', 1000::BIGINT, '{"category":"DONATION","description":"general donation limit rate in bps"}'::jsonb),
        ('DONATION_CARRYFORWARD_YEARS', 10::BIGINT, '{"category":"DONATION","description":"donation carryforward years"}'::jsonb),
        ('ENTERTAINMENT_REVENUE_RATE_BPS', 30::BIGINT, '{"category":"ENTERTAINMENT","description":"revenue based entertainment limit rate in bps"}'::jsonb),
        ('ENTERTAINMENT_NO_CARD_DISALLOW_BPS', 10000::BIGINT, '{"category":"ENTERTAINMENT","description":"non-card entertainment disallowance rate in bps"}'::jsonb),
        ('INTEREST_DEEMED_RATE_BPS', 460::BIGINT, '{"category":"INTEREST","description":"default weighted average interest rate in bps"}'::jsonb)
) AS seed(item_code, amount, metadata)
WHERE law.version_code LIKE 'CIT-%'
  AND NOT EXISTS (
      SELECT 1
      FROM tax_limits existing
      WHERE existing.law_version_id = law.law_version_id
        AND existing.item_code = seed.item_code
        AND existing.effective_from = law.effective_from
  );
