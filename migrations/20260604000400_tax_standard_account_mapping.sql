CREATE TABLE IF NOT EXISTS standard_accounts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code            VARCHAR(50) NOT NULL UNIQUE,
    name_ko         VARCHAR(100) NOT NULL,
    fs_type         VARCHAR(10) NOT NULL CHECK (fs_type IN ('BS', 'IS', 'CF', 'CE')),
    account_class   VARCHAR(20) NOT NULL CHECK (
        account_class IN ('ASSET', 'LIABILITY', 'EQUITY', 'REVENUE', 'EXPENSE', 'CONTRA')
    ),
    normal_balance  VARCHAR(6) NOT NULL CHECK (normal_balance IN ('DEBIT', 'CREDIT')),
    tax_relevance   VARCHAR(20),
    sub_class       VARCHAR(30),
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order      INT
);

CREATE INDEX IF NOT EXISTS idx_standard_accounts_tax_relevance
    ON standard_accounts(tax_relevance, is_active, sort_order);

WITH seed(code, name_ko, fs_type, account_class, normal_balance, tax_relevance, sub_class, sort_order) AS (
    VALUES
        ('CASH', 'Cash', 'BS', 'ASSET', 'DEBIT', NULL, 'CURRENT_ASSET', 10),
        ('STD_CASH', 'Cash', 'BS', 'ASSET', 'DEBIT', NULL, 'CURRENT_ASSET', 11),
        ('AR', 'Accounts receivable', 'BS', 'ASSET', 'DEBIT', 'AR', 'CURRENT_ASSET', 20),
        ('STD_AR', 'Accounts receivable', 'BS', 'ASSET', 'DEBIT', 'AR', 'CURRENT_ASSET', 21),
        ('STD_INVENTORY', 'Inventory', 'BS', 'ASSET', 'DEBIT', 'INVENTORY', 'CURRENT_ASSET', 30),
        ('STD_PREPAID', 'Prepaid expenses', 'BS', 'ASSET', 'DEBIT', NULL, 'CURRENT_ASSET', 40),
        ('PPE_NET', 'Property, plant and equipment', 'BS', 'ASSET', 'DEBIT', 'PPE', 'NON_CURRENT_ASSET', 50),
        ('STD_LAND', 'Land', 'BS', 'ASSET', 'DEBIT', 'PPE', 'NON_CURRENT_ASSET', 51),
        ('STD_BUILDING', 'Buildings', 'BS', 'ASSET', 'DEBIT', 'PPE', 'NON_CURRENT_ASSET', 52),
        ('STD_VEHICLE', 'Vehicles', 'BS', 'ASSET', 'DEBIT', 'PPE', 'NON_CURRENT_ASSET', 53),
        ('STD_MACHINERY', 'Machinery', 'BS', 'ASSET', 'DEBIT', 'PPE', 'NON_CURRENT_ASSET', 54),
        ('STD_INTANGIBLE', 'Intangible assets', 'BS', 'ASSET', 'DEBIT', 'INTANGIBLE', 'NON_CURRENT_ASSET', 60),
        ('STD_PAYABLE', 'Accounts payable', 'BS', 'LIABILITY', 'CREDIT', NULL, 'CURRENT_LIABILITY', 100),
        ('STD_LOAN', 'Borrowings', 'BS', 'LIABILITY', 'CREDIT', 'INTEREST_EXP', 'LIABILITY', 110),
        ('STD_TAX_PAYABLE', 'Tax payable', 'BS', 'LIABILITY', 'CREDIT', NULL, 'CURRENT_LIABILITY', 120),
        ('STD_ACCRUAL', 'Accrued expenses', 'BS', 'LIABILITY', 'CREDIT', NULL, 'CURRENT_LIABILITY', 130),
        ('STD_CAPITAL', 'Capital stock', 'BS', 'EQUITY', 'CREDIT', NULL, 'EQUITY', 200),
        ('STD_RETAINED_EARNINGS', 'Retained earnings', 'BS', 'EQUITY', 'CREDIT', NULL, 'EQUITY', 210),
        ('REVENUE', 'Revenue', 'IS', 'REVENUE', 'CREDIT', 'REVENUE', NULL, 300),
        ('STD_PRODUCT_REVENUE', 'Product revenue', 'IS', 'REVENUE', 'CREDIT', 'REVENUE', NULL, 301),
        ('STD_SERVICE_REVENUE', 'Service revenue', 'IS', 'REVENUE', 'CREDIT', 'REVENUE', NULL, 302),
        ('STD_INTEREST_INCOME', 'Interest income', 'IS', 'REVENUE', 'CREDIT', NULL, NULL, 303),
        ('STD_FX_GAIN', 'Foreign exchange gain', 'IS', 'REVENUE', 'CREDIT', NULL, NULL, 304),
        ('STD_COGS', 'Cost of goods sold', 'IS', 'EXPENSE', 'DEBIT', NULL, NULL, 400),
        ('STD_SALARY', 'Salary expense', 'IS', 'EXPENSE', 'DEBIT', NULL, NULL, 410),
        ('STD_RENT', 'Rent expense', 'IS', 'EXPENSE', 'DEBIT', NULL, NULL, 420),
        ('DONATION_EXP', 'Donation expense', 'IS', 'EXPENSE', 'DEBIT', 'DONATION', NULL, 430),
        ('STD_DONATION', 'Donation expense', 'IS', 'EXPENSE', 'DEBIT', 'DONATION', NULL, 431),
        ('ENTERTAIN_EXP', 'Entertainment expense', 'IS', 'EXPENSE', 'DEBIT', 'ENTERTAINMENT', NULL, 440),
        ('STD_ENTERTAINMENT', 'Entertainment expense', 'IS', 'EXPENSE', 'DEBIT', 'ENTERTAINMENT', NULL, 441),
        ('INTEREST_EXP', 'Interest expense', 'IS', 'EXPENSE', 'DEBIT', 'INTEREST_EXP', NULL, 450),
        ('STD_INTEREST_EXPENSE', 'Interest expense', 'IS', 'EXPENSE', 'DEBIT', 'INTEREST_EXP', NULL, 451),
        ('STD_DEPRECIATION', 'Depreciation expense', 'IS', 'EXPENSE', 'DEBIT', 'PPE', NULL, 460),
        ('STD_RND', 'Research and development expense', 'IS', 'EXPENSE', 'DEBIT', NULL, NULL, 470),
        ('STD_FOREIGN_SERVICE', 'Foreign service expense', 'IS', 'EXPENSE', 'DEBIT', NULL, NULL, 480),
        ('STD_TAX_EXPENSE', 'Income tax expense', 'IS', 'EXPENSE', 'DEBIT', NULL, NULL, 490),
        ('STD_EXPENSE', 'General expense', 'IS', 'EXPENSE', 'DEBIT', NULL, NULL, 500),
        ('NET_INCOME', 'Net income', 'IS', 'EQUITY', 'CREDIT', 'NET_INCOME', NULL, 900),
        ('STD_NET_INCOME', 'Net income', 'IS', 'EQUITY', 'CREDIT', 'NET_INCOME', NULL, 901),
        ('ACCOUNTING_INCOME', 'Accounting income', 'IS', 'EQUITY', 'CREDIT', 'NET_INCOME', NULL, 902),
        ('PENSION_PROV', 'Pension provision', 'BS', 'LIABILITY', 'CREDIT', 'PENSION', 'LIABILITY', 910),
        ('BAD_DEBT_PROV', 'Bad debt provision', 'BS', 'CONTRA', 'CREDIT', 'BAD_DEBT', 'CONTRA_ASSET', 920)
)
INSERT INTO standard_accounts (
    code, name_ko, fs_type, account_class, normal_balance, tax_relevance, sub_class, sort_order
)
SELECT code, name_ko, fs_type, account_class, normal_balance, tax_relevance, sub_class, sort_order
FROM seed
ON CONFLICT (code) DO UPDATE
SET name_ko = EXCLUDED.name_ko,
    fs_type = EXCLUDED.fs_type,
    account_class = EXCLUDED.account_class,
    normal_balance = EXCLUDED.normal_balance,
    tax_relevance = EXCLUDED.tax_relevance,
    sub_class = EXCLUDED.sub_class,
    sort_order = EXCLUDED.sort_order,
    is_active = TRUE;

INSERT INTO validation_rules (rule_code, severity, area, message_template, applies_to)
VALUES (
    'TD_TAX_REQUIRED_MAPPINGS',
    'ERROR',
    'tax-data',
    'Required tax standard account mappings are missing or mismatched: {mandatory_mapping_missing_codes}',
    '#/workspace/ws-info'
)
ON CONFLICT (rule_code) DO UPDATE
SET severity = EXCLUDED.severity,
    area = EXCLUDED.area,
    message_template = EXCLUDED.message_template,
    applies_to = EXCLUDED.applies_to,
    active = TRUE;

DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM information_schema.schemata
        WHERE schema_name LIKE 'tenant\_%' ESCAPE '\'
    LOOP
        IF to_regclass(format('%I.account_mappings', tenant_schema)) IS NOT NULL THEN
            EXECUTE format(
                'ALTER TABLE %I.account_mappings
                 ADD COLUMN IF NOT EXISTS std_account_code VARCHAR(50)',
                tenant_schema
            );
            EXECUTE format(
                'ALTER TABLE %I.account_mappings
                 ADD COLUMN IF NOT EXISTS std_account_name VARCHAR(100)',
                tenant_schema
            );
            EXECUTE format(
                'ALTER TABLE %I.account_mappings
                 ADD COLUMN IF NOT EXISTS is_auto_mapped BOOLEAN NOT NULL DEFAULT FALSE',
                tenant_schema
            );
            EXECUTE format(
                'ALTER TABLE %I.account_mappings
                 ADD COLUMN IF NOT EXISTS map_confidence DOUBLE PRECISION NOT NULL DEFAULT 1.000',
                tenant_schema
            );
            EXECUTE format(
                'UPDATE %I.account_mappings
                 SET std_account_code = COALESCE(std_account_code, NULLIF(TRIM(standard_account_code), '''')),
                     std_account_name = COALESCE(std_account_name, NULLIF(TRIM(standard_account_name), ''''))',
                tenant_schema
            );
            EXECUTE format(
                'INSERT INTO public.standard_accounts (
                     code, name_ko, fs_type, account_class, normal_balance, is_active
                 )
                 SELECT DISTINCT code,
                        COALESCE(MAX(account_name), code),
                        ''IS'', ''EXPENSE'', ''DEBIT'', TRUE
                 FROM (
                     SELECT NULLIF(TRIM(standard_account_code), '''') AS code,
                            NULLIF(TRIM(standard_account_name), '''') AS account_name
                     FROM %I.account_mappings
                 ) legacy
                 WHERE code IS NOT NULL
                   AND NOT EXISTS (
                       SELECT 1 FROM public.standard_accounts sa WHERE sa.code = legacy.code
                   )
                 GROUP BY code
                 ON CONFLICT (code) DO NOTHING',
                tenant_schema
            );
            IF NOT EXISTS (
                SELECT 1
                FROM pg_constraint
                WHERE conrelid = format('%I.account_mappings', tenant_schema)::regclass
                  AND conname = 'account_mappings_std_account_code_fkey'
            ) THEN
                EXECUTE format(
                    'ALTER TABLE %I.account_mappings
                     ADD CONSTRAINT account_mappings_std_account_code_fkey
                     FOREIGN KEY (std_account_code) REFERENCES public.standard_accounts(code)',
                    tenant_schema
                );
            END IF;
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_account_mappings_std_account
                 ON %I.account_mappings(customer_id, std_account_code)',
                tenant_schema
            );
        END IF;

        IF to_regclass(format('%I.fs_lines', tenant_schema)) IS NOT NULL THEN
            EXECUTE format(
                'ALTER TABLE %I.fs_lines
                 ADD COLUMN IF NOT EXISTS std_account_code VARCHAR(50)',
                tenant_schema
            );
            EXECUTE format(
                'ALTER TABLE %I.fs_lines
                 ADD COLUMN IF NOT EXISTS std_account_name VARCHAR(100)',
                tenant_schema
            );
            EXECUTE format(
                'ALTER TABLE %I.fs_lines
                 ADD COLUMN IF NOT EXISTS is_auto_mapped BOOLEAN NOT NULL DEFAULT FALSE',
                tenant_schema
            );
            EXECUTE format(
                'ALTER TABLE %I.fs_lines
                 ADD COLUMN IF NOT EXISTS map_confidence DOUBLE PRECISION',
                tenant_schema
            );
            EXECUTE format(
                'UPDATE %I.fs_lines
                 SET std_account_code = COALESCE(std_account_code, NULLIF(TRIM(standard_account_code), '''')),
                     std_account_name = COALESCE(std_account_name, NULLIF(TRIM(standard_account_name), ''''))',
                tenant_schema
            );
            EXECUTE format(
                'INSERT INTO public.standard_accounts (
                     code, name_ko, fs_type, account_class, normal_balance, is_active
                 )
                 SELECT DISTINCT code,
                        COALESCE(MAX(account_name), code),
                        ''IS'', ''EXPENSE'', ''DEBIT'', TRUE
                 FROM (
                     SELECT NULLIF(TRIM(standard_account_code), '''') AS code,
                            NULLIF(TRIM(standard_account_name), '''') AS account_name
                     FROM %I.fs_lines
                 ) legacy
                 WHERE code IS NOT NULL
                   AND NOT EXISTS (
                       SELECT 1 FROM public.standard_accounts sa WHERE sa.code = legacy.code
                   )
                 GROUP BY code
                 ON CONFLICT (code) DO NOTHING',
                tenant_schema
            );
            IF NOT EXISTS (
                SELECT 1
                FROM pg_constraint
                WHERE conrelid = format('%I.fs_lines', tenant_schema)::regclass
                  AND conname = 'fs_lines_std_account_code_fkey'
            ) THEN
                EXECUTE format(
                    'ALTER TABLE %I.fs_lines
                     ADD CONSTRAINT fs_lines_std_account_code_fkey
                     FOREIGN KEY (std_account_code) REFERENCES public.standard_accounts(code)',
                    tenant_schema
                );
            END IF;
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_fs_lines_std_acct
                 ON %I.fs_lines(std_account_code)',
                tenant_schema
            );
        END IF;
    END LOOP;
END $$;
