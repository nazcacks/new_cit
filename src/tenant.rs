use anyhow::{Context, Result};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use crate::{
    db::{execute_batch, quote_ident},
    domain::{
        AmendmentDiff, AmendmentPreview, ApprovalLine, AuditLog, BusinessYear,
        BusinessYearWorkflow, CreateBusinessYearRequest, CreateCustomerRequest,
        CreateTenantRequest, CreateUserReportDefinitionRequest, Customer, DashboardSummary,
        Notification, ReserveTrendReportRow, TaxBurdenReportRow, Tenant, TenantRef,
        UnlockBusinessYearRequest, UpdateBusinessYearStatusRequest, UpdateNotificationRequest,
        WorkflowEvent, WorkflowEventRequest, WorkflowQueueItem, YearComparisonReportRow,
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

    if let Err(error) = provision_tenant_schema(pool, &tenant.schema_name).await {
        cleanup_failed_tenant(pool, tenant.tenant_id, &tenant.schema_name).await?;
        return Err(error).context("failed to provision tenant schema");
    }
    Ok(tenant)
}

async fn cleanup_failed_tenant(pool: &PgPool, tenant_id: i64, schema_name: &str) -> Result<()> {
    let schema = quote_ident(schema_name)?;
    let sql = format!("DROP SCHEMA IF EXISTS {schema} CASCADE");
    let _ = sqlx::query(&sql).execute(pool).await;
    let _ = sqlx::query("DELETE FROM tenants WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await;
    Ok(())
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
            lock_mode       VARCHAR(30) NOT NULL DEFAULT 'OPEN',
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CHECK (start_date <= end_date),
            UNIQUE(customer_id, year_label)
        );
        ALTER TABLE {schema}.business_years
            ALTER COLUMN status SET DEFAULT 'DRAFT';
        ALTER TABLE {schema}.business_years
            ADD COLUMN IF NOT EXISTS lock_mode VARCHAR(30) NOT NULL DEFAULT 'OPEN';
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

        CREATE TABLE IF NOT EXISTS {schema}.adjustment_items_history (
            history_id      BIGSERIAL PRIMARY KEY,
            adjustment_item_id BIGINT,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            source_module   VARCHAR(50) NOT NULL,
            action          VARCHAR(30) NOT NULL,
            old_data        JSONB,
            new_data        JSONB,
            changed_by      VARCHAR(100) NOT NULL DEFAULT 'system',
            changed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_adjustment_items_history_by
            ON {schema}.adjustment_items_history(by_id, source_module, changed_at DESC);

        CREATE TABLE IF NOT EXISTS {schema}.adjustment_item_attachments (
            attachment_id   BIGSERIAL PRIMARY KEY,
            adjustment_item_id BIGINT NOT NULL REFERENCES {schema}.adjustment_items(adjustment_item_id) ON DELETE CASCADE,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            file_name       VARCHAR(255) NOT NULL,
            content_type    VARCHAR(100) NOT NULL DEFAULT 'application/octet-stream',
            storage_url     TEXT,
            memo            TEXT,
            uploaded_by     VARCHAR(100) NOT NULL DEFAULT 'system',
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_adjustment_item_attachments_by
            ON {schema}.adjustment_item_attachments(by_id, adjustment_item_id, created_at DESC);

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

        CREATE TABLE IF NOT EXISTS {schema}.print_history (
            print_id        BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            form_code       VARCHAR(50),
            file_name       VARCHAR(255) NOT NULL,
            content_type    VARCHAR(100) NOT NULL,
            watermark       VARCHAR(40) NOT NULL,
            status          VARCHAR(20) NOT NULL DEFAULT 'GENERATED',
            printed_by      VARCHAR(100) NOT NULL DEFAULT 'system',
            metadata        JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_print_history_by
            ON {schema}.print_history(by_id, created_at DESC);

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

        CREATE TABLE IF NOT EXISTS {schema}.validation_issues (
            issue_id      BIGSERIAL PRIMARY KEY,
            by_id         BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            rule_code     VARCHAR(80) NOT NULL REFERENCES public.validation_rules(rule_code),
            severity      VARCHAR(20) NOT NULL,
            area          VARCHAR(40) NOT NULL,
            message       TEXT NOT NULL,
            target_path   VARCHAR(200),
            status        VARCHAR(20) NOT NULL DEFAULT 'OPEN',
            metadata      JSONB NOT NULL DEFAULT '{{}}'::jsonb,
            created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            dismissed_at  TIMESTAMPTZ
        );
        CREATE INDEX IF NOT EXISTS idx_validation_issues_by
            ON {schema}.validation_issues(by_id, status, severity, created_at DESC);
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
                  locked_at, lock_mode, created_at, updated_at
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
               locked_at, lock_mode, created_at, updated_at
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
    let requested_next = normalize_business_year_status(&request.status)?;
    let pending_count = if current.status == "IN_REVIEW" && requested_next == "APPROVED" {
        pending_approval_count(pool, tenant, by_id).await?
    } else {
        0
    };
    let partial_approval =
        current.status == "IN_REVIEW" && requested_next == "APPROVED" && pending_count > 1;
    let next = if partial_approval {
        "IN_REVIEW".to_string()
    } else {
        requested_next.clone()
    };
    validate_business_year_status_transition(&current.status, &next)?;
    let actor = request.actor.as_deref().unwrap_or("system");
    let comment = request.comment.as_deref();
    let approver = request.approver.as_deref().unwrap_or(actor);
    let approvers = request
        .approvers
        .as_deref()
        .filter(|items| !items.is_empty())
        .map(|items| items.iter().map(|item| item.as_str()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec![approver]);
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
            lock_mode = CASE
                WHEN $3 THEN 'FILED_LOCK'
                WHEN $4 THEN 'AMENDMENT_UNLOCK'
                ELSE lock_mode
            END,
            updated_at = NOW()
        WHERE by_id = $1
        RETURNING by_id, customer_id, year_label, start_date, end_date, status,
                  locked_at, lock_mode, created_at, updated_at
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
        ApprovalLineSync {
            by_id,
            from_status: &current.status,
            requested_status: &requested_next,
            approvers: &approvers,
            approver,
            comment,
        },
    )
    .await?;
    if partial_approval {
        append_workflow_event(
            pool,
            tenant,
            by_id,
            WorkflowEventRequest {
                action: Some("APPROVE_STEP".to_string()),
                actor: Some(actor.to_string()),
                comment: comment.map(ToString::to_string),
                to_status: Some("IN_REVIEW".to_string()),
                metadata: Some(json!({
                    "approved_by": approver,
                    "remaining_pending": pending_count - 1,
                    "next_step": "wait_for_next_approver"
                })),
            },
        )
        .await?;
    }
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

struct ApprovalLineSync<'a> {
    by_id: i64,
    from_status: &'a str,
    requested_status: &'a str,
    approvers: &'a [&'a str],
    approver: &'a str,
    comment: Option<&'a str>,
}

async fn pending_approval_count(pool: &PgPool, tenant: &TenantRef, by_id: i64) -> Result<i64> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query_scalar::<_, i64>(&format!(
        "SELECT COUNT(*) FROM {schema}.approval_lines WHERE by_id = $1 AND status = 'PENDING'"
    ))
    .bind(by_id)
    .fetch_one(pool)
    .await
    .context("failed to count pending approval lines")
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
        .bind(json!({
            "from_status": from_status,
            "to_status": to_status,
            "next_step": next_step_for_status(to_status)
        }))
        .execute(pool)
        .await
        .context("failed to insert workflow event")?;
    insert_workflow_notification(pool, tenant, by_id, to_status, actor).await?;
    Ok(())
}

async fn sync_approval_line(
    pool: &PgPool,
    tenant: &TenantRef,
    sync: ApprovalLineSync<'_>,
) -> Result<()> {
    let ApprovalLineSync {
        by_id,
        from_status,
        requested_status,
        approvers,
        approver,
        comment,
    } = sync;
    if from_status == requested_status && requested_status != "APPROVED" {
        return Ok(());
    }
    let schema = quote_ident(&tenant.schema_name)?;
    match requested_status {
        "IN_REVIEW" => {
            let clear_sql = format!(
                "DELETE FROM {schema}.approval_lines WHERE by_id = $1 AND status = 'PENDING'"
            );
            sqlx::query(&clear_sql)
                .bind(by_id)
                .execute(pool)
                .await
                .context("failed to clear pending approval lines")?;
            let sql = format!(
                r#"
                INSERT INTO {schema}.approval_lines (
                    by_id, step_order, approver_login_id, status, comment
                )
                VALUES ($1, $2, $3, 'PENDING', $4)
                "#
            );
            for (index, line_approver) in approvers.iter().enumerate() {
                sqlx::query(&sql)
                    .bind(by_id)
                    .bind((index + 1) as i32)
                    .bind(line_approver)
                    .bind(comment)
                    .execute(pool)
                    .await
                    .context("failed to create approval line")?;
            }
        }
        "APPROVED" => {
            let update_sql = format!(
                r#"
                UPDATE {schema}.approval_lines
                SET status = 'APPROVED', approver_login_id = $2,
                    acted_at = NOW(), comment = $3
                WHERE line_id = (
                    SELECT line_id
                    FROM {schema}.approval_lines
                    WHERE by_id = $1 AND status = 'PENDING'
                    ORDER BY step_order, line_id
                    LIMIT 1
                )
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

async fn insert_workflow_notification(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    to_status: &str,
    actor: &str,
) -> Result<()> {
    let (title, message, severity) = match to_status {
        "IN_REVIEW" => (
            "Approval requested",
            "A business year is waiting for approval",
            "INFO",
        ),
        "APPROVED" => (
            "Approval completed",
            "All approval lines are approved",
            "INFO",
        ),
        "DRAFT" => (
            "Approval returned",
            "Approval was returned to draft",
            "WARN",
        ),
        "FILED" => (
            "Filing completed",
            "The business year has been filed and locked",
            "INFO",
        ),
        "AMENDED" => (
            "Amendment opened",
            "The filed business year was unlocked for amendment",
            "WARN",
        ),
        _ => return Ok(()),
    };
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.notifications (by_id, title, message, severity, metadata)
        VALUES ($1, $2, $3, $4, $5)
        "#
    );
    sqlx::query(&sql)
        .bind(by_id)
        .bind(title)
        .bind(message)
        .bind(severity)
        .bind(json!({ "workflow_status": to_status, "actor": actor }))
        .execute(pool)
        .await
        .context("failed to insert workflow notification")?;
    Ok(())
}

fn next_step_for_status(status: &str) -> &'static str {
    match status {
        "DRAFT" => "review_rework",
        "IN_REVIEW" => "approval",
        "APPROVED" => "efiling",
        "FILED" => "post_filing",
        "AMENDED" => "amendment_rework",
        _ => "review",
    }
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

pub async fn ensure_due_notifications(pool: &PgPool, tenant: &TenantRef) -> Result<i64> {
    let schema = quote_ident(&tenant.schema_name)?;
    let mut created = 0_i64;
    for (bucket, days, severity) in [("D-30", 30_i64, "WARN"), ("D-7", 7_i64, "ERROR")] {
        let title = format!("사업연도 마감 {bucket}");
        let message_suffix = format!(" 사업연도 마감일이 {days}일 이내입니다.");
        sqlx::query(&format!(
            r#"
            UPDATE {schema}.notifications
            SET title = $2,
                message = CONCAT(COALESCE(metadata->>'year_label', ''), $3)
            WHERE metadata->>'due_bucket' = $1::TEXT
              AND title LIKE 'Business year due%'
            "#
        ))
        .bind(bucket)
        .bind(&title)
        .bind(&message_suffix)
        .execute(pool)
        .await
        .context("failed to normalize due notifications")?;
        let sql = format!(
            r#"
            INSERT INTO {schema}.notifications (by_id, title, message, severity, metadata)
            SELECT b.by_id,
                   $4,
                   CONCAT(b.year_label, $5),
                   $3,
                   jsonb_build_object('by_id', b.by_id, 'year_label', b.year_label, 'end_date', b.end_date, 'due_bucket', $1::TEXT)
            FROM {schema}.business_years b
            WHERE b.status <> 'FILED'
              AND b.end_date BETWEEN CURRENT_DATE AND CURRENT_DATE + ($2::INT * INTERVAL '1 day')
              AND NOT EXISTS (
                  SELECT 1 FROM {schema}.notifications n
                  WHERE n.by_id = b.by_id
                    AND n.metadata->>'due_bucket' = $1::TEXT
              )
            "#
        );
        created += sqlx::query(&sql)
            .bind(bucket)
            .bind(days as i32)
            .bind(severity)
            .bind(&title)
            .bind(&message_suffix)
            .execute(pool)
            .await
            .context("failed to ensure due notifications")?
            .rows_affected() as i64;
    }
    Ok(created)
}

#[allow(dead_code)]
async fn ensure_due_notifications_legacy(pool: &PgPool, tenant: &TenantRef) -> Result<()> {
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

pub async fn update_notification(
    pool: &PgPool,
    tenant: &TenantRef,
    notification_id: i64,
    request: UpdateNotificationRequest,
) -> Result<Notification> {
    let status = request
        .status
        .as_deref()
        .unwrap_or("READ")
        .trim()
        .to_ascii_uppercase();
    if !matches!(status.as_str(), "READ" | "UNREAD" | "ARCHIVED") {
        anyhow::bail!("invalid notification status");
    }
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.notifications
        SET status = $2,
            read_at = CASE
                WHEN $2 = 'READ' THEN COALESCE(read_at, NOW())
                WHEN $2 = 'UNREAD' THEN NULL
                ELSE read_at
            END
        WHERE notification_id = $1
        RETURNING notification_id, by_id, title, message, severity, status,
                  metadata, created_at, read_at
        "#
    );
    let notification = sqlx::query_as::<_, Notification>(&sql)
        .bind(notification_id)
        .bind(&status)
        .fetch_one(pool)
        .await
        .context("notification not found")?;
    insert_audit_log(
        pool,
        tenant,
        AuditLogEntry {
            table_name: "notifications",
            record_id: notification_id.to_string(),
            action: "UPDATE",
            old_data: None,
            new_data: json!({ "status": notification.status.clone() }),
            changed_by: "system",
        },
    )
    .await?;
    Ok(notification)
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

pub async fn reserve_trend_report(
    pool: &PgPool,
    tenant: &TenantRef,
) -> Result<Vec<ReserveTrendReportRow>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT b.customer_id, b.year_label, r.reserve_code, r.direction,
               SUM(r.amount)::BIGINT AS amount
        FROM {schema}.business_years b
        JOIN {schema}.reserves r ON r.by_id = b.by_id
        GROUP BY b.customer_id, b.year_label, r.reserve_code, r.direction
        ORDER BY b.customer_id, r.reserve_code, b.year_label
        "#
    );
    sqlx::query_as::<_, ReserveTrendReportRow>(&sql)
        .fetch_all(pool)
        .await
        .context("failed to load reserve trend report")
}

pub async fn loss_expiry_report(pool: &PgPool, tenant: &TenantRef) -> Result<Vec<Value>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT c.customer_id, c.customer_code, c.customer_name,
               l.loss_id, l.origin_year, l.original_amount, l.used_amount,
               l.expired_amount, l.remaining_amount, l.expires_year,
               (l.expires_year - EXTRACT(YEAR FROM CURRENT_DATE)::INT) AS years_until_expiry
        FROM {schema}.carryforward_loss l
        JOIN {schema}.customers c ON c.customer_id = l.customer_id
        WHERE l.remaining_amount > 0
        ORDER BY l.expires_year, c.customer_code, l.origin_year
        "#
    );
    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .context("failed to load loss expiry report")?;
    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "customer_id": row.get::<i64, _>("customer_id"),
                "customer_code": row.get::<String, _>("customer_code"),
                "customer_name": row.get::<String, _>("customer_name"),
                "loss_id": row.get::<i64, _>("loss_id"),
                "origin_year": row.get::<i32, _>("origin_year"),
                "original_amount": row.get::<i64, _>("original_amount"),
                "used_amount": row.get::<i64, _>("used_amount"),
                "expired_amount": row.get::<i64, _>("expired_amount"),
                "remaining_amount": row.get::<i64, _>("remaining_amount"),
                "expires_year": row.get::<i32, _>("expires_year"),
                "years_until_expiry": row.get::<i32, _>("years_until_expiry")
            })
        })
        .collect())
}

pub async fn industry_statistics_report(pool: &PgPool, tenant: &TenantRef) -> Result<Vec<Value>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT COALESCE(c.industry_code, 'UNSPECIFIED') AS industry_code,
               c.is_sme,
               COUNT(DISTINCT c.customer_id)::BIGINT AS customer_count,
               COUNT(DISTINCT b.by_id)::BIGINT AS business_year_count,
               COALESCE(SUM(a.amount) FILTER (WHERE a.adj_code = 'TOTAL_TAX_DUE'), 0)::BIGINT AS total_tax_due,
               COALESCE(AVG(a.amount) FILTER (WHERE a.adj_code = 'TOTAL_TAX_DUE'), 0)::BIGINT AS average_tax_due
        FROM {schema}.customers c
        LEFT JOIN {schema}.business_years b ON b.customer_id = c.customer_id
        LEFT JOIN {schema}.tax_adjustments a ON a.by_id = b.by_id
        GROUP BY COALESCE(c.industry_code, 'UNSPECIFIED'), c.is_sme
        ORDER BY industry_code, c.is_sme DESC
        "#
    );
    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .context("failed to load industry statistics report")?;
    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "industry_code": row.get::<String, _>("industry_code"),
                "is_sme": row.get::<bool, _>("is_sme"),
                "customer_count": row.get::<i64, _>("customer_count"),
                "business_year_count": row.get::<i64, _>("business_year_count"),
                "total_tax_due": row.get::<i64, _>("total_tax_due"),
                "average_tax_due": row.get::<i64, _>("average_tax_due")
            })
        })
        .collect())
}

pub async fn list_user_report_definitions(pool: &PgPool, tenant: &TenantRef) -> Result<Vec<Value>> {
    let rows = sqlx::query(
        r#"
        SELECT report_id, tenant_id, user_id, report_name, source, columns,
               filters, active, created_at, updated_at
        FROM user_report_definitions
        WHERE tenant_id = $1 AND active = TRUE
        ORDER BY updated_at DESC, report_id DESC
        "#,
    )
    .bind(tenant.tenant_id)
    .fetch_all(pool)
    .await
    .context("failed to list user report definitions")?;
    Ok(rows.into_iter().map(user_report_json).collect())
}

pub async fn create_user_report_definition(
    pool: &PgPool,
    tenant: &TenantRef,
    user_id: i64,
    request: CreateUserReportDefinitionRequest,
) -> Result<Value> {
    let report_name = request.report_name.trim();
    let source = request.source.trim().to_ascii_uppercase();
    if report_name.is_empty() || source.is_empty() {
        anyhow::bail!("invalid user report definition");
    }
    if !matches!(
        source.as_str(),
        "TAX_BURDEN" | "YEAR_COMPARISON" | "RESERVE_TREND" | "LOSS_EXPIRY" | "INDUSTRY"
    ) {
        anyhow::bail!("unsupported user report source");
    }
    let columns = json!(request.columns.unwrap_or_default());
    let filters = request.filters.unwrap_or_else(|| json!({}));
    let row = sqlx::query(
        r#"
        INSERT INTO user_report_definitions (
            tenant_id, user_id, report_name, source, columns, filters
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING report_id, tenant_id, user_id, report_name, source, columns,
                  filters, active, created_at, updated_at
        "#,
    )
    .bind(tenant.tenant_id)
    .bind(user_id)
    .bind(report_name)
    .bind(source)
    .bind(columns)
    .bind(filters)
    .fetch_one(pool)
    .await
    .context("failed to create user report definition")?;
    Ok(user_report_json(row))
}

pub async fn run_user_report(pool: &PgPool, tenant: &TenantRef, report_id: i64) -> Result<Value> {
    let report = sqlx::query(
        r#"
        SELECT report_id, report_name, source, columns, filters
        FROM user_report_definitions
        WHERE tenant_id = $1 AND report_id = $2 AND active = TRUE
        "#,
    )
    .bind(tenant.tenant_id)
    .bind(report_id)
    .fetch_one(pool)
    .await
    .context("user report definition not found")?;
    let source = report.get::<String, _>("source");
    let rows = match source.as_str() {
        "TAX_BURDEN" => tax_burden_report(pool, tenant)
            .await?
            .into_iter()
            .map(|row| json!(row))
            .collect::<Vec<_>>(),
        "YEAR_COMPARISON" => year_comparison_report(pool, tenant)
            .await?
            .into_iter()
            .map(|row| json!(row))
            .collect::<Vec<_>>(),
        "RESERVE_TREND" => reserve_trend_report(pool, tenant)
            .await?
            .into_iter()
            .map(|row| json!(row))
            .collect::<Vec<_>>(),
        "LOSS_EXPIRY" => loss_expiry_report(pool, tenant).await?,
        "INDUSTRY" => industry_statistics_report(pool, tenant).await?,
        _ => Vec::new(),
    };
    Ok(json!({
        "report_id": report.get::<i64, _>("report_id"),
        "report_name": report.get::<String, _>("report_name"),
        "source": source,
        "columns": report.get::<Value, _>("columns"),
        "filters": report.get::<Value, _>("filters"),
        "rows": rows
    }))
}

fn user_report_json(row: sqlx::postgres::PgRow) -> Value {
    json!({
        "report_id": row.get::<i64, _>("report_id"),
        "tenant_id": row.get::<i64, _>("tenant_id"),
        "user_id": row.get::<Option<i64>, _>("user_id"),
        "report_name": row.get::<String, _>("report_name"),
        "source": row.get::<String, _>("source"),
        "columns": row.get::<Value, _>("columns"),
        "filters": row.get::<Value, _>("filters"),
        "active": row.get::<bool, _>("active"),
        "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
        "updated_at": row.get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
    })
}

pub async fn workflow_queue(
    pool: &PgPool,
    tenant: &TenantRef,
    assignee: Option<&str>,
) -> Result<Vec<WorkflowQueueItem>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let assignee = assignee.filter(|value| !value.eq_ignore_ascii_case("me"));
    let sql = format!(
        r#"
        SELECT b.by_id,
               b.customer_id,
               c.customer_name,
               b.year_label,
               b.status,
               al.approver_login_id,
               we.created_at AS submitted_at,
               GREATEST(0, FLOOR(EXTRACT(EPOCH FROM (NOW() - COALESCE(we.created_at, b.updated_at))) / 86400))::BIGINT AS pending_days
        FROM {schema}.business_years b
        JOIN {schema}.customers c ON c.customer_id = b.customer_id
        LEFT JOIN LATERAL (
            SELECT approver_login_id
            FROM {schema}.approval_lines
            WHERE by_id = b.by_id AND status = 'PENDING'
            ORDER BY step_order, line_id
            LIMIT 1
        ) al ON TRUE
        LEFT JOIN LATERAL (
            SELECT created_at
            FROM {schema}.workflow_events
            WHERE by_id = b.by_id AND to_status = 'IN_REVIEW'
            ORDER BY created_at DESC, event_id DESC
            LIMIT 1
        ) we ON TRUE
        WHERE b.status = 'IN_REVIEW'
          AND ($1::TEXT IS NULL OR al.approver_login_id = $1)
        ORDER BY pending_days DESC, b.updated_at DESC
        "#
    );
    sqlx::query_as::<_, WorkflowQueueItem>(&sql)
        .bind(assignee)
        .fetch_all(pool)
        .await
        .context("failed to load workflow queue")
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

pub async fn append_workflow_event(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: WorkflowEventRequest,
) -> Result<WorkflowEvent> {
    let business_year = get_business_year(pool, tenant, by_id).await?;
    let action = request
        .action
        .as_deref()
        .unwrap_or("COMMENT")
        .trim()
        .to_ascii_uppercase();
    let to_status = request
        .to_status
        .as_deref()
        .unwrap_or(&business_year.status)
        .trim()
        .to_ascii_uppercase();
    let actor = request.actor.as_deref().unwrap_or("system");
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.workflow_events (
            by_id, from_status, to_status, action, actor, comment, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING event_id, by_id, from_status, to_status, action, actor,
                  comment, metadata, created_at
        "#
    );
    let event = sqlx::query_as::<_, WorkflowEvent>(&sql)
        .bind(by_id)
        .bind(&business_year.status)
        .bind(to_status)
        .bind(action)
        .bind(actor)
        .bind(request.comment)
        .bind(request.metadata.unwrap_or_else(|| json!({})))
        .fetch_one(pool)
        .await
        .context("failed to append workflow event")?;
    Ok(event)
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

pub async fn unlock_business_year(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: UnlockBusinessYearRequest,
) -> Result<BusinessYear> {
    let current = get_business_year(pool, tenant, by_id).await?;
    let actor = request.actor.unwrap_or_else(|| "system".to_string());
    let reason = request
        .reason
        .unwrap_or_else(|| "amendment unlock".to_string());
    let version_mode = request
        .version_mode
        .unwrap_or_else(|| "CURRENT".to_string());

    if current.status == "FILED" {
        return update_business_year_status(
            pool,
            tenant,
            by_id,
            UpdateBusinessYearStatusRequest {
                status: "AMENDED".to_string(),
                actor: Some(actor),
                approver: None,
                approvers: None,
                comment: Some(format!("{reason}; version_mode={version_mode}")),
            },
        )
        .await;
    }

    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.business_years
            SET status = 'AMENDED',
                locked_at = NULL,
                lock_mode = 'AMENDMENT_UNLOCK',
                updated_at = NOW()
        WHERE by_id = $1
        RETURNING by_id, customer_id, year_label, start_date, end_date, status,
                  locked_at, lock_mode, created_at, updated_at
        "#
    );
    let by = sqlx::query_as::<_, BusinessYear>(&sql)
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("failed to unlock business year")?;
    append_workflow_event(
        pool,
        tenant,
        by_id,
        WorkflowEventRequest {
            action: Some("UNLOCK".to_string()),
            actor: Some(actor.clone()),
            comment: Some(reason),
            to_status: Some("AMENDED".to_string()),
            metadata: Some(json!({ "version_mode": version_mode })),
        },
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
            changed_by: &actor,
        },
    )
    .await?;
    Ok(by)
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
               locked_at, lock_mode, created_at, updated_at
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

pub async fn ensure_business_year_editable(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    area: &str,
) -> Result<()> {
    let by = get_business_year(pool, tenant, by_id).await?;
    if by.status == "FILED" || by.locked_at.is_some() {
        anyhow::bail!(
            "business year is locked after FILED status; {area} edits are blocked until amendment unlock"
        );
    }
    Ok(())
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
