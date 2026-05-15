DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM information_schema.schemata
        WHERE schema_name LIKE 'tenant\_%' ESCAPE '\'
    LOOP
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I.import_batches (
                batch_id BIGSERIAL PRIMARY KEY,
                by_id BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                customer_id BIGINT REFERENCES %I.customers(customer_id),
                data_type VARCHAR(40) NOT NULL,
                source_file_name VARCHAR(255),
                row_count INT NOT NULL DEFAULT 0,
                valid_count INT NOT NULL DEFAULT 0,
                error_count INT NOT NULL DEFAULT 0,
                auto_mapped_count INT NOT NULL DEFAULT 0,
                status VARCHAR(30) NOT NULL DEFAULT ''IMPORTED'',
                metadata JSONB NOT NULL DEFAULT ''{}''::jsonb,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )',
            tenant_schema,
            tenant_schema,
            tenant_schema
        );
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS idx_import_batches_by
             ON %I.import_batches(by_id, data_type, created_at DESC)',
            tenant_schema
        );

        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I.import_errors (
                error_id BIGSERIAL PRIMARY KEY,
                batch_id BIGINT NOT NULL REFERENCES %I.import_batches(batch_id) ON DELETE CASCADE,
                row_no INT NOT NULL,
                field_name VARCHAR(80),
                severity VARCHAR(20) NOT NULL DEFAULT ''ERROR'',
                message TEXT NOT NULL,
                raw_row JSONB NOT NULL DEFAULT ''{}''::jsonb,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )',
            tenant_schema,
            tenant_schema
        );

        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I.account_mappings (
                mapping_id BIGSERIAL PRIMARY KEY,
                customer_id BIGINT NOT NULL REFERENCES %I.customers(customer_id),
                statement_type VARCHAR(30) NOT NULL DEFAULT ''BS'',
                source_account_code VARCHAR(50) NOT NULL,
                source_account_name VARCHAR(200) NOT NULL,
                standard_account_code VARCHAR(50) NOT NULL,
                standard_account_name VARCHAR(200) NOT NULL,
                use_count INT NOT NULL DEFAULT 1,
                last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE(customer_id, statement_type, source_account_code)
            )',
            tenant_schema,
            tenant_schema
        );

        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I.transactions (
                transaction_id BIGSERIAL PRIMARY KEY,
                by_id BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                batch_id BIGINT REFERENCES %I.import_batches(batch_id),
                tx_date DATE NOT NULL,
                partner_name VARCHAR(200) NOT NULL,
                category VARCHAR(40) NOT NULL,
                account_code VARCHAR(50),
                description TEXT,
                amount BIGINT NOT NULL,
                evidence_type VARCHAR(40),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )',
            tenant_schema,
            tenant_schema,
            tenant_schema
        );
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS idx_transactions_by
             ON %I.transactions(by_id, category, tx_date)',
            tenant_schema
        );

        EXECUTE format(
            'ALTER TABLE %I.financial_statements
             ADD COLUMN IF NOT EXISTS batch_id BIGINT REFERENCES %I.import_batches(batch_id)',
            tenant_schema,
            tenant_schema
        );
        EXECUTE format(
            'ALTER TABLE %I.fs_lines
             ADD COLUMN IF NOT EXISTS batch_id BIGINT REFERENCES %I.import_batches(batch_id)',
            tenant_schema,
            tenant_schema
        );
        EXECUTE format(
            'ALTER TABLE %I.fs_lines
             ADD COLUMN IF NOT EXISTS row_no INT',
            tenant_schema
        );
        EXECUTE format(
            'ALTER TABLE %I.fs_lines
             ADD COLUMN IF NOT EXISTS standard_account_code VARCHAR(50)',
            tenant_schema
        );
        EXECUTE format(
            'ALTER TABLE %I.fs_lines
             ADD COLUMN IF NOT EXISTS standard_account_name VARCHAR(200)',
            tenant_schema
        );
        EXECUTE format(
            'ALTER TABLE %I.assets
             ADD COLUMN IF NOT EXISTS batch_id BIGINT REFERENCES %I.import_batches(batch_id)',
            tenant_schema,
            tenant_schema
        );
        EXECUTE format(
            'ALTER TABLE %I.assets
             ADD COLUMN IF NOT EXISTS asset_category VARCHAR(50) NOT NULL DEFAULT ''GENERAL''',
            tenant_schema
        );
        EXECUTE format(
            'ALTER TABLE %I.assets
             ADD COLUMN IF NOT EXISTS is_business_vehicle BOOLEAN NOT NULL DEFAULT FALSE',
            tenant_schema
        );
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS idx_assets_by
             ON %I.assets(by_id, asset_category, asset_code)',
            tenant_schema
        );
    END LOOP;
END $$;
