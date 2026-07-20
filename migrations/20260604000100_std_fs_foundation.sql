CREATE TABLE IF NOT EXISTS std_fs_item_versions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_code    VARCHAR(40) NOT NULL UNIQUE,
    industry_type   VARCHAR(20) NOT NULL,
    corp_type       VARCHAR(20) NOT NULL DEFAULT 'DOMESTIC',
    effective_from  DATE NOT NULL,
    effective_to    DATE,
    nts_doc_ref     VARCHAR(200),
    status          VARCHAR(15) NOT NULL DEFAULT 'DRAFT',
    xml_schema_ver  VARCHAR(40),
    created_by      BIGINT REFERENCES users(user_id),
    reviewed_by     BIGINT REFERENCES users(user_id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    activated_at    TIMESTAMPTZ,
    CHECK (effective_to IS NULL OR effective_from <= effective_to),
    CHECK (status IN ('DRAFT', 'REVIEWED', 'ACTIVE', 'RETIRED')),
    UNIQUE (industry_type, corp_type, effective_from)
);

CREATE INDEX IF NOT EXISTS idx_std_fs_item_versions_active
    ON std_fs_item_versions(industry_type, corp_type, effective_from DESC)
    WHERE status = 'ACTIVE';

CREATE TABLE IF NOT EXISTS std_fs_items (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id      UUID NOT NULL REFERENCES std_fs_item_versions(id) ON DELETE CASCADE,
    stmt_type       VARCHAR(10) NOT NULL,
    item_code       VARCHAR(10) NOT NULL,
    item_name       VARCHAR(150) NOT NULL,
    parent_code     VARCHAR(10),
    level           INT NOT NULL DEFAULT 1,
    account_class   VARCHAR(20),
    normal_balance  VARCHAR(6),
    is_subtotal     BOOLEAN NOT NULL DEFAULT FALSE,
    is_required     BOOLEAN NOT NULL DEFAULT FALSE,
    agg_formula     TEXT,
    xml_field_id    VARCHAR(40),
    sort_order      INT,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    CHECK (stmt_type IN ('STD_BS', 'STD_IS', 'STD_COST', 'STD_RE')),
    CHECK (normal_balance IS NULL OR normal_balance IN ('DEBIT', 'CREDIT')),
    UNIQUE (version_id, stmt_type, item_code),
    UNIQUE (version_id, item_code)
);

CREATE INDEX IF NOT EXISTS idx_std_fs_items_ver
    ON std_fs_items(version_id, stmt_type, sort_order, item_code);
CREATE INDEX IF NOT EXISTS idx_std_fs_items_parent
    ON std_fs_items(version_id, stmt_type, parent_code);

WITH version_row AS (
    INSERT INTO std_fs_item_versions (
        version_code, industry_type, corp_type, effective_from, effective_to,
        nts_doc_ref, status, xml_schema_ver, activated_at
    )
    VALUES (
        'NTS-2024-GENERAL', 'GENERAL', 'DOMESTIC', DATE '2024-01-01', NULL,
        'seed: standard financial statement baseline', 'ACTIVE', 'NTS-STD-FS-2024', NOW()
    )
    ON CONFLICT (version_code) DO UPDATE
    SET industry_type = EXCLUDED.industry_type,
        corp_type = EXCLUDED.corp_type,
        effective_from = EXCLUDED.effective_from,
        effective_to = EXCLUDED.effective_to,
        nts_doc_ref = EXCLUDED.nts_doc_ref,
        status = EXCLUDED.status,
        xml_schema_ver = EXCLUDED.xml_schema_ver,
        activated_at = COALESCE(std_fs_item_versions.activated_at, EXCLUDED.activated_at)
    RETURNING id
)
INSERT INTO std_fs_items (
    version_id, stmt_type, item_code, item_name, parent_code, level,
    account_class, normal_balance, is_subtotal, is_required, agg_formula,
    xml_field_id, sort_order
)
SELECT version_row.id, seed.stmt_type, seed.item_code, seed.item_name, seed.parent_code,
       seed.level, seed.account_class, seed.normal_balance, seed.is_subtotal,
       seed.is_required, seed.agg_formula, seed.xml_field_id, seed.sort_order
FROM version_row
CROSS JOIN (
    VALUES
        ('STD_BS', '1000', '자산', NULL, 1, 'ASSET', 'DEBIT', TRUE, TRUE, '1100+1500', 'BS_ASSET_TOTAL', 100),
        ('STD_BS', '1100', '유동자산', '1000', 2, 'ASSET', 'DEBIT', TRUE, FALSE, NULL, 'BS_CURRENT_ASSET', 110),
        ('STD_BS', '1010', '현금및현금성자산', '1100', 3, 'ASSET', 'DEBIT', FALSE, FALSE, NULL, 'BS_CASH', 111),
        ('STD_BS', '1030', '매출채권', '1100', 3, 'ASSET', 'DEBIT', FALSE, FALSE, NULL, 'BS_AR', 112),
        ('STD_BS', '1050', '선급비용', '1100', 3, 'ASSET', 'DEBIT', FALSE, FALSE, NULL, 'BS_PREPAID', 113),
        ('STD_BS', '1200', '재고자산', '1100', 3, 'ASSET', 'DEBIT', FALSE, FALSE, NULL, 'BS_INVENTORY', 114),
        ('STD_BS', '1500', '비유동자산', '1000', 2, 'ASSET', 'DEBIT', TRUE, FALSE, NULL, 'BS_NONCURRENT_ASSET', 150),
        ('STD_BS', '1521', '토지', '1500', 3, 'ASSET', 'DEBIT', FALSE, FALSE, NULL, 'BS_LAND', 151),
        ('STD_BS', '1522', '건물', '1500', 3, 'ASSET', 'DEBIT', FALSE, FALSE, NULL, 'BS_BUILDING', 152),
        ('STD_BS', '1523', '차량운반구', '1500', 3, 'ASSET', 'DEBIT', FALSE, FALSE, NULL, 'BS_VEHICLE', 153),
        ('STD_BS', '1524', '기계장치', '1500', 3, 'ASSET', 'DEBIT', FALSE, FALSE, NULL, 'BS_MACHINERY', 154),
        ('STD_BS', '1530', '무형자산', '1500', 3, 'ASSET', 'DEBIT', FALSE, FALSE, NULL, 'BS_INTANGIBLE', 155),
        ('STD_BS', '2000', '부채', NULL, 1, 'LIABILITY', 'CREDIT', TRUE, TRUE, NULL, 'BS_LIABILITY_TOTAL', 200),
        ('STD_BS', '2010', '매입채무', '2000', 2, 'LIABILITY', 'CREDIT', FALSE, FALSE, NULL, 'BS_AP', 201),
        ('STD_BS', '2020', '차입금', '2000', 2, 'LIABILITY', 'CREDIT', FALSE, FALSE, NULL, 'BS_LOAN', 202),
        ('STD_BS', '2030', '미지급세금', '2000', 2, 'LIABILITY', 'CREDIT', FALSE, FALSE, NULL, 'BS_TAX_PAYABLE', 203),
        ('STD_BS', '2040', '미지급비용', '2000', 2, 'LIABILITY', 'CREDIT', FALSE, FALSE, NULL, 'BS_ACCRUAL', 204),
        ('STD_BS', '3000', '자본', NULL, 1, 'EQUITY', 'CREDIT', TRUE, TRUE, NULL, 'BS_EQUITY_TOTAL', 300),
        ('STD_BS', '3010', '자본금', '3000', 2, 'EQUITY', 'CREDIT', FALSE, FALSE, NULL, 'BS_CAPITAL', 301),
        ('STD_BS', '3020', '이익잉여금', '3000', 2, 'EQUITY', 'CREDIT', FALSE, FALSE, NULL, 'BS_RETAINED_EARNINGS', 302),
        ('STD_IS', '4000', '매출액', NULL, 1, 'REVENUE', 'CREDIT', TRUE, TRUE, NULL, 'IS_REVENUE_TOTAL', 400),
        ('STD_IS', '4010', '제품매출', '4000', 2, 'REVENUE', 'CREDIT', FALSE, FALSE, NULL, 'IS_PRODUCT_REVENUE', 401),
        ('STD_IS', '4020', '용역매출', '4000', 2, 'REVENUE', 'CREDIT', FALSE, FALSE, NULL, 'IS_SERVICE_REVENUE', 402),
        ('STD_IS', '4030', '이자수익', '4000', 2, 'REVENUE', 'CREDIT', FALSE, FALSE, NULL, 'IS_INTEREST_INCOME', 403),
        ('STD_IS', '4040', '외환차익', '4000', 2, 'REVENUE', 'CREDIT', FALSE, FALSE, NULL, 'IS_FX_GAIN', 404),
        ('STD_IS', '4500', '매출원가', NULL, 1, 'EXPENSE', 'DEBIT', TRUE, FALSE, NULL, 'IS_COGS_TOTAL', 450),
        ('STD_IS', '4510', '매출원가', '4500', 2, 'EXPENSE', 'DEBIT', FALSE, FALSE, NULL, 'IS_COGS', 451),
        ('STD_IS', '5100', '판매비와관리비', NULL, 1, 'EXPENSE', 'DEBIT', TRUE, FALSE, NULL, 'IS_SGA_TOTAL', 510),
        ('STD_IS', '5110', '급여', '5100', 2, 'EXPENSE', 'DEBIT', FALSE, FALSE, NULL, 'IS_SALARY', 511),
        ('STD_IS', '5120', '임차료', '5100', 2, 'EXPENSE', 'DEBIT', FALSE, FALSE, NULL, 'IS_RENT', 512),
        ('STD_IS', '5130', '기부금', '5100', 2, 'EXPENSE', 'DEBIT', FALSE, FALSE, NULL, 'IS_DONATION', 513),
        ('STD_IS', '5140', '접대비', '5100', 2, 'EXPENSE', 'DEBIT', FALSE, FALSE, NULL, 'IS_ENTERTAINMENT', 514),
        ('STD_IS', '5150', '이자비용', '5100', 2, 'EXPENSE', 'DEBIT', FALSE, FALSE, NULL, 'IS_INTEREST_EXPENSE', 515),
        ('STD_IS', '5170', '감가상각비', '5100', 2, 'EXPENSE', 'DEBIT', FALSE, FALSE, NULL, 'IS_DEPRECIATION', 517),
        ('STD_IS', '5180', '연구개발비', '5100', 2, 'EXPENSE', 'DEBIT', FALSE, FALSE, NULL, 'IS_RND', 518),
        ('STD_IS', '5190', '해외용역비', '5100', 2, 'EXPENSE', 'DEBIT', FALSE, FALSE, NULL, 'IS_FOREIGN_EXPENSE', 519),
        ('STD_IS', '8500', '법인세비용', NULL, 1, 'EXPENSE', 'DEBIT', FALSE, FALSE, NULL, 'IS_TAX_EXPENSE', 850),
        ('STD_IS', '9000', '당기순이익', NULL, 1, 'EQUITY', 'CREDIT', TRUE, TRUE, NULL, 'IS_NET_INCOME', 900)
) AS seed(
    stmt_type, item_code, item_name, parent_code, level, account_class, normal_balance,
    is_subtotal, is_required, agg_formula, xml_field_id, sort_order
)
ON CONFLICT (version_id, stmt_type, item_code) DO UPDATE
SET item_name = EXCLUDED.item_name,
    parent_code = EXCLUDED.parent_code,
    level = EXCLUDED.level,
    account_class = EXCLUDED.account_class,
    normal_balance = EXCLUDED.normal_balance,
    is_subtotal = EXCLUDED.is_subtotal,
    is_required = EXCLUDED.is_required,
    agg_formula = EXCLUDED.agg_formula,
    xml_field_id = EXCLUDED.xml_field_id,
    sort_order = EXCLUDED.sort_order,
    is_active = TRUE;

DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM information_schema.schemata
        WHERE schema_name LIKE 'tenant\_%' ESCAPE '\'
    LOOP
        IF to_regclass(format('%I.fs_lines', tenant_schema)) IS NOT NULL THEN
            EXECUTE format(
                'ALTER TABLE %I.fs_lines
                 ADD COLUMN IF NOT EXISTS std_fs_item_code VARCHAR(10)',
                tenant_schema
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_fs_lines_stdfs
                 ON %I.fs_lines(std_fs_item_code)',
                tenant_schema
            );
        END IF;

        IF to_regclass(format('%I.by_law_snapshot', tenant_schema)) IS NOT NULL THEN
            EXECUTE format(
                'ALTER TABLE %I.by_law_snapshot
                 ADD COLUMN IF NOT EXISTS std_fs_version_id UUID REFERENCES public.std_fs_item_versions(id)',
                tenant_schema
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_by_law_snapshot_stdfs
                 ON %I.by_law_snapshot(std_fs_version_id)',
                tenant_schema
            );
        END IF;

        IF to_regclass(format('%I.customers', tenant_schema)) IS NOT NULL THEN
            EXECUTE format(
                'CREATE TABLE IF NOT EXISTS %I.std_fs_mappings (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    tenant_id BIGINT NOT NULL REFERENCES public.tenants(tenant_id),
                    customer_id BIGINT NOT NULL REFERENCES %I.customers(customer_id) ON DELETE CASCADE,
                    version_id UUID NOT NULL REFERENCES public.std_fs_item_versions(id),
                    account_code VARCHAR(50) NOT NULL,
                    account_name VARCHAR(200),
                    std_fs_item_code VARCHAR(10) NOT NULL,
                    is_auto_mapped BOOLEAN NOT NULL DEFAULT FALSE,
                    usage_count INT NOT NULL DEFAULT 1,
                    last_used_at TIMESTAMPTZ,
                    created_by BIGINT REFERENCES public.users(user_id),
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    UNIQUE(customer_id, version_id, account_code)
                )',
                tenant_schema,
                tenant_schema
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_std_fs_mappings_customer
                 ON %I.std_fs_mappings(customer_id, version_id, std_fs_item_code)',
                tenant_schema
            );
        END IF;

        IF to_regclass(format('%I.business_years', tenant_schema)) IS NOT NULL THEN
            EXECUTE format(
                'CREATE TABLE IF NOT EXISTS %I.std_fs_statements (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    tenant_id BIGINT NOT NULL REFERENCES public.tenants(tenant_id),
                    business_year_id BIGINT NOT NULL REFERENCES %I.business_years(by_id) ON DELETE CASCADE,
                    version_id UUID NOT NULL REFERENCES public.std_fs_item_versions(id),
                    stmt_type VARCHAR(10) NOT NULL,
                    status VARCHAR(15) NOT NULL DEFAULT ''DRAFT'',
                    item_code VARCHAR(10) NOT NULL,
                    amount BIGINT NOT NULL DEFAULT 0,
                    source_line_ids JSONB NOT NULL DEFAULT ''[]''::jsonb,
                    total_check JSONB NOT NULL DEFAULT ''{}''::jsonb,
                    confirmed_at TIMESTAMPTZ,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    CHECK (stmt_type IN (''STD_BS'', ''STD_IS'', ''STD_COST'', ''STD_RE'')),
                    CHECK (status IN (''DRAFT'', ''CONFIRMED'', ''SUPERSEDED'')),
                    UNIQUE(business_year_id, version_id, stmt_type, item_code, status)
                )',
                tenant_schema,
                tenant_schema
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_std_fs_statements_by
                 ON %I.std_fs_statements(business_year_id, version_id, stmt_type, status)',
                tenant_schema
            );
        END IF;
    END LOOP;
END $$;
