use anyhow::{Context, Result};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{
    db::{execute_batch, quote_ident},
    domain::{
        AmendmentDiff, AmendmentPreview, ApprovalLine, AuditLog, BusinessYear,
        BusinessYearWorkflow, CreateBusinessYearRequest, CreateCustomerRequest,
        CreateTenantRequest, Customer, DashboardSummary, Notification, TaxBurdenReportRow, Tenant,
        TenantRef, UpdateBusinessYearStatusRequest, WorkflowEvent, YearComparisonReportRow,
    },
};

const DEFAULT_CUSTOMER_WORK_SCOPES: &[&str] = &["INFO", "ADJUST", "FORM", "VALIDATE", "PRINT"];
const ALLOWED_CUSTOMER_WORK_SCOPES: &[&str] = &[
    "INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST",
];

pub fn normalize_tenant_code(code: &str) -> Result<String> {
    let normalized = code.trim().to_ascii_lowercase();
    if normalized.is_empty()
        || normalized.len() > 20
        || !normalized
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        || normalized
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
    {
        anyhow::bail!("tenant_code must match ^[a-z][a-z0-9_]*$ and be <= 20 characters");
    }
    Ok(normalized)
}

pub async fn create_tenant(pool: &PgPool, request: CreateTenantRequest) -> Result<Tenant> {
    let tenant_code = normalize_tenant_code(&request.tenant_code)?;
    let schema_name = format!("tenant_{tenant_code}");

    let tenant = sqlx::query_as::<_, Tenant>(
        r#"
        INSERT INTO tenants (
            tenant_code,
            tenant_name,
            biz_reg_no,
            contract_start,
            contract_end,
            schema_name,
            allowed_ips,
            max_users
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, COALESCE($8, 10))
        RETURNING
            tenant_id, tenant_code, tenant_name, biz_reg_no, contract_start,
            contract_end, schema_name, status, allowed_ips, max_users,
            created_at, updated_at
        "#,
    )
    .bind(&tenant_code)
    .bind(request.tenant_name.trim())
    .bind(request.biz_reg_no.trim())
    .bind(request.contract_start)
    .bind(request.contract_end)
    .bind(&schema_name)
    .bind(request.allowed_ips)
    .bind(request.max_users)
    .fetch_one(pool)
    .await
    .context("failed to insert tenant")?;

    provision_tenant_schema(pool, &tenant.schema_name).await?;
    Ok(tenant)
}

pub async fn list_tenants(pool: &PgPool) -> Result<Vec<Tenant>> {
    sqlx::query_as::<_, Tenant>(
        r#"
        SELECT tenant_id, tenant_code, tenant_name, biz_reg_no, contract_start,
               contract_end, schema_name, status, allowed_ips, max_users,
               created_at, updated_at
        FROM tenants
        ORDER BY tenant_code
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list tenants")
}

pub async fn resolve_tenant(pool: &PgPool, tenant_code: &str) -> Result<TenantRef> {
    let tenant_code = normalize_tenant_code(tenant_code)?;
    sqlx::query_as::<_, TenantRef>(
        r#"
        SELECT tenant_id, tenant_code, schema_name
        FROM tenants
        WHERE tenant_code = $1 AND status = 'ACTIVE'
        "#,
    )
    .bind(tenant_code)
    .fetch_one(pool)
    .await
    .context("tenant not found")
}

pub async fn provision_tenant_schema(pool: &PgPool, schema_name: &str) -> Result<()> {
    let schema = quote_ident(schema_name)?;
    let sql = format!(
        r#"
        CREATE SCHEMA IF NOT EXISTS {schema};

        CREATE TABLE IF NOT EXISTS {schema}.customers (
            customer_id     BIGSERIAL PRIMARY KEY,
            tenant_id       BIGINT NOT NULL REFERENCES public.tenants(tenant_id),
            customer_code   VARCHAR(50) NOT NULL,
            customer_name   VARCHAR(200) NOT NULL,
            biz_reg_no      VARCHAR(13) NOT NULL,
            corp_reg_no     VARCHAR(20),
            industry_code   VARCHAR(20),
            is_sme          BOOLEAN NOT NULL DEFAULT FALSE,
            work_scopes     TEXT[] NOT NULL DEFAULT ARRAY['INFO','ADJUST','FORM','VALIDATE','PRINT']::TEXT[],
            status          VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(tenant_id, customer_code)
        );
        ALTER TABLE {schema}.customers
            ADD COLUMN IF NOT EXISTS work_scopes TEXT[] NOT NULL DEFAULT ARRAY['INFO','ADJUST','FORM','VALIDATE','PRINT']::TEXT[];

        CREATE TABLE IF NOT EXISTS {schema}.business_years (
            by_id           BIGSERIAL PRIMARY KEY,
            customer_id     BIGINT NOT NULL REFERENCES {schema}.customers(customer_id),
            year_label      INT NOT NULL,
            start_date      DATE NOT NULL,
            end_date        DATE NOT NULL,
            status          VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
            locked_at       TIMESTAMPTZ,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CHECK (start_date <= end_date),
            UNIQUE(customer_id, year_label)
        );
        ALTER TABLE {schema}.business_years
            ALTER COLUMN status SET DEFAULT 'DRAFT';
        UPDATE {schema}.business_years
            SET status = 'DRAFT'
            WHERE status = 'OPEN';

        CREATE TABLE IF NOT EXISTS {schema}.import_batches (
            batch_id        BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            customer_id     BIGINT REFERENCES {schema}.customers(customer_id),
            data_type       VARCHAR(40) NOT NULL,
            source_file_name VARCHAR(255),
            row_count       INT NOT NULL DEFAULT 0,
            valid_count     INT NOT NULL DEFAULT 0,
            error_count     INT NOT NULL DEFAULT 0,
            auto_mapped_count INT NOT NULL DEFAULT 0,
            status          VARCHAR(30) NOT NULL DEFAULT 'IMPORTED',
            metadata        JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_import_batches_by
            ON {schema}.import_batches(by_id, data_type, created_at DESC);

        CREATE TABLE IF NOT EXISTS {schema}.import_errors (
            error_id        BIGSERIAL PRIMARY KEY,
            batch_id        BIGINT NOT NULL REFERENCES {schema}.import_batches(batch_id) ON DELETE CASCADE,
            row_no          INT NOT NULL,
            field_name      VARCHAR(80),
            severity        VARCHAR(20) NOT NULL DEFAULT 'ERROR',
            message         TEXT NOT NULL,
            raw_row         JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS {schema}.account_mappings (
            mapping_id      BIGSERIAL PRIMARY KEY,
            customer_id     BIGINT NOT NULL REFERENCES {schema}.customers(customer_id),
            statement_type  VARCHAR(30) NOT NULL DEFAULT 'BS',
            source_account_code VARCHAR(50) NOT NULL,
            source_account_name VARCHAR(200) NOT NULL,
            standard_account_code VARCHAR(50) NOT NULL,
            standard_account_name VARCHAR(200) NOT NULL,
            use_count       INT NOT NULL DEFAULT 1,
            last_used_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(customer_id, statement_type, source_account_code)
        );

        CREATE TABLE IF NOT EXISTS {schema}.financial_statements (
            fs_id           BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            batch_id        BIGINT REFERENCES {schema}.import_batches(batch_id),
            statement_type  VARCHAR(30) NOT NULL,
            currency        VARCHAR(3) NOT NULL DEFAULT 'KRW',
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        ALTER TABLE {schema}.financial_statements
            ADD COLUMN IF NOT EXISTS batch_id BIGINT REFERENCES {schema}.import_batches(batch_id);

        CREATE TABLE IF NOT EXISTS {schema}.fs_lines (
            line_id         BIGSERIAL PRIMARY KEY,
            fs_id           BIGINT NOT NULL REFERENCES {schema}.financial_statements(fs_id),
            batch_id        BIGINT REFERENCES {schema}.import_batches(batch_id),
            row_no          INT,
            account_code    VARCHAR(50) NOT NULL,
            account_name    VARCHAR(200) NOT NULL,
            standard_account_code VARCHAR(50),
            standard_account_name VARCHAR(200),
            amount          BIGINT NOT NULL,
            debit_credit    VARCHAR(10) NOT NULL
        );
        ALTER TABLE {schema}.fs_lines
            ADD COLUMN IF NOT EXISTS batch_id BIGINT REFERENCES {schema}.import_batches(batch_id);
        ALTER TABLE {schema}.fs_lines
            ADD COLUMN IF NOT EXISTS row_no INT;
        ALTER TABLE {schema}.fs_lines
            ADD COLUMN IF NOT EXISTS standard_account_code VARCHAR(50);
        ALTER TABLE {schema}.fs_lines
            ADD COLUMN IF NOT EXISTS standard_account_name VARCHAR(200);

        CREATE TABLE IF NOT EXISTS {schema}.assets (
            asset_id        BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            batch_id        BIGINT REFERENCES {schema}.import_batches(batch_id),
            asset_code      VARCHAR(50) NOT NULL,
            asset_name      VARCHAR(200) NOT NULL,
            asset_category  VARCHAR(50) NOT NULL DEFAULT 'GENERAL',
            is_business_vehicle BOOLEAN NOT NULL DEFAULT FALSE,
            acquisition_date DATE NOT NULL,
            acquisition_cost BIGINT NOT NULL,
            useful_life_years INT NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        ALTER TABLE {schema}.assets
            ADD COLUMN IF NOT EXISTS batch_id BIGINT REFERENCES {schema}.import_batches(batch_id);
        ALTER TABLE {schema}.assets
            ADD COLUMN IF NOT EXISTS asset_category VARCHAR(50) NOT NULL DEFAULT 'GENERAL';
        ALTER TABLE {schema}.assets
            ADD COLUMN IF NOT EXISTS is_business_vehicle BOOLEAN NOT NULL DEFAULT FALSE;
        CREATE INDEX IF NOT EXISTS idx_assets_by
            ON {schema}.assets(by_id, asset_category, asset_code);

        CREATE TABLE IF NOT EXISTS {schema}.vehicle_usage_logs (
            usage_log_id   BIGSERIAL PRIMARY KEY,
            by_id          BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            asset_id       BIGINT NOT NULL REFERENCES {schema}.assets(asset_id),
            usage_month    DATE NOT NULL,
            total_distance_km DOUBLE PRECISION NOT NULL DEFAULT 0,
            business_distance_km DOUBLE PRECISION NOT NULL DEFAULT 0,
            business_use_bps INT NOT NULL DEFAULT 10000,
            created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(by_id, asset_id, usage_month)
        );
        CREATE INDEX IF NOT EXISTS idx_vehicle_usage_logs_by
            ON {schema}.vehicle_usage_logs(by_id, asset_id, usage_month);

        CREATE TABLE IF NOT EXISTS {schema}.depreciation (
            depreciation_id BIGSERIAL PRIMARY KEY,
            asset_id        BIGINT NOT NULL REFERENCES {schema}.assets(asset_id),
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            book_amount     BIGINT NOT NULL,
            tax_limit       BIGINT NOT NULL,
            adjustment_amount BIGINT NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS {schema}.transactions (
            transaction_id  BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            batch_id        BIGINT REFERENCES {schema}.import_batches(batch_id),
            tx_date         DATE NOT NULL,
            partner_name    VARCHAR(200) NOT NULL,
            category        VARCHAR(40) NOT NULL,
            account_code    VARCHAR(50),
            description     TEXT,
            amount          BIGINT NOT NULL,
            evidence_type   VARCHAR(40),
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_transactions_by
            ON {schema}.transactions(by_id, category, tx_date);

        CREATE TABLE IF NOT EXISTS {schema}.donation_carryforwards (
            carryforward_id BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
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
        );
        CREATE INDEX IF NOT EXISTS idx_donation_carryforwards_by
            ON {schema}.donation_carryforwards(by_id, donation_type, expires_year);

        CREATE TABLE IF NOT EXISTS {schema}.entertainment_revenue_breakdowns (
            revenue_breakdown_id BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            revenue_category VARCHAR(80) NOT NULL,
            amount          BIGINT NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_entertainment_revenue_breakdowns_by
            ON {schema}.entertainment_revenue_breakdowns(by_id, revenue_category);

        CREATE TABLE IF NOT EXISTS {schema}.loan_interest_facts (
            loan_interest_fact_id BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            weighted_average_loan_balance BIGINT NOT NULL DEFAULT 0,
            weighted_average_interest_rate_bps INT NOT NULL DEFAULT 0,
            deemed_interest BIGINT NOT NULL DEFAULT 0,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_loan_interest_facts_by
            ON {schema}.loan_interest_facts(by_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS {schema}.valuation_positions (
            valuation_position_id BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            module_code     VARCHAR(20) NOT NULL,
            item_code       VARCHAR(80) NOT NULL,
            item_name       VARCHAR(200) NOT NULL,
            position_type   VARCHAR(40) NOT NULL DEFAULT 'GENERAL',
            monetary        BOOLEAN NOT NULL DEFAULT TRUE,
            valuation_method VARCHAR(40) NOT NULL DEFAULT 'CLOSING_RATE',
            book_amount     BIGINT NOT NULL,
            tax_amount      BIGINT NOT NULL,
            adjustment_amount BIGINT NOT NULL,
            metadata        JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_valuation_positions_by
            ON {schema}.valuation_positions(by_id, module_code, item_code);

        CREATE TABLE IF NOT EXISTS {schema}.by_law_snapshot (
            snapshot_id      BIGSERIAL PRIMARY KEY,
            by_id            BIGINT NOT NULL UNIQUE REFERENCES {schema}.business_years(by_id),
            law_version_id   BIGINT NOT NULL REFERENCES public.tax_law_versions(law_version_id),
            rate_version_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
            form_version_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
            efile_master_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
            snapshot_data    JSONB NOT NULL,
            locked           BOOLEAN NOT NULL DEFAULT FALSE,
            created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS {schema}.tax_adjustments (
            adjustment_id  BIGSERIAL PRIMARY KEY,
            by_id          BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            adj_category   VARCHAR(50) NOT NULL,
            adj_code       VARCHAR(50) NOT NULL,
            amount         BIGINT NOT NULL,
            direction      VARCHAR(20) NOT NULL CHECK (direction IN ('ADD', 'DEDUCT', 'INFO')),
            description    TEXT,
            snapshot_id    BIGINT REFERENCES {schema}.by_law_snapshot(snapshot_id),
            metadata       JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            status         VARCHAR(20) NOT NULL DEFAULT 'POSTED',
            created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_tax_adjustments_by ON {schema}.tax_adjustments(by_id, adj_category, created_at DESC);

        CREATE TABLE IF NOT EXISTS {schema}.adjustment_items (
            adjustment_item_id BIGSERIAL PRIMARY KEY,
            by_id          BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            adjustment_id  BIGINT REFERENCES {schema}.tax_adjustments(adjustment_id) ON DELETE SET NULL,
            section        VARCHAR(50) NOT NULL,
            item_code      VARCHAR(80) NOT NULL,
            item_name      VARCHAR(200) NOT NULL,
            amount         BIGINT NOT NULL,
            direction      VARCHAR(20) NOT NULL CHECK (direction IN ('ADD', 'DEDUCT', 'INFO')),
            disposition    VARCHAR(40) NOT NULL DEFAULT 'OTHER',
            source_module  VARCHAR(50) NOT NULL DEFAULT 'B1',
            law_ref        VARCHAR(100),
            metadata       JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_adjustment_items_by
            ON {schema}.adjustment_items(by_id, source_module, section, item_code);

        CREATE TABLE IF NOT EXISTS {schema}.reserves (
            reserve_id     BIGSERIAL PRIMARY KEY,
            by_id          BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            adjustment_id  BIGINT REFERENCES {schema}.tax_adjustments(adjustment_id),
            reserve_code   VARCHAR(50) NOT NULL,
            amount         BIGINT NOT NULL,
            direction      VARCHAR(20) NOT NULL,
            carryforward_to INT,
            source_module  VARCHAR(50) NOT NULL DEFAULT 'MANUAL',
            created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        ALTER TABLE {schema}.reserves
            ADD COLUMN IF NOT EXISTS source_module VARCHAR(50) NOT NULL DEFAULT 'MANUAL';
        CREATE INDEX IF NOT EXISTS idx_reserves_by
            ON {schema}.reserves(by_id, source_module, reserve_code);

        CREATE TABLE IF NOT EXISTS {schema}.carryforward_loss (
            loss_id        BIGSERIAL PRIMARY KEY,
            customer_id    BIGINT NOT NULL REFERENCES {schema}.customers(customer_id),
            origin_year    INT NOT NULL,
            original_amount BIGINT NOT NULL,
            used_amount    BIGINT NOT NULL DEFAULT 0,
            expired_amount BIGINT NOT NULL DEFAULT 0,
            remaining_amount BIGINT NOT NULL,
            expires_year   INT NOT NULL,
            created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        ALTER TABLE {schema}.carryforward_loss
            ADD COLUMN IF NOT EXISTS used_amount BIGINT NOT NULL DEFAULT 0;
        ALTER TABLE {schema}.carryforward_loss
            ADD COLUMN IF NOT EXISTS expired_amount BIGINT NOT NULL DEFAULT 0;
        ALTER TABLE {schema}.carryforward_loss
            ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

        CREATE TABLE IF NOT EXISTS {schema}.capital_changes (
            capital_change_id BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            change_date     DATE NOT NULL,
            change_type     VARCHAR(40) NOT NULL,
            amount          BIGINT NOT NULL,
            description     TEXT,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_capital_changes_by
            ON {schema}.capital_changes(by_id, change_date, capital_change_id);

        CREATE TABLE IF NOT EXISTS {schema}.tax_credit_claims (
            credit_claim_id BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            credit_type     VARCHAR(50) NOT NULL,
            base_amount     BIGINT NOT NULL,
            rate_bps        BIGINT NOT NULL,
            requested_amount BIGINT NOT NULL,
            allowed_amount  BIGINT NOT NULL,
            metadata        JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_tax_credit_claims_by
            ON {schema}.tax_credit_claims(by_id, credit_type);

        CREATE TABLE IF NOT EXISTS {schema}.minimum_tax_results (
            minimum_tax_result_id BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            tax_base        BIGINT NOT NULL,
            regular_tax     BIGINT NOT NULL,
            minimum_tax     BIGINT NOT NULL,
            additional_tax  BIGINT NOT NULL,
            metadata        JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_minimum_tax_results_by
            ON {schema}.minimum_tax_results(by_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS {schema}.penalty_tax_items (
            penalty_tax_item_id BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            penalty_type    VARCHAR(50) NOT NULL,
            tax_base        BIGINT NOT NULL,
            rate_bps        BIGINT NOT NULL,
            days_late       INT,
            reduction_bps   BIGINT NOT NULL DEFAULT 0,
            penalty_amount  BIGINT NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_penalty_tax_items_by
            ON {schema}.penalty_tax_items(by_id, penalty_type);

        CREATE TABLE IF NOT EXISTS {schema}.foreign_income_items (
            foreign_income_item_id BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            income_type     VARCHAR(50) NOT NULL,
            gross_amount    BIGINT NOT NULL,
            attributable_expense BIGINT NOT NULL DEFAULT 0,
            pe_allocation_bps BIGINT NOT NULL DEFAULT 10000,
            allocated_income BIGINT NOT NULL,
            withholding_tax BIGINT NOT NULL DEFAULT 0,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_foreign_income_items_by
            ON {schema}.foreign_income_items(by_id, income_type);

        CREATE TABLE IF NOT EXISTS {schema}.consolidated_entities (
            consolidated_entity_id BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            entity_code     VARCHAR(50) NOT NULL,
            entity_name     VARCHAR(200) NOT NULL,
            ownership_bps   BIGINT NOT NULL,
            taxable_income  BIGINT NOT NULL,
            standalone_tax  BIGINT NOT NULL DEFAULT 0,
            allocated_tax   BIGINT NOT NULL DEFAULT 0,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_consolidated_entities_by
            ON {schema}.consolidated_entities(by_id, entity_code);

        CREATE TABLE IF NOT EXISTS {schema}.consolidation_eliminations (
            consolidation_elimination_id BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            elimination_type VARCHAR(50) NOT NULL,
            amount          BIGINT NOT NULL,
            direction       VARCHAR(20) NOT NULL,
            description     TEXT,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_consolidation_eliminations_by
            ON {schema}.consolidation_eliminations(by_id, elimination_type);

        CREATE TABLE IF NOT EXISTS {schema}.tax_agents (
            tax_agent_id BIGSERIAL PRIMARY KEY,
            customer_id  BIGINT NOT NULL REFERENCES {schema}.customers(customer_id),
            agent_name   VARCHAR(100) NOT NULL,
            agent_type   VARCHAR(30) NOT NULL DEFAULT 'TAX_ACCOUNTANT',
            email        VARCHAR(200),
            phone        VARCHAR(30),
            active       BOOLEAN NOT NULL DEFAULT TRUE,
            created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS {schema}.customer_users (
            customer_user_id BIGSERIAL PRIMARY KEY,
            customer_id      BIGINT NOT NULL REFERENCES {schema}.customers(customer_id),
            user_id          BIGINT NOT NULL REFERENCES public.users(user_id),
            relationship_type VARCHAR(30) NOT NULL DEFAULT 'STAFF',
            active           BOOLEAN NOT NULL DEFAULT TRUE,
            created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(customer_id, user_id)
        );

        CREATE TABLE IF NOT EXISTS {schema}.form_data (
            form_data_id    BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            form_code       VARCHAR(50) NOT NULL,
            form_version_id BIGINT NOT NULL REFERENCES public.form_versions(form_version_id),
            data_json       JSONB NOT NULL,
            snapshot_id     BIGINT REFERENCES {schema}.by_law_snapshot(snapshot_id),
            status          VARCHAR(20) NOT NULL DEFAULT 'GENERATED',
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(by_id, form_code)
        );

        CREATE TABLE IF NOT EXISTS {schema}.form_data_migration_history (
            migration_id    BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            form_code       VARCHAR(50) NOT NULL,
            from_version_id BIGINT,
            to_version_id   BIGINT NOT NULL,
            migrated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            result_json     JSONB NOT NULL DEFAULT '{{}}'::jsonb
        );

        CREATE TABLE IF NOT EXISTS {schema}.form_data_history (
            history_id      BIGSERIAL PRIMARY KEY,
            form_data_id    BIGINT NOT NULL REFERENCES {schema}.form_data(form_data_id) ON DELETE CASCADE,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            form_code       VARCHAR(50) NOT NULL,
            change_type     VARCHAR(30) NOT NULL,
            changed_by      VARCHAR(100) NOT NULL DEFAULT 'system',
            reason          TEXT,
            old_data        JSONB,
            new_data        JSONB NOT NULL,
            changed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_form_data_history_by
            ON {schema}.form_data_history(by_id, form_code, changed_at DESC);

        CREATE TABLE IF NOT EXISTS {schema}.efiling_history (
            efiling_id      BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            efile_master_id BIGINT NOT NULL REFERENCES public.efile_masters(efile_master_id),
            status          VARCHAR(20) NOT NULL DEFAULT 'GENERATED',
            total_records   INT NOT NULL DEFAULT 0,
            checksum        VARCHAR(80) NOT NULL DEFAULT '',
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            submitted_at    TIMESTAMPTZ
        );

        CREATE TABLE IF NOT EXISTS {schema}.efiling_files (
            file_id         BIGSERIAL PRIMARY KEY,
            efiling_id      BIGINT NOT NULL REFERENCES {schema}.efiling_history(efiling_id),
            file_name       VARCHAR(200) NOT NULL,
            encoding        VARCHAR(30) NOT NULL,
            contents        BYTEA NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_efiling_files_efiling ON {schema}.efiling_files(efiling_id);

        CREATE TABLE IF NOT EXISTS {schema}.efiling_validation (
            validation_id   BIGSERIAL PRIMARY KEY,
            efiling_id      BIGINT NOT NULL REFERENCES {schema}.efiling_history(efiling_id),
            validation_code VARCHAR(50) NOT NULL,
            severity        VARCHAR(20) NOT NULL,
            message         TEXT NOT NULL,
            field_path      VARCHAR(200),
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS {schema}.workflow_events (
            event_id    BIGSERIAL PRIMARY KEY,
            by_id       BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            from_status VARCHAR(30),
            to_status   VARCHAR(30) NOT NULL,
            action      VARCHAR(50) NOT NULL,
            actor       VARCHAR(100) NOT NULL DEFAULT 'system',
            comment     TEXT,
            metadata    JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_workflow_events_by
            ON {schema}.workflow_events(by_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS {schema}.approval_lines (
            line_id           BIGSERIAL PRIMARY KEY,
            by_id             BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            step_order        INT NOT NULL DEFAULT 1,
            approver_login_id VARCHAR(100) NOT NULL,
            status            VARCHAR(30) NOT NULL DEFAULT 'PENDING',
            acted_at          TIMESTAMPTZ,
            comment           TEXT,
            created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_approval_lines_by
            ON {schema}.approval_lines(by_id, step_order);

        CREATE TABLE IF NOT EXISTS {schema}.audit_logs (
            audit_id        BIGSERIAL PRIMARY KEY,
            table_name      VARCHAR(100) NOT NULL,
            record_id       VARCHAR(100) NOT NULL,
            action          VARCHAR(20) NOT NULL,
            old_data        JSONB,
            new_data        JSONB,
            changed_by      VARCHAR(100) NOT NULL DEFAULT 'system',
            changed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            event_date      DATE NOT NULL DEFAULT CURRENT_DATE,
            prev_hash       VARCHAR(64),
            hash_current    VARCHAR(64)
        );
        CREATE INDEX IF NOT EXISTS idx_audit_logs_event_date
            ON {schema}.audit_logs(event_date, audit_id);

        CREATE TABLE IF NOT EXISTS {schema}.notifications (
            notification_id BIGSERIAL PRIMARY KEY,
            by_id           BIGINT REFERENCES {schema}.business_years(by_id),
            title           VARCHAR(200) NOT NULL,
            message         TEXT NOT NULL,
            severity        VARCHAR(20) NOT NULL DEFAULT 'INFO',
            status          VARCHAR(20) NOT NULL DEFAULT 'UNREAD',
            metadata        JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            read_at         TIMESTAMPTZ
        );
        CREATE INDEX IF NOT EXISTS idx_notifications_status
            ON {schema}.notifications(status, created_at DESC);
        "#
    );
    execute_batch(pool, &sql).await
}

pub async fn create_customer(
    pool: &PgPool,
    tenant: &TenantRef,
    request: CreateCustomerRequest,
) -> Result<Customer> {
    let schema = quote_ident(&tenant.schema_name)?;
    let work_scopes = normalize_customer_work_scopes(request.work_scopes.as_deref())?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.customers (
            tenant_id, customer_code, customer_name, biz_reg_no, corp_reg_no, industry_code, is_sme, work_scopes
        )
        VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, FALSE), $8)
        RETURNING customer_id, tenant_id, customer_code, customer_name, biz_reg_no, corp_reg_no,
                  industry_code, is_sme, work_scopes, status, created_at, updated_at
        "#
    );

    let customer = sqlx::query_as::<_, Customer>(&sql)
        .bind(tenant.tenant_id)
        .bind(request.customer_code.trim())
        .bind(request.customer_name.trim())
        .bind(request.biz_reg_no.trim())
        .bind(request.corp_reg_no)
        .bind(request.industry_code)
        .bind(request.is_sme)
        .bind(work_scopes)
        .fetch_one(pool)
        .await
        .context("failed to create customer")?;
    insert_audit_log(
        pool,
        tenant,
        AuditLogEntry {
            table_name: "customers",
            record_id: customer.customer_id.to_string(),
            action: "CREATE",
            old_data: None,
            new_data: json!({
                "customer_code": customer.customer_code.clone(),
                "customer_name": customer.customer_name.clone(),
                "work_scopes": customer.work_scopes.clone()
            }),
            changed_by: "system",
        },
    )
    .await?;
    Ok(customer)
}

pub async fn list_customers(pool: &PgPool, tenant: &TenantRef) -> Result<Vec<Customer>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT customer_id, tenant_id, customer_code, customer_name, biz_reg_no, corp_reg_no,
               industry_code, is_sme, work_scopes, status, created_at, updated_at
        FROM {schema}.customers
        WHERE tenant_id = $1
        ORDER BY customer_code
        "#
    );

    sqlx::query_as::<_, Customer>(&sql)
        .bind(tenant.tenant_id)
        .fetch_all(pool)
        .await
        .context("failed to list customers")
}

fn normalize_customer_work_scopes(scopes: Option<&[String]>) -> Result<Vec<String>> {
    let mut normalized = scopes
        .filter(|items| !items.is_empty())
        .map(|items| {
            items
                .iter()
                .map(|scope| {
                    let normalized = scope.trim().to_ascii_uppercase();
                    if !ALLOWED_CUSTOMER_WORK_SCOPES.contains(&normalized.as_str()) {
                        anyhow::bail!("invalid customer work_scope");
                    }
                    Ok(normalized)
                })
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_else(|| {
            DEFAULT_CUSTOMER_WORK_SCOPES
                .iter()
                .map(|scope| (*scope).to_string())
                .collect()
        });
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

pub async fn create_business_year(
    pool: &PgPool,
    tenant: &TenantRef,
    request: CreateBusinessYearRequest,
) -> Result<BusinessYear> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.business_years (customer_id, year_label, start_date, end_date)
        VALUES ($1, $2, $3, $4)
        RETURNING by_id, customer_id, year_label, start_date, end_date, status,
                  locked_at, created_at, updated_at
        "#
    );

    let business_year = sqlx::query_as::<_, BusinessYear>(&sql)
        .bind(request.customer_id)
        .bind(request.year_label)
        .bind(request.start_date)
        .bind(request.end_date)
        .fetch_one(pool)
        .await
        .context("failed to create business year")?;
    insert_audit_log(
        pool,
        tenant,
        AuditLogEntry {
            table_name: "business_years",
            record_id: business_year.by_id.to_string(),
            action: "CREATE",
            old_data: None,
            new_data: json!({
                "customer_id": business_year.customer_id,
                "year_label": business_year.year_label,
                "status": business_year.status.clone()
            }),
            changed_by: "system",
        },
    )
    .await?;
    Ok(business_year)
}

pub async fn list_business_years(pool: &PgPool, tenant: &TenantRef) -> Result<Vec<BusinessYear>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT by_id, customer_id, year_label, start_date, end_date, status,
               locked_at, created_at, updated_at
        FROM {schema}.business_years
        ORDER BY year_label DESC, by_id DESC
        "#
    );

    sqlx::query_as::<_, BusinessYear>(&sql)
        .fetch_all(pool)
        .await
        .context("failed to list business years")
}

pub async fn update_business_year_status(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: UpdateBusinessYearStatusRequest,
) -> Result<BusinessYear> {
    let current = get_business_year(pool, tenant, by_id).await?;
    let next = normalize_business_year_status(&request.status)?;
    validate_business_year_status_transition(&current.status, &next)?;
    let actor = request.actor.as_deref().unwrap_or("system");
    let comment = request.comment.as_deref();
    let approver = request.approver.as_deref().unwrap_or(actor);
    let lock_on_file = next == "FILED";
    let unlock_for_amendment = next == "AMENDED";
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.business_years
        SET status = $2,
            locked_at = CASE
                WHEN $3 THEN COALESCE(locked_at, NOW())
                WHEN $4 THEN NULL
                ELSE locked_at
            END,
            updated_at = NOW()
        WHERE by_id = $1
        RETURNING by_id, customer_id, year_label, start_date, end_date, status,
                  locked_at, created_at, updated_at
        "#
    );

    let by = sqlx::query_as::<_, BusinessYear>(&sql)
        .bind(by_id)
        .bind(&next)
        .bind(lock_on_file)
        .bind(unlock_for_amendment)
        .fetch_one(pool)
        .await
        .context("failed to update business year status")?;
    record_workflow_transition(
        pool,
        tenant,
        by_id,
        &current.status,
        &by.status,
        actor,
        comment,
    )
    .await?;
    sync_approval_line(
        pool,
        tenant,
        by_id,
        &current.status,
        &by.status,
        approver,
        comment,
    )
    .await?;
    insert_audit_log(
        pool,
        tenant,
        AuditLogEntry {
            table_name: "business_years",
            record_id: by_id.to_string(),
            action: "UPDATE",
            old_data: Some(json!({ "status": current.status, "locked_at": current.locked_at })),
            new_data: json!({ "status": by.status.clone(), "locked_at": by.locked_at }),
            changed_by: actor,
        },
    )
    .await?;
    Ok(by)
}

async fn record_workflow_transition(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    from_status: &str,
    to_status: &str,
    actor: &str,
    comment: Option<&str>,
) -> Result<()> {
    if from_status == to_status {
        return Ok(());
    }
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.workflow_events (
            by_id, from_status, to_status, action, actor, comment, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#
    );
    sqlx::query(&sql)
        .bind(by_id)
        .bind(from_status)
        .bind(to_status)
        .bind(workflow_action(from_status, to_status))
        .bind(actor)
        .bind(comment)
        .bind(json!({ "from_status": from_status, "to_status": to_status }))
        .execute(pool)
        .await
        .context("failed to insert workflow event")?;
    Ok(())
}

async fn sync_approval_line(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    from_status: &str,
    to_status: &str,
    approver: &str,
    comment: Option<&str>,
) -> Result<()> {
    if from_status == to_status {
        return Ok(());
    }
    let schema = quote_ident(&tenant.schema_name)?;
    match to_status {
        "IN_REVIEW" => {
            let sql = format!(
                r#"
                INSERT INTO {schema}.approval_lines (
                    by_id, step_order, approver_login_id, status, comment
                )
                SELECT $1, 1, $2, 'PENDING', $3
                WHERE NOT EXISTS (
                    SELECT 1 FROM {schema}.approval_lines
                    WHERE by_id = $1 AND status = 'PENDING'
                )
                "#
            );
            sqlx::query(&sql)
                .bind(by_id)
                .bind(approver)
                .bind(comment)
                .execute(pool)
                .await
                .context("failed to create approval line")?;
        }
        "APPROVED" => {
            let update_sql = format!(
                r#"
                UPDATE {schema}.approval_lines
                SET status = 'APPROVED', approver_login_id = $2,
                    acted_at = NOW(), comment = $3
                WHERE by_id = $1 AND status = 'PENDING'
                "#
            );
            let updated = sqlx::query(&update_sql)
                .bind(by_id)
                .bind(approver)
                .bind(comment)
                .execute(pool)
                .await
                .context("failed to approve approval line")?;
            if updated.rows_affected() == 0 {
                let insert_sql = format!(
                    r#"
                    INSERT INTO {schema}.approval_lines (
                        by_id, step_order, approver_login_id, status, acted_at, comment
                    )
                    VALUES ($1, 1, $2, 'APPROVED', NOW(), $3)
                    "#
                );
                sqlx::query(&insert_sql)
                    .bind(by_id)
                    .bind(approver)
                    .bind(comment)
                    .execute(pool)
                    .await
                    .context("failed to insert approved approval line")?;
            }
        }
        "DRAFT" if from_status == "IN_REVIEW" => {
            let sql = format!(
                r#"
                UPDATE {schema}.approval_lines
                SET status = 'RETURNED', acted_at = NOW(), comment = $2
                WHERE by_id = $1 AND status = 'PENDING'
                "#
            );
            sqlx::query(&sql)
                .bind(by_id)
                .bind(comment)
                .execute(pool)
                .await
                .context("failed to return approval line")?;
        }
        _ => {}
    }
    Ok(())
}

fn workflow_action(from_status: &str, to_status: &str) -> &'static str {
    match (from_status, to_status) {
        ("DRAFT", "IN_REVIEW") => "SUBMIT_REVIEW",
        ("IN_REVIEW", "APPROVED") => "APPROVE",
        ("IN_REVIEW", "DRAFT") => "RETURN",
        ("APPROVED", "FILED") => "FILE",
        ("FILED", "AMENDED") => "START_AMENDMENT",
        ("AMENDED", "IN_REVIEW") => "RESUBMIT_AMENDMENT",
        _ => "STATUS_CHANGE",
    }
}

struct AuditLogEntry<'a> {
    table_name: &'a str,
    record_id: String,
    action: &'a str,
    old_data: Option<Value>,
    new_data: Value,
    changed_by: &'a str,
}

async fn insert_audit_log(
    pool: &PgPool,
    tenant: &TenantRef,
    entry: AuditLogEntry<'_>,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        WITH prev AS (
            SELECT hash_current
            FROM {schema}.audit_logs
            WHERE event_date = CURRENT_DATE
            ORDER BY audit_id DESC
            LIMIT 1
        )
        INSERT INTO {schema}.audit_logs (
            table_name, record_id, action, old_data, new_data, changed_by,
            event_date, prev_hash, hash_current
        )
        SELECT $1, $2, $3, $4, $5, $6, CURRENT_DATE,
               prev.hash_current,
               md5(COALESCE(prev.hash_current, '') || $1 || $2 || $3 ||
                   COALESCE($4::text, '') || COALESCE($5::text, '') || $6)
        FROM (SELECT 1) seed
        LEFT JOIN prev ON TRUE
        "#
    );
    sqlx::query(&sql)
        .bind(entry.table_name)
        .bind(&entry.record_id)
        .bind(entry.action)
        .bind(entry.old_data)
        .bind(entry.new_data)
        .bind(entry.changed_by)
        .execute(pool)
        .await
        .context("failed to insert audit log")?;
    Ok(())
}

pub async fn list_audit_logs(
    pool: &PgPool,
    tenant: &TenantRef,
    limit: i64,
) -> Result<Vec<AuditLog>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT audit_id, table_name, record_id, action, old_data, new_data,
               changed_by, changed_at, event_date, prev_hash, hash_current
        FROM {schema}.audit_logs
        ORDER BY audit_id DESC
        LIMIT $1
        "#
    );
    sqlx::query_as::<_, AuditLog>(&sql)
        .bind(limit.clamp(1, 200))
        .fetch_all(pool)
        .await
        .context("failed to list audit logs")
}

pub async fn ensure_due_notifications(pool: &PgPool, tenant: &TenantRef) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.notifications (by_id, title, message, severity, metadata)
        SELECT b.by_id,
               '사업연도 마감 D-30',
               CONCAT(b.year_label, ' 사업연도 마감일이 30일 이내입니다.'),
               'WARN',
               jsonb_build_object('by_id', b.by_id, 'end_date', b.end_date)
        FROM {schema}.business_years b
        WHERE b.status <> 'FILED'
          AND b.end_date BETWEEN CURRENT_DATE AND CURRENT_DATE + INTERVAL '30 days'
          AND NOT EXISTS (
              SELECT 1 FROM {schema}.notifications n
              WHERE n.by_id = b.by_id
                AND n.title = '사업연도 마감 D-30'
          )
        "#
    );
    sqlx::query(&sql)
        .execute(pool)
        .await
        .context("failed to ensure due notifications")?;
    Ok(())
}

pub async fn list_notifications(pool: &PgPool, tenant: &TenantRef) -> Result<Vec<Notification>> {
    ensure_due_notifications(pool, tenant).await?;
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT notification_id, by_id, title, message, severity, status,
               metadata, created_at, read_at
        FROM {schema}.notifications
        ORDER BY created_at DESC, notification_id DESC
        LIMIT 100
        "#
    );
    sqlx::query_as::<_, Notification>(&sql)
        .fetch_all(pool)
        .await
        .context("failed to list notifications")
}

pub async fn dashboard_summary(pool: &PgPool, tenant: &TenantRef) -> Result<DashboardSummary> {
    ensure_due_notifications(pool, tenant).await?;
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT
            (SELECT COUNT(*) FROM {schema}.customers) AS customer_count,
            (SELECT COUNT(*) FROM {schema}.business_years) AS business_year_count,
            (SELECT COUNT(*) FROM {schema}.business_years WHERE status = 'FILED') AS filed_count,
            (SELECT COUNT(*) FROM {schema}.business_years WHERE status = 'IN_REVIEW') AS pending_review_count,
            (SELECT COUNT(*) FROM {schema}.business_years
             WHERE status <> 'FILED'
               AND end_date BETWEEN CURRENT_DATE AND CURRENT_DATE + INTERVAL '30 days') AS due_soon_count,
            (SELECT COUNT(*) FROM {schema}.notifications WHERE status = 'UNREAD') AS unread_notifications,
            (SELECT COUNT(*) FROM {schema}.audit_logs) AS audit_log_count
        "#
    );
    let row = sqlx::query_as::<_, DashboardCounts>(&sql)
        .fetch_one(pool)
        .await
        .context("failed to load dashboard summary")?;
    Ok(DashboardSummary {
        tenant_code: tenant.tenant_code.clone(),
        customer_count: row.customer_count,
        business_year_count: row.business_year_count,
        filed_count: row.filed_count,
        pending_review_count: row.pending_review_count,
        due_soon_count: row.due_soon_count,
        unread_notifications: row.unread_notifications,
        audit_log_count: row.audit_log_count,
    })
}

#[derive(sqlx::FromRow)]
struct DashboardCounts {
    customer_count: i64,
    business_year_count: i64,
    filed_count: i64,
    pending_review_count: i64,
    due_soon_count: i64,
    unread_notifications: i64,
    audit_log_count: i64,
}

pub async fn tax_burden_report(
    pool: &PgPool,
    tenant: &TenantRef,
) -> Result<Vec<TaxBurdenReportRow>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT b.by_id, b.customer_id, b.year_label,
               COALESCE(MAX(a.amount) FILTER (WHERE a.adj_code = 'TAXABLE_INCOME'), 0)::BIGINT AS taxable_income,
               COALESCE(MAX(a.amount) FILTER (WHERE a.adj_code = 'TOTAL_TAX_DUE'), 0)::BIGINT AS total_tax_due,
               CASE
                   WHEN COALESCE(MAX(a.amount) FILTER (WHERE a.adj_code = 'TAXABLE_INCOME'), 0) = 0 THEN 0
                   ELSE (
                       COALESCE(MAX(a.amount) FILTER (WHERE a.adj_code = 'TOTAL_TAX_DUE'), 0) * 10000
                       / COALESCE(MAX(a.amount) FILTER (WHERE a.adj_code = 'TAXABLE_INCOME'), 1)
                   )::BIGINT
               END AS effective_tax_rate_bps
        FROM {schema}.business_years b
        LEFT JOIN {schema}.tax_adjustments a ON a.by_id = b.by_id
        GROUP BY b.by_id, b.customer_id, b.year_label
        ORDER BY b.year_label DESC, b.by_id DESC
        "#
    );
    sqlx::query_as::<_, TaxBurdenReportRow>(&sql)
        .fetch_all(pool)
        .await
        .context("failed to load tax burden report")
}

pub async fn year_comparison_report(
    pool: &PgPool,
    tenant: &TenantRef,
) -> Result<Vec<YearComparisonReportRow>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT b.customer_id, b.year_label, b.status,
               COALESCE(a.total_adjustment_amount, 0)::BIGINT AS total_adjustment_amount,
               COALESCE(r.reserve_count, 0)::BIGINT AS reserve_count
        FROM {schema}.business_years b
        LEFT JOIN (
            SELECT by_id, SUM(amount)::BIGINT AS total_adjustment_amount
            FROM {schema}.tax_adjustments
            GROUP BY by_id
        ) a ON a.by_id = b.by_id
        LEFT JOIN (
            SELECT by_id, COUNT(*)::BIGINT AS reserve_count
            FROM {schema}.reserves
            GROUP BY by_id
        ) r ON r.by_id = b.by_id
        ORDER BY b.customer_id, b.year_label DESC
        "#
    );
    sqlx::query_as::<_, YearComparisonReportRow>(&sql)
        .fetch_all(pool)
        .await
        .context("failed to load year comparison report")
}

pub async fn get_business_year_workflow(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<BusinessYearWorkflow> {
    Ok(BusinessYearWorkflow {
        business_year: get_business_year(pool, tenant, by_id).await?,
        events: list_workflow_events(pool, tenant, by_id).await?,
        approval_lines: list_approval_lines(pool, tenant, by_id).await?,
    })
}

pub async fn list_workflow_events(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<WorkflowEvent>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT event_id, by_id, from_status, to_status, action, actor,
               comment, metadata, created_at
        FROM {schema}.workflow_events
        WHERE by_id = $1
        ORDER BY created_at DESC, event_id DESC
        "#
    );
    sqlx::query_as::<_, WorkflowEvent>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list workflow events")
}

pub async fn list_approval_lines(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<ApprovalLine>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT line_id, by_id, step_order, approver_login_id, status,
               acted_at, comment, created_at
        FROM {schema}.approval_lines
        WHERE by_id = $1
        ORDER BY step_order, line_id
        "#
    );
    sqlx::query_as::<_, ApprovalLine>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list approval lines")
}

pub async fn preview_amendment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<AmendmentPreview> {
    let business_year = get_business_year(pool, tenant, by_id).await?;
    let events = list_workflow_events(pool, tenant, by_id).await?;
    let filed_event = events.iter().find(|event| event.to_status == "FILED");
    let differences = vec![
        AmendmentDiff {
            area: "BUSINESS_YEAR".to_string(),
            field: "status".to_string(),
            original_value: json!(filed_event
                .map(|event| event.to_status.as_str())
                .unwrap_or("FILED")),
            current_value: json!(business_year.status.clone()),
            description: "원 신고 상태와 현재 수정신고 진행 상태".to_string(),
        },
        AmendmentDiff {
            area: "LOCK".to_string(),
            field: "locked_at".to_string(),
            original_value: filed_event
                .map(|event| json!(event.created_at))
                .unwrap_or(Value::Null),
            current_value: business_year
                .locked_at
                .map(|locked_at| json!(locked_at))
                .unwrap_or(Value::Null),
            description: "수정신고 진입 시 작업 잠금 해제 여부".to_string(),
        },
    ];
    Ok(AmendmentPreview {
        tenant_code: tenant.tenant_code.clone(),
        by_id,
        current_status: business_year.status.clone(),
        locked: business_year.locked_at.is_some(),
        differences,
    })
}

pub async fn get_business_year(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<BusinessYear> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT by_id, customer_id, year_label, start_date, end_date, status,
               locked_at, created_at, updated_at
        FROM {schema}.business_years
        WHERE by_id = $1
        "#
    );

    sqlx::query_as::<_, BusinessYear>(&sql)
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("business year not found")
}

fn normalize_business_year_status(status: &str) -> Result<String> {
    let status = status.trim().to_ascii_uppercase();
    let allowed = ["DRAFT", "IN_REVIEW", "APPROVED", "FILED", "AMENDED"];
    if !allowed.contains(&status.as_str()) {
        anyhow::bail!("invalid business year status");
    }
    Ok(status)
}

fn validate_business_year_status_transition(current: &str, next: &str) -> Result<()> {
    if current == next {
        return Ok(());
    }
    let allowed = match current {
        "DRAFT" => matches!(next, "IN_REVIEW"),
        "IN_REVIEW" => matches!(next, "APPROVED" | "DRAFT"),
        "APPROVED" => matches!(next, "FILED" | "IN_REVIEW"),
        "FILED" => matches!(next, "AMENDED"),
        "AMENDED" => matches!(next, "IN_REVIEW"),
        _ => false,
    };
    if allowed {
        Ok(())
    } else {
        anyhow::bail!("invalid business year status transition: {current} -> {next}");
    }
}
