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
            CREATE TABLE IF NOT EXISTS %I.foreign_income_items (
                foreign_income_item_id BIGSERIAL PRIMARY KEY,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                income_type     VARCHAR(50) NOT NULL,
                gross_amount    BIGINT NOT NULL,
                attributable_expense BIGINT NOT NULL DEFAULT 0,
                pe_allocation_bps BIGINT NOT NULL DEFAULT 10000,
                allocated_income BIGINT NOT NULL,
                withholding_tax BIGINT NOT NULL DEFAULT 0,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $sql$, tenant_schema, tenant_schema);
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.foreign_income_items(by_id, income_type)',
            'idx_' || tenant_schema || '_foreign_income_by',
            tenant_schema
        );

        EXECUTE format($sql$
            CREATE TABLE IF NOT EXISTS %I.consolidated_entities (
                consolidated_entity_id BIGSERIAL PRIMARY KEY,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                entity_code     VARCHAR(50) NOT NULL,
                entity_name     VARCHAR(200) NOT NULL,
                ownership_bps   BIGINT NOT NULL,
                taxable_income  BIGINT NOT NULL,
                standalone_tax  BIGINT NOT NULL DEFAULT 0,
                allocated_tax   BIGINT NOT NULL DEFAULT 0,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $sql$, tenant_schema, tenant_schema);
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.consolidated_entities(by_id, entity_code)',
            'idx_' || tenant_schema || '_consolidated_entities_by',
            tenant_schema
        );

        EXECUTE format($sql$
            CREATE TABLE IF NOT EXISTS %I.consolidation_eliminations (
                consolidation_elimination_id BIGSERIAL PRIMARY KEY,
                by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                elimination_type VARCHAR(50) NOT NULL,
                amount          BIGINT NOT NULL,
                direction       VARCHAR(20) NOT NULL,
                description     TEXT,
                created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        $sql$, tenant_schema, tenant_schema);
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.consolidation_eliminations(by_id, elimination_type)',
            'idx_' || tenant_schema || '_consolidation_elim_by',
            tenant_schema
        );
    END LOOP;
END $$;
