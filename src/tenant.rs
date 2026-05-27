use std::collections::HashMap;

use anyhow::{Context, Result};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use crate::{
    db::{execute_batch, quote_ident},
    domain::{
        AmendmentDiff, AmendmentPreview, ApprovalLine, AuditLog, AuthUser, BusinessYear,
        BusinessYearWorkflow, CreateBusinessYearRequest, CreateCustomerRequest,
        CreateTenantRequest, CreateUserReportDefinitionRequest, Customer, DashboardFilingDeadline,
        DashboardFilingDeadlineSummary, DashboardIndustryDistributionItem,
        DashboardIndustryDistributionSummary, DashboardLossExpiryKpiBucket,
        DashboardLossExpiryKpiSummary, DashboardNotificationItem, DashboardNotificationSummary,
        DashboardRecentActivityItem, DashboardRecentActivitySummary, DashboardSummary,
        DashboardTaxBurdenKpiPoint, DashboardTaxBurdenKpiSummary, DashboardWorkStatus,
        Notification, ReserveTrendReportRow, TaxBurdenReportRow, Tenant, TenantRef,
        UnlockBusinessYearRequest, UpdateBusinessYearStatusRequest, UpdateNotificationRequest,
        UpdateTenantPlanRequest, UpdateTenantStatusRequest, WorkflowEvent, WorkflowEventRequest,
        WorkflowQueueItem, YearComparisonReportRow,
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

fn normalize_tenant_status(status: &str) -> Result<String> {
    let normalized = status.trim().to_ascii_uppercase();
    if matches!(normalized.as_str(), "ACTIVE" | "SUSPENDED" | "CLOSED") {
        Ok(normalized)
    } else {
        anyhow::bail!("invalid tenant status");
    }
}

fn normalize_tenant_plan(plan: &str) -> Result<String> {
    let normalized = plan.trim().to_ascii_uppercase();
    if matches!(
        normalized.as_str(),
        "FREE" | "STANDARD" | "PRO" | "ENTERPRISE"
    ) {
        Ok(normalized)
    } else {
        anyhow::bail!("invalid tenant plan");
    }
}

pub async fn create_tenant(pool: &PgPool, request: CreateTenantRequest) -> Result<Tenant> {
    let tenant_code = normalize_tenant_code(&request.tenant_code)?;
    let schema_name = format!("tenant_{tenant_code}");
    let plan = normalize_tenant_plan(request.plan.as_deref().unwrap_or("STANDARD"))?;

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
            max_users,
            plan
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, COALESCE($8, 10), $9)
        RETURNING
            tenant_id, tenant_code, tenant_name, biz_reg_no, contract_start,
            contract_end, schema_name, status, plan, suspended_at, allowed_ips, max_users,
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
    .bind(plan)
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
               contract_end, schema_name, status, plan, suspended_at, allowed_ips, max_users,
               created_at, updated_at
        FROM tenants
        ORDER BY tenant_code
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list tenants")
}

pub async fn suggest_tenants(pool: &PgPool, q: Option<&str>) -> Result<Vec<Value>> {
    let pattern = format!("%{}%", q.unwrap_or("").trim().to_ascii_lowercase());
    let rows = sqlx::query(
        r#"
        SELECT tenant_code, tenant_name, status, plan
        FROM tenants
        WHERE status = 'ACTIVE'
          AND (
              $1 = '%%'
              OR tenant_code ILIKE $1
              OR tenant_name ILIKE $1
          )
        ORDER BY
          CASE WHEN tenant_code = TRIM(BOTH '%' FROM $1) THEN 0 ELSE 1 END,
          tenant_code
        LIMIT 10
        "#,
    )
    .bind(pattern)
    .fetch_all(pool)
    .await
    .context("failed to suggest tenants")?;
    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "tenant_code": row.get::<String, _>("tenant_code"),
                "tenant_name": row.get::<String, _>("tenant_name"),
                "status": row.get::<String, _>("status"),
                "plan": row.get::<String, _>("plan"),
            })
        })
        .collect())
}

pub async fn update_tenant_status(
    pool: &PgPool,
    tenant_code: &str,
    request: UpdateTenantStatusRequest,
) -> Result<Tenant> {
    let tenant_code = normalize_tenant_code(tenant_code)?;
    let status = normalize_tenant_status(&request.status)?;
    sqlx::query_as::<_, Tenant>(
        r#"
        UPDATE tenants
        SET status = $2,
            suspended_at = CASE
                WHEN $2 = 'SUSPENDED' THEN COALESCE(suspended_at, NOW())
                WHEN $2 = 'ACTIVE' THEN NULL
                ELSE suspended_at
            END,
            updated_at = NOW()
        WHERE tenant_code = $1
        RETURNING tenant_id, tenant_code, tenant_name, biz_reg_no, contract_start,
                  contract_end, schema_name, status, plan, suspended_at, allowed_ips,
                  max_users, created_at, updated_at
        "#,
    )
    .bind(tenant_code)
    .bind(status)
    .fetch_one(pool)
    .await
    .context("failed to update tenant status")
}

pub async fn update_tenant_plan(
    pool: &PgPool,
    tenant_code: &str,
    request: UpdateTenantPlanRequest,
) -> Result<Tenant> {
    let tenant_code = normalize_tenant_code(tenant_code)?;
    let plan = normalize_tenant_plan(&request.plan)?;
    sqlx::query_as::<_, Tenant>(
        r#"
        UPDATE tenants
        SET plan = $2,
            updated_at = NOW()
        WHERE tenant_code = $1
        RETURNING tenant_id, tenant_code, tenant_name, biz_reg_no, contract_start,
                  contract_end, schema_name, status, plan, suspended_at, allowed_ips,
                  max_users, created_at, updated_at
        "#,
    )
    .bind(tenant_code)
    .bind(plan)
    .fetch_one(pool)
    .await
    .context("failed to update tenant plan")
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
            original_by_id  BIGINT REFERENCES {schema}.business_years(by_id),
            amendment_sequence INT NOT NULL DEFAULT 0,
            amendment_reason TEXT,
            version_mode    VARCHAR(30),
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CHECK (start_date <= end_date)
        );
        ALTER TABLE {schema}.business_years
            ALTER COLUMN status SET DEFAULT 'DRAFT';
        ALTER TABLE {schema}.business_years
            ADD COLUMN IF NOT EXISTS lock_mode VARCHAR(30) NOT NULL DEFAULT 'OPEN';
        ALTER TABLE {schema}.business_years
            ADD COLUMN IF NOT EXISTS original_by_id BIGINT REFERENCES {schema}.business_years(by_id);
        ALTER TABLE {schema}.business_years
            ADD COLUMN IF NOT EXISTS amendment_sequence INT NOT NULL DEFAULT 0;
        ALTER TABLE {schema}.business_years
            ADD COLUMN IF NOT EXISTS amendment_reason TEXT;
        ALTER TABLE {schema}.business_years
            ADD COLUMN IF NOT EXISTS version_mode VARCHAR(30);
        ALTER TABLE {schema}.business_years
            DROP CONSTRAINT IF EXISTS business_years_customer_id_year_label_key;
        CREATE UNIQUE INDEX IF NOT EXISTS idx_business_years_customer_year_sequence
            ON {schema}.business_years(customer_id, year_label, amendment_sequence);
        CREATE INDEX IF NOT EXISTS idx_business_years_original
            ON {schema}.business_years(original_by_id, amendment_sequence);
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
            submitted_at    TIMESTAMPTZ,
            receipt_no      VARCHAR(80),
            receipt_at      TIMESTAMPTZ
        );
        ALTER TABLE {schema}.efiling_history
            ADD COLUMN IF NOT EXISTS receipt_no VARCHAR(80);
        ALTER TABLE {schema}.efiling_history
            ADD COLUMN IF NOT EXISTS receipt_at TIMESTAMPTZ;

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
    let source_by = if let Some(source_by_id) = request.carry_forward_from_by_id {
        let source = get_business_year(pool, tenant, source_by_id).await?;
        if source.customer_id != request.customer_id {
            anyhow::bail!("carry-forward source must belong to the same customer");
        }
        Some(source)
    } else {
        None
    };
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
                "status": business_year.status.clone(),
                "carry_forward_from_by_id": source_by.as_ref().map(|item| item.by_id)
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

pub async fn dashboard_recent_activities(
    pool: &PgPool,
    tenant: &TenantRef,
    user: &AuthUser,
    limit: i64,
) -> Result<DashboardRecentActivitySummary> {
    let super_admin = user.roles.iter().any(|role| role == "SUPER_ADMIN");
    if user.tenant_id != tenant.tenant_id && !super_admin {
        anyhow::bail!("tenant access denied");
    }
    let all_access = super_admin
        || (user.tenant_id == tenant.tenant_id
            && user.roles.iter().any(|role| {
                matches!(
                    role.as_str(),
                    "TENANT_ADMIN" | "SYSTEM_ADMIN" | "SUPER_ADMIN"
                )
            }));
    let limit = limit.clamp(1, 50);
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        WITH visible_customers AS (
            SELECT a.customer_id
            FROM user_customer_access a
            WHERE a.user_id = $3
              AND a.tenant_id = $1
              AND a.access_level <> 'BLOCKED'
              AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
              AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
            UNION
            SELECT d.customer_id
            FROM access_delegations d
            WHERE d.tenant_id = $1
              AND d.delegatee_user_id = $3
              AND d.status = 'ACTIVE'
              AND (d.valid_from IS NULL OR d.valid_from <= CURRENT_DATE)
              AND (d.valid_to IS NULL OR d.valid_to >= CURRENT_DATE)
        ),
        resolved AS (
            SELECT
                al.audit_id,
                al.table_name,
                al.record_id,
                al.action,
                al.old_data,
                al.new_data,
                al.changed_by,
                COALESCE(u.user_name, al.changed_by) AS actor_name,
                al.changed_at,
                COALESCE(by_log.by_id, by_notification.by_id) AS by_id,
                COALESCE(by_log.customer_id, by_notification.customer_id, customer_log.customer_id) AS customer_id,
                COALESCE(customer_by.customer_name, customer_notification.customer_name, customer_log.customer_name) AS customer_name,
                COALESCE(by_log.year_label, by_notification.year_label) AS fiscal_year,
                COALESCE(by_log.start_date, by_notification.start_date) AS start_date,
                COALESCE(by_log.end_date, by_notification.end_date) AS end_date,
                COALESCE(by_log.status, by_notification.status) AS business_year_status
            FROM {schema}.audit_logs al
            LEFT JOIN users u
              ON u.tenant_id = $1
             AND u.login_id = al.changed_by
            LEFT JOIN {schema}.business_years by_log
              ON al.table_name = 'business_years'
             AND al.record_id ~ '^[0-9]+$'
             AND by_log.by_id = al.record_id::BIGINT
            LEFT JOIN {schema}.customers customer_by
              ON customer_by.customer_id = by_log.customer_id
            LEFT JOIN {schema}.customers customer_log
              ON al.table_name = 'customers'
             AND al.record_id ~ '^[0-9]+$'
             AND customer_log.customer_id = al.record_id::BIGINT
            LEFT JOIN {schema}.notifications notification_log
              ON al.table_name = 'notifications'
             AND al.record_id ~ '^[0-9]+$'
             AND notification_log.notification_id = al.record_id::BIGINT
            LEFT JOIN {schema}.business_years by_notification
              ON by_notification.by_id = notification_log.by_id
            LEFT JOIN {schema}.customers customer_notification
              ON customer_notification.customer_id = by_notification.customer_id
        ),
        filtered AS (
            SELECT *
            FROM resolved
            WHERE $2::BOOL
               OR changed_by = $4
               OR customer_id IN (SELECT customer_id FROM visible_customers)
        )
        SELECT *,
               COUNT(*) OVER() AS total_count
        FROM filtered
        ORDER BY audit_id DESC
        LIMIT $5
        "#
    );

    let rows = sqlx::query(&sql)
        .bind(tenant.tenant_id)
        .bind(all_access)
        .bind(user.user_id)
        .bind(&user.login_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("failed to load dashboard recent activities")?;

    let total_count = rows
        .first()
        .map(|row| row.get::<i64, _>("total_count"))
        .unwrap_or(0);
    let activities = rows
        .into_iter()
        .map(|row| {
            let table_name = row.get::<String, _>("table_name");
            let action = row.get::<String, _>("action");
            let old_data = row.get::<Option<Value>, _>("old_data");
            let new_data = row.get::<Option<Value>, _>("new_data");
            let descriptor = dashboard_activity_descriptor(
                &table_name,
                &action,
                old_data.as_ref(),
                new_data.as_ref(),
            );
            DashboardRecentActivityItem {
                audit_id: row.get("audit_id"),
                activity_type: descriptor.activity_type,
                type_label: descriptor.type_label,
                description: descriptor.description,
                table_name,
                action,
                record_id: row.get("record_id"),
                actor_login_id: row.get("changed_by"),
                actor_name: row.get("actor_name"),
                customer_id: row.get("customer_id"),
                customer_name: row.get("customer_name"),
                by_id: row.get("by_id"),
                fiscal_year: row.get("fiscal_year"),
                start_date: row.get("start_date"),
                end_date: row.get("end_date"),
                business_year_status: row.get("business_year_status"),
                route_key: descriptor.route_key,
                occurred_at: row.get("changed_at"),
            }
        })
        .collect();

    Ok(DashboardRecentActivitySummary {
        activities,
        total_count,
        limit,
    })
}

struct DashboardActivityDescriptor {
    activity_type: String,
    type_label: String,
    description: String,
    route_key: String,
}

fn dashboard_activity_descriptor(
    table_name: &str,
    action: &str,
    old_data: Option<&Value>,
    new_data: Option<&Value>,
) -> DashboardActivityDescriptor {
    let old_status = old_data
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str);
    let new_status = new_data
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str);
    let action = action.to_ascii_uppercase();
    match (table_name, action.as_str(), old_status, new_status) {
        ("business_years", "UPDATE", Some("DRAFT"), Some("IN_REVIEW")) => activity_descriptor(
            "REVIEW_REQUESTED",
            "결재 요청",
            "결재 요청 (IN_REVIEW 전환)",
            "ws/appr:inbox",
        ),
        ("business_years", "UPDATE", Some("IN_REVIEW"), Some("APPROVED")) => activity_descriptor(
            "REVIEW_APPROVED",
            "결재 승인",
            "결재 승인 (APPROVED 전환)",
            "ws/appr:inbox",
        ),
        ("business_years", "UPDATE", Some("IN_REVIEW"), Some("DRAFT")) => activity_descriptor(
            "REVIEW_REJECTED",
            "결재 반려",
            "결재 반려 (DRAFT 전환)",
            "ws/appr:rejected",
        ),
        ("business_years", "UPDATE", Some("APPROVED"), Some("FILED")) => activity_descriptor(
            "EFILING_SUBMITTED",
            "신고 완료",
            "신고 제출 완료 (FILED)",
            "post/hist:list",
        ),
        ("business_years", "UPDATE", Some("FILED"), Some("AMENDED")) => activity_descriptor(
            "LOCK_RELEASED",
            "잠금 해제",
            "잠금 해제 신청 (수정신고)",
            "post/amend:unlock",
        ),
        ("business_years", "CREATE", _, _) => activity_descriptor(
            "BUSINESS_YEAR_CREATED",
            "사업연도 생성",
            "사업연도 생성",
            "ws/start:snapshot",
        ),
        ("business_years", "UPDATE", _, _) => activity_descriptor(
            "BUSINESS_YEAR_UPDATED",
            "사업연도 변경",
            "사업연도 상태 변경",
            "ws/start:snapshot",
        ),
        ("customers", "CREATE", _, _) => activity_descriptor(
            "CUSTOMER_CREATED",
            "고객사 등록",
            "고객사 등록",
            "ws/start:customer-pick",
        ),
        ("tax_adjustments", _, _, _) | ("adjustment_items", _, _, _) => activity_descriptor(
            "TAX_ADJ_SAVED",
            "세무조정 저장",
            "세무조정 저장",
            "ws/adj:B1",
        ),
        ("form_data", _, _, _) => activity_descriptor(
            "FORM_GENERATED",
            "서식 생성",
            "서식 생성 완료",
            "ws/form:preview",
        ),
        ("validation_issues", _, _, _) => {
            activity_descriptor("VALIDATION_RUN", "검증 실행", "검증 실행", "ws/val:run")
        }
        ("efiling_history", _, _, _) | ("efiling_files", _, _, _) => activity_descriptor(
            "EFILING_SUBMITTED",
            "전자신고",
            "전자신고 파일 처리",
            "post/hist:list",
        ),
        ("notifications", _, _, _) => activity_descriptor(
            "NOTIFICATION_UPDATED",
            "알림 변경",
            "알림 상태 변경",
            "rp-alerts",
        ),
        _ => activity_descriptor(
            "AUDIT_EVENT",
            "업무 변경",
            &format!("{} {}", table_name, action),
            "ad-audit",
        ),
    }
}

fn activity_descriptor(
    activity_type: &str,
    type_label: &str,
    description: &str,
    route_key: &str,
) -> DashboardActivityDescriptor {
    DashboardActivityDescriptor {
        activity_type: activity_type.to_string(),
        type_label: type_label.to_string(),
        description: description.to_string(),
        route_key: route_key.to_string(),
    }
}

pub async fn ensure_due_notifications(pool: &PgPool, tenant: &TenantRef) -> Result<i64> {
    let schema = quote_ident(&tenant.schema_name)?;
    let mut created = 0_i64;
    for (bucket, days, severity, notification_type) in [
        ("D-30", 30_i64, "WARN", "DEADLINE_D30"),
        ("D-7", 7_i64, "ERROR", "DEADLINE_D7"),
        ("D-Day", 0_i64, "ERROR", "DEADLINE_DDAY"),
    ] {
        let title = format!("사업연도 마감 {bucket}");
        let message_suffix = if days == 0 {
            " 사업연도 마감일이 오늘입니다.".to_string()
        } else {
            format!(" 사업연도 마감일이 {days}일 이내입니다.")
        };
        sqlx::query(&format!(
            r#"
            UPDATE {schema}.notifications
            SET title = $2,
                message = CONCAT(COALESCE(metadata->>'year_label', ''), $3),
                metadata = jsonb_set(
                    jsonb_set(metadata, '{{notification_type}}', to_jsonb($4::TEXT), true),
                    '{{route_key}}',
                    to_jsonb(COALESCE(metadata->>'route_key', 'ws/start:snapshot')::TEXT),
                    true
                )
            WHERE metadata->>'due_bucket' = $1::TEXT
              AND title LIKE 'Business year due%'
            "#
        ))
        .bind(bucket)
        .bind(&title)
        .bind(&message_suffix)
        .bind(notification_type)
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
                   jsonb_build_object(
                       'by_id', b.by_id,
                       'year_label', b.year_label,
                       'end_date', b.end_date,
                       'due_bucket', $1::TEXT,
                       'notification_type', $6::TEXT,
                       'route_key', CASE
                           WHEN b.status = 'IN_REVIEW' THEN 'ws/appr:inbox'
                           WHEN b.status = 'APPROVED' THEN 'ws/print:preview'
                           WHEN b.status = 'AMENDED' THEN 'post/amend:unlock'
                           ELSE 'ws/start:snapshot'
                       END,
                       'days_remaining', (b.end_date - CURRENT_DATE)
                   )
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
            .bind(notification_type)
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

fn ensure_dashboard_user_tenant_access(user: &AuthUser, tenant: &TenantRef) -> Result<()> {
    if user.tenant_id == tenant.tenant_id || user.roles.iter().any(|role| role == "SUPER_ADMIN") {
        Ok(())
    } else {
        anyhow::bail!("tenant access denied")
    }
}

fn dashboard_user_has_all_access(user: &AuthUser, tenant: &TenantRef) -> bool {
    user.roles.iter().any(|role| role == "SUPER_ADMIN")
        || (user.tenant_id == tenant.tenant_id
            && user.roles.iter().any(|role| {
                matches!(
                    role.as_str(),
                    "TENANT_ADMIN" | "SYSTEM_ADMIN" | "SUPER_ADMIN"
                )
            }))
}

pub async fn dashboard_notifications(
    pool: &PgPool,
    tenant: &TenantRef,
    user: &AuthUser,
    limit: i64,
    unread_only: bool,
) -> Result<DashboardNotificationSummary> {
    ensure_dashboard_user_tenant_access(user, tenant)?;
    ensure_due_notifications(pool, tenant).await?;
    let all_access = dashboard_user_has_all_access(user, tenant);
    let schema = quote_ident(&tenant.schema_name)?;
    let bounded_limit = limit.clamp(1, 50);
    let notifications = sqlx::query_as::<_, DashboardNotificationItem>(&format!(
        r#"
        WITH visible_customers AS (
            SELECT a.customer_id
            FROM user_customer_access a
            WHERE a.user_id = $3
              AND a.tenant_id = $1
              AND a.access_level <> 'BLOCKED'
              AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
              AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
            UNION
            SELECT d.customer_id
            FROM access_delegations d
            WHERE d.tenant_id = $1
              AND d.delegatee_user_id = $3
              AND d.status = 'ACTIVE'
              AND (d.valid_from IS NULL OR d.valid_from <= CURRENT_DATE)
              AND (d.valid_to IS NULL OR d.valid_to >= CURRENT_DATE)
        )
        SELECT
            n.notification_id,
            n.by_id,
            b.customer_id,
            c.customer_name,
            b.year_label AS fiscal_year,
            b.start_date,
            b.end_date AS filing_due_date,
            b.status AS business_year_status,
            n.title,
            n.message,
            n.severity,
            n.status,
            COALESCE(
                n.metadata->>'notification_type',
                CASE WHEN n.metadata ? 'due_bucket' THEN 'DUE_DEADLINE' ELSE 'GENERAL' END
            ) AS notification_type,
            NULLIF(n.metadata->>'due_bucket', '') AS due_bucket,
            COALESCE(
                n.metadata->>'route_key',
                CASE
                    WHEN n.by_id IS NULL THEN 'rp-alerts'
                    WHEN b.status = 'IN_REVIEW' AND EXISTS (
                        SELECT 1
                        FROM {schema}.approval_lines al
                        WHERE al.by_id = b.by_id
                          AND al.status = 'PENDING'
                    ) THEN 'ws/appr:inbox'
                    WHEN b.status = 'IN_REVIEW' THEN 'ws/val:run'
                    WHEN b.status = 'APPROVED' THEN 'ws/print:preview'
                    WHEN b.status = 'FILED' THEN 'post/hist:list'
                    WHEN b.status = 'AMENDED' THEN 'post/amend:unlock'
                    ELSE 'ws/start:snapshot'
                END
            ) AS route_key,
            n.created_at,
            n.read_at
        FROM {schema}.notifications n
        LEFT JOIN {schema}.business_years b ON b.by_id = n.by_id
        LEFT JOIN {schema}.customers c ON c.customer_id = b.customer_id
        WHERE n.status <> 'ARCHIVED'
          AND ($4::BOOL = FALSE OR n.status = 'UNREAD')
          AND (
              $2::BOOL
              OR b.customer_id IN (SELECT customer_id FROM visible_customers)
          )
        ORDER BY
            CASE WHEN n.status = 'UNREAD' THEN 0 ELSE 1 END,
            CASE n.severity WHEN 'ERROR' THEN 0 WHEN 'WARN' THEN 1 ELSE 2 END,
            n.created_at DESC,
            n.notification_id DESC
        LIMIT $5
        "#
    ))
    .bind(tenant.tenant_id)
    .bind(all_access)
    .bind(user.user_id)
    .bind(unread_only)
    .bind(bounded_limit)
    .fetch_all(pool)
    .await
    .context("failed to load dashboard notifications")?;

    let count_row = sqlx::query(&format!(
        r#"
        WITH visible_customers AS (
            SELECT a.customer_id
            FROM user_customer_access a
            WHERE a.user_id = $3
              AND a.tenant_id = $1
              AND a.access_level <> 'BLOCKED'
              AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
              AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
            UNION
            SELECT d.customer_id
            FROM access_delegations d
            WHERE d.tenant_id = $1
              AND d.delegatee_user_id = $3
              AND d.status = 'ACTIVE'
              AND (d.valid_from IS NULL OR d.valid_from <= CURRENT_DATE)
              AND (d.valid_to IS NULL OR d.valid_to >= CURRENT_DATE)
        ),
        visible_notifications AS (
            SELECT n.notification_id, n.status
            FROM {schema}.notifications n
            LEFT JOIN {schema}.business_years b ON b.by_id = n.by_id
            WHERE n.status <> 'ARCHIVED'
              AND (
                  $2::BOOL
                  OR b.customer_id IN (SELECT customer_id FROM visible_customers)
              )
        )
        SELECT
            (SELECT COUNT(*) FROM visible_notifications WHERE status = 'UNREAD') AS unread_count,
            (SELECT COUNT(*)
             FROM visible_notifications
             WHERE $4::BOOL = FALSE OR status = 'UNREAD') AS total_count
        "#
    ))
    .bind(tenant.tenant_id)
    .bind(all_access)
    .bind(user.user_id)
    .bind(unread_only)
    .fetch_one(pool)
    .await
    .context("failed to count dashboard notifications")?;

    Ok(DashboardNotificationSummary {
        notifications,
        unread_count: count_row.get("unread_count"),
        total_count: count_row.get("total_count"),
        limit: bounded_limit,
        unread_only,
    })
}

pub async fn update_notification(
    pool: &PgPool,
    tenant: &TenantRef,
    user: &AuthUser,
    notification_id: i64,
    request: UpdateNotificationRequest,
) -> Result<Notification> {
    ensure_dashboard_user_tenant_access(user, tenant)?;
    let all_access = dashboard_user_has_all_access(user, tenant);
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
        WITH visible_customers AS (
            SELECT a.customer_id
            FROM user_customer_access a
            WHERE a.user_id = $3
              AND a.tenant_id = $1
              AND a.access_level <> 'BLOCKED'
              AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
              AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
            UNION
            SELECT d.customer_id
            FROM access_delegations d
            WHERE d.tenant_id = $1
              AND d.delegatee_user_id = $3
              AND d.status = 'ACTIVE'
              AND (d.valid_from IS NULL OR d.valid_from <= CURRENT_DATE)
              AND (d.valid_to IS NULL OR d.valid_to >= CURRENT_DATE)
        )
        UPDATE {schema}.notifications n
        SET status = $2,
            read_at = CASE
                WHEN $2 = 'READ' THEN COALESCE(read_at, NOW())
                WHEN $2 = 'UNREAD' THEN NULL
                ELSE read_at
            END
        WHERE n.notification_id = $4
          AND (
              $5::BOOL
              OR EXISTS (
                  SELECT 1
                  FROM {schema}.business_years b
                  WHERE b.by_id = n.by_id
                    AND b.customer_id IN (SELECT customer_id FROM visible_customers)
              )
          )
        RETURNING notification_id, by_id, title, message, severity, status,
                  metadata, created_at, read_at
        "#
    );
    let notification = sqlx::query_as::<_, Notification>(&sql)
        .bind(tenant.tenant_id)
        .bind(&status)
        .bind(user.user_id)
        .bind(notification_id)
        .bind(all_access)
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

pub async fn dashboard_summary(
    pool: &PgPool,
    tenant: &TenantRef,
    user: &AuthUser,
) -> Result<DashboardSummary> {
    ensure_dashboard_user_tenant_access(user, tenant)?;
    ensure_due_notifications(pool, tenant).await?;
    let all_access = dashboard_user_has_all_access(user, tenant);
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        WITH visible_customers AS (
            SELECT a.customer_id
            FROM user_customer_access a
            WHERE a.user_id = $3
              AND a.tenant_id = $1
              AND a.access_level <> 'BLOCKED'
              AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
              AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
            UNION
            SELECT d.customer_id
            FROM access_delegations d
            WHERE d.tenant_id = $1
              AND d.delegatee_user_id = $3
              AND d.status = 'ACTIVE'
              AND (d.valid_from IS NULL OR d.valid_from <= CURRENT_DATE)
              AND (d.valid_to IS NULL OR d.valid_to >= CURRENT_DATE)
        ),
        visible_business_years AS (
            SELECT b.*
            FROM {schema}.business_years b
            WHERE $2::BOOL OR b.customer_id IN (SELECT customer_id FROM visible_customers)
        ),
        visible_notifications AS (
            SELECT n.notification_id, n.status
            FROM {schema}.notifications n
            LEFT JOIN {schema}.business_years b ON b.by_id = n.by_id
            WHERE n.status <> 'ARCHIVED'
              AND (
                  $2::BOOL
                  OR b.customer_id IN (SELECT customer_id FROM visible_customers)
              )
        ),
        visible_audit_logs AS (
            SELECT al.audit_id
            FROM {schema}.audit_logs al
            WHERE $2::BOOL
               OR EXISTS (
                   SELECT 1
                   FROM visible_business_years b
                   WHERE (al.table_name = 'business_years' AND al.record_id = b.by_id::TEXT)
                      OR al.new_data->>'by_id' = b.by_id::TEXT
                      OR al.old_data->>'by_id' = b.by_id::TEXT
               )
               OR EXISTS (
                   SELECT 1
                   FROM visible_customers c
                   WHERE (al.table_name = 'customers' AND al.record_id = c.customer_id::TEXT)
                      OR al.new_data->>'customer_id' = c.customer_id::TEXT
                      OR al.old_data->>'customer_id' = c.customer_id::TEXT
               )
        )
        SELECT
            (SELECT COUNT(*)
             FROM {schema}.customers c
             WHERE $2::BOOL OR c.customer_id IN (SELECT customer_id FROM visible_customers)) AS customer_count,
            (SELECT COUNT(*) FROM visible_business_years) AS business_year_count,
            (SELECT COUNT(*) FROM visible_business_years WHERE status = 'FILED') AS filed_count,
            (SELECT COUNT(*) FROM visible_business_years WHERE status = 'IN_REVIEW') AS pending_review_count,
            (SELECT COUNT(*) FROM visible_business_years
             WHERE status <> 'FILED'
               AND end_date BETWEEN CURRENT_DATE AND CURRENT_DATE + INTERVAL '30 days') AS due_soon_count,
            (SELECT COUNT(*) FROM visible_notifications WHERE status = 'UNREAD') AS unread_notifications,
            (SELECT COUNT(*) FROM visible_audit_logs) AS audit_log_count
        "#
    );
    let row = sqlx::query_as::<_, DashboardCounts>(&sql)
        .bind(tenant.tenant_id)
        .bind(all_access)
        .bind(user.user_id)
        .fetch_one(pool)
        .await
        .context("failed to load dashboard summary")?;
    let (work_status, rejected_count) =
        dashboard_work_status(pool, &schema, tenant.tenant_id, all_access, user.user_id).await?;
    let filing_deadlines = dashboard_filing_deadlines_for_schema(
        pool,
        &schema,
        tenant.tenant_id,
        all_access,
        user.user_id,
        30,
    )
    .await?;
    Ok(DashboardSummary {
        tenant_code: tenant.tenant_code.clone(),
        customer_count: row.customer_count,
        business_year_count: row.business_year_count,
        filed_count: row.filed_count,
        pending_review_count: row.pending_review_count,
        due_soon_count: row.due_soon_count,
        unread_notifications: row.unread_notifications,
        audit_log_count: row.audit_log_count,
        work_status,
        rejected_count,
        filing_deadlines,
    })
}

pub async fn dashboard_filing_deadlines(
    pool: &PgPool,
    tenant: &TenantRef,
    user: &AuthUser,
    within_days: i64,
) -> Result<DashboardFilingDeadlineSummary> {
    ensure_dashboard_user_tenant_access(user, tenant)?;
    let all_access = dashboard_user_has_all_access(user, tenant);
    let schema = quote_ident(&tenant.schema_name)?;
    dashboard_filing_deadlines_for_schema(
        pool,
        &schema,
        tenant.tenant_id,
        all_access,
        user.user_id,
        within_days,
    )
    .await
}

async fn dashboard_filing_deadlines_for_schema(
    pool: &PgPool,
    schema: &str,
    tenant_id: i64,
    all_access: bool,
    user_id: i64,
    within_days: i64,
) -> Result<DashboardFilingDeadlineSummary> {
    let bounded_days = within_days.clamp(1, 365);
    let deadlines = sqlx::query_as::<_, DashboardFilingDeadline>(&format!(
        r#"
        WITH visible_customers AS (
            SELECT a.customer_id
            FROM user_customer_access a
            WHERE a.user_id = $3
              AND a.tenant_id = $1
              AND a.access_level <> 'BLOCKED'
              AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
              AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
            UNION
            SELECT d.customer_id
            FROM access_delegations d
            WHERE d.tenant_id = $1
              AND d.delegatee_user_id = $3
              AND d.status = 'ACTIVE'
              AND (d.valid_from IS NULL OR d.valid_from <= CURRENT_DATE)
              AND (d.valid_to IS NULL OR d.valid_to >= CURRENT_DATE)
        ),
        adjustment_progress AS (
            SELECT
                by_id,
                COUNT(DISTINCT COALESCE(NULLIF(metadata->>'module', ''), adj_category))::BIGINT AS saved_modules
            FROM {schema}.tax_adjustments
            WHERE status = 'POSTED'
            GROUP BY by_id
        ),
        deadline_rows AS (
            SELECT
                b.by_id AS business_year_id,
                b.customer_id,
                c.customer_name,
                b.year_label AS fiscal_year,
                b.start_date,
                b.end_date AS filing_due_date,
                (b.end_date - CURRENT_DATE)::BIGINT AS days_remaining,
                b.status,
                CASE
                    WHEN b.status = 'DRAFT' THEN '작성중'
                    WHEN b.status = 'IN_REVIEW' AND EXISTS (
                        SELECT 1
                        FROM {schema}.approval_lines al
                        WHERE al.by_id = b.by_id
                          AND al.status = 'PENDING'
                    ) THEN '결재 대기'
                    WHEN b.status = 'IN_REVIEW' THEN '검증 대기'
                    WHEN b.status = 'APPROVED' THEN '승인 완료'
                    WHEN b.status = 'AMENDED' THEN '수정신고'
                    ELSE b.status
                END AS status_label,
                LEAST(100, ROUND(COALESCE(ap.saved_modules, 0) * 100.0 / 17, 0))::BIGINT AS progress_pct,
                CASE
                    WHEN (b.end_date - CURRENT_DATE) <= 7 THEN 'CRITICAL'
                    WHEN (b.end_date - CURRENT_DATE) <= 14 THEN 'WARNING'
                    ELSE 'NOTICE'
                END AS urgency_level,
                CASE
                    WHEN b.status = 'IN_REVIEW' AND EXISTS (
                        SELECT 1
                        FROM {schema}.approval_lines al
                        WHERE al.by_id = b.by_id
                          AND al.status = 'PENDING'
                    ) THEN 'ws/appr:request'
                    WHEN b.status = 'IN_REVIEW' THEN 'ws/val:run'
                    WHEN b.status = 'APPROVED' THEN 'ws/print:preview'
                    WHEN b.status = 'AMENDED' THEN 'post/amend:unlock'
                    ELSE 'ws/start:snapshot'
                END AS route_key
            FROM {schema}.business_years b
            JOIN {schema}.customers c ON c.customer_id = b.customer_id
            LEFT JOIN adjustment_progress ap ON ap.by_id = b.by_id
            WHERE b.status <> 'FILED'
              AND b.end_date BETWEEN CURRENT_DATE AND CURRENT_DATE + ($4::INT * INTERVAL '1 day')
              AND (
                  $2::BOOL
                  OR b.customer_id IN (SELECT customer_id FROM visible_customers)
              )
        )
        SELECT *
        FROM deadline_rows
        ORDER BY days_remaining ASC, filing_due_date ASC, business_year_id ASC
        "#
    ))
    .bind(tenant_id)
    .bind(all_access)
    .bind(user_id)
    .bind(bounded_days as i32)
    .fetch_all(pool)
    .await
    .context("failed to load dashboard filing deadlines")?;
    let total_count = deadlines.len() as i64;
    Ok(DashboardFilingDeadlineSummary {
        deadlines,
        total_count,
    })
}

async fn dashboard_work_status(
    pool: &PgPool,
    schema: &str,
    tenant_id: i64,
    all_access: bool,
    user_id: i64,
) -> Result<(Vec<DashboardWorkStatus>, i64)> {
    let rows = sqlx::query_as::<_, DashboardWorkStatusCounts>(&format!(
        r#"
        WITH visible_customers AS (
            SELECT a.customer_id
            FROM user_customer_access a
            WHERE a.user_id = $3
              AND a.tenant_id = $1
              AND a.access_level <> 'BLOCKED'
              AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
              AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
            UNION
            SELECT d.customer_id
            FROM access_delegations d
            WHERE d.tenant_id = $1
              AND d.delegatee_user_id = $3
              AND d.status = 'ACTIVE'
              AND (d.valid_from IS NULL OR d.valid_from <= CURRENT_DATE)
              AND (d.valid_to IS NULL OR d.valid_to >= CURRENT_DATE)
        ),
        base AS (
            SELECT
                b.by_id,
                b.customer_id,
                b.status,
                b.end_date,
                EXISTS (
                    SELECT 1
                    FROM {schema}.approval_lines al
                    WHERE al.by_id = b.by_id
                      AND al.status = 'PENDING'
                ) AS has_pending_approval,
                EXISTS (
                    SELECT 1
                    FROM {schema}.approval_lines al
                    WHERE al.by_id = b.by_id
                      AND al.status = 'RETURNED'
                ) AS has_returned_approval
            FROM {schema}.business_years b
            WHERE $2::BOOL
               OR b.customer_id IN (SELECT customer_id FROM visible_customers)
        ),
        classified AS (
            SELECT
                by_id,
                customer_id,
                end_date,
                has_returned_approval,
                CASE
                    WHEN status = 'IN_REVIEW' AND has_pending_approval THEN 'IN_REVIEW_APPROVAL'
                    WHEN status = 'IN_REVIEW' THEN 'IN_REVIEW_VALIDATION'
                    WHEN status IN ('DRAFT', 'APPROVED', 'FILED') THEN status
                    ELSE NULL
                END AS status_key
            FROM base
        )
        SELECT
            status_key,
            COUNT(*)::BIGINT AS year_count,
            COUNT(DISTINCT customer_id)::BIGINT AS customer_count,
            COUNT(*) FILTER (
                WHERE status_key <> 'FILED'
                  AND end_date BETWEEN CURRENT_DATE AND CURRENT_DATE + INTERVAL '7 days'
            )::BIGINT AS urgent_count
        FROM classified
        WHERE status_key IS NOT NULL
        GROUP BY status_key
        "#
    ))
    .bind(tenant_id)
    .bind(all_access)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("failed to load dashboard work status")?;
    let rejected_count = sqlx::query_scalar::<_, i64>(&format!(
        r#"
        WITH visible_customers AS (
            SELECT a.customer_id
            FROM user_customer_access a
            WHERE a.user_id = $3
              AND a.tenant_id = $1
              AND a.access_level <> 'BLOCKED'
              AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
              AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
            UNION
            SELECT d.customer_id
            FROM access_delegations d
            WHERE d.tenant_id = $1
              AND d.delegatee_user_id = $3
              AND d.status = 'ACTIVE'
              AND (d.valid_from IS NULL OR d.valid_from <= CURRENT_DATE)
              AND (d.valid_to IS NULL OR d.valid_to >= CURRENT_DATE)
        )
        SELECT COUNT(*)::BIGINT
        FROM {schema}.business_years b
        WHERE b.status = 'DRAFT'
          AND (
              $2::BOOL
              OR b.customer_id IN (SELECT customer_id FROM visible_customers)
          )
          AND EXISTS (
              SELECT 1
              FROM {schema}.approval_lines al
              WHERE al.by_id = b.by_id
                AND al.status = 'RETURNED'
          )
        "#
    ))
    .bind(tenant_id)
    .bind(all_access)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .context("failed to count rejected dashboard work status")?;
    let counts = rows
        .into_iter()
        .map(|row| (row.status_key.clone(), row))
        .collect::<HashMap<_, _>>();
    let work_status = DASHBOARD_WORK_STATUS_DEFS
        .iter()
        .map(|definition| {
            let count = counts.get(definition.status);
            DashboardWorkStatus {
                status: definition.status.to_string(),
                label: definition.label.to_string(),
                year_count: count.map(|item| item.year_count).unwrap_or_default(),
                customer_count: count.map(|item| item.customer_count).unwrap_or_default(),
                urgent_count: count.map(|item| item.urgent_count).unwrap_or_default(),
                color: definition.color.to_string(),
            }
        })
        .collect();
    Ok((work_status, rejected_count))
}

const DASHBOARD_WORK_STATUS_DEFS: &[DashboardWorkStatusDefinition] = &[
    DashboardWorkStatusDefinition {
        status: "DRAFT",
        label: "작성중",
        color: "#3B82F6",
    },
    DashboardWorkStatusDefinition {
        status: "IN_REVIEW_VALIDATION",
        label: "검증 대기",
        color: "#FB923C",
    },
    DashboardWorkStatusDefinition {
        status: "IN_REVIEW_APPROVAL",
        label: "결재 대기",
        color: "#EF4444",
    },
    DashboardWorkStatusDefinition {
        status: "APPROVED",
        label: "승인 완료",
        color: "#22C55E",
    },
    DashboardWorkStatusDefinition {
        status: "FILED",
        label: "신고 완료",
        color: "#6B7280",
    },
];

struct DashboardWorkStatusDefinition {
    status: &'static str,
    label: &'static str,
    color: &'static str,
}

#[derive(sqlx::FromRow)]
struct DashboardWorkStatusCounts {
    status_key: String,
    year_count: i64,
    customer_count: i64,
    urgent_count: i64,
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

pub async fn dashboard_tax_burden_kpi(
    pool: &PgPool,
    tenant: &TenantRef,
    user: &AuthUser,
    years: i64,
    customer_id: Option<i64>,
) -> Result<DashboardTaxBurdenKpiSummary> {
    let super_admin = user.roles.iter().any(|role| role == "SUPER_ADMIN");
    if user.tenant_id != tenant.tenant_id && !super_admin {
        anyhow::bail!("tenant access denied");
    }
    let all_access = super_admin
        || (user.tenant_id == tenant.tenant_id
            && user.roles.iter().any(|role| {
                matches!(
                    role.as_str(),
                    "TENANT_ADMIN" | "SYSTEM_ADMIN" | "SUPER_ADMIN"
                )
            }));
    let years = years.clamp(1, 10);
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        WITH visible_customers AS (
            SELECT a.customer_id
            FROM user_customer_access a
            WHERE a.user_id = $3
              AND a.tenant_id = $1
              AND a.access_level <> 'BLOCKED'
              AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
              AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
            UNION
            SELECT d.customer_id
            FROM access_delegations d
            WHERE d.tenant_id = $1
              AND d.delegatee_user_id = $3
              AND d.status = 'ACTIVE'
              AND (d.valid_from IS NULL OR d.valid_from <= CURRENT_DATE)
              AND (d.valid_to IS NULL OR d.valid_to >= CURRENT_DATE)
        ),
        business_year_values AS (
            SELECT
                b.by_id,
                b.customer_id,
                b.year_label,
                COALESCE(
                    MAX(a.amount) FILTER (
                        WHERE a.adj_code = 'TAXABLE_INCOME'
                          AND a.status = 'POSTED'
                    ),
                    CASE
                        WHEN latest_form.data_json->>'taxable_income' ~ '^-?[0-9]+$'
                        THEN (latest_form.data_json->>'taxable_income')::BIGINT
                        ELSE 0
                    END
                )::BIGINT AS taxable_income,
                COALESCE(
                    MAX(a.amount) FILTER (
                        WHERE a.adj_code = 'TOTAL_TAX_DUE'
                          AND a.status = 'POSTED'
                    ),
                    CASE
                        WHEN latest_form.data_json->>'total_tax_due' ~ '^-?[0-9]+$'
                        THEN (latest_form.data_json->>'total_tax_due')::BIGINT
                        ELSE 0
                    END
                )::BIGINT AS total_tax_due
            FROM {schema}.business_years b
            LEFT JOIN {schema}.tax_adjustments a ON a.by_id = b.by_id
            LEFT JOIN LATERAL (
                SELECT data_json
                FROM {schema}.form_data fd
                WHERE fd.by_id = b.by_id
                  AND fd.form_code = 'FORM3'
                ORDER BY fd.updated_at DESC, fd.form_data_id DESC
                LIMIT 1
            ) latest_form ON TRUE
            WHERE b.status IN ('APPROVED', 'FILED', 'AMENDED')
              AND ($2::BOOL OR b.customer_id IN (SELECT customer_id FROM visible_customers))
              AND ($4::BIGINT IS NULL OR b.customer_id = $4)
            GROUP BY b.by_id, b.customer_id, b.year_label, latest_form.data_json
        ),
        selected_years AS (
            SELECT DISTINCT year_label
            FROM business_year_values
            WHERE taxable_income > 0
            ORDER BY year_label DESC
            LIMIT $5
        )
        SELECT
            v.year_label AS fiscal_year,
            COUNT(DISTINCT v.customer_id)::BIGINT AS customer_count,
            SUM(v.taxable_income)::BIGINT AS taxable_income,
            SUM(v.total_tax_due)::BIGINT AS total_tax_due,
            CASE
                WHEN SUM(v.taxable_income) = 0 THEN 0
                ELSE (SUM(v.total_tax_due) * 10000 / SUM(v.taxable_income))::BIGINT
            END AS effective_tax_rate_bps,
            CASE
                WHEN SUM(v.taxable_income) = 0 THEN 0::DOUBLE PRECISION
                ELSE (SUM(v.total_tax_due)::DOUBLE PRECISION * 100.0 / SUM(v.taxable_income)::DOUBLE PRECISION)
            END AS effective_tax_rate_pct
        FROM business_year_values v
        JOIN selected_years y ON y.year_label = v.year_label
        WHERE v.taxable_income > 0
        GROUP BY v.year_label
        ORDER BY v.year_label ASC
        "#
    );
    let trend = sqlx::query_as::<_, DashboardTaxBurdenKpiPoint>(&sql)
        .bind(tenant.tenant_id)
        .bind(all_access)
        .bind(user.user_id)
        .bind(customer_id)
        .bind(years)
        .fetch_all(pool)
        .await
        .context("failed to load dashboard tax burden KPI")?;
    let total_taxable_income = trend.iter().map(|row| row.taxable_income).sum::<i64>();
    let total_tax_due = trend.iter().map(|row| row.total_tax_due).sum::<i64>();
    let average_effective_tax_rate_bps = if total_taxable_income == 0 {
        0
    } else {
        total_tax_due * 10_000 / total_taxable_income
    };
    let average_effective_tax_rate_pct = if total_taxable_income == 0 {
        0.0
    } else {
        total_tax_due as f64 * 100.0 / total_taxable_income as f64
    };
    Ok(DashboardTaxBurdenKpiSummary {
        years,
        customer_id,
        trend,
        total_taxable_income,
        total_tax_due,
        average_effective_tax_rate_bps,
        average_effective_tax_rate_pct,
    })
}

pub async fn dashboard_industry_distribution(
    pool: &PgPool,
    tenant: &TenantRef,
    user: &AuthUser,
) -> Result<DashboardIndustryDistributionSummary> {
    let super_admin = user.roles.iter().any(|role| role == "SUPER_ADMIN");
    if user.tenant_id != tenant.tenant_id && !super_admin {
        anyhow::bail!("tenant access denied");
    }
    let all_access = super_admin
        || (user.tenant_id == tenant.tenant_id
            && user.roles.iter().any(|role| {
                matches!(
                    role.as_str(),
                    "TENANT_ADMIN" | "SYSTEM_ADMIN" | "SUPER_ADMIN"
                )
            }));
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        WITH visible_customers AS (
            SELECT a.customer_id
            FROM user_customer_access a
            WHERE a.user_id = $3
              AND a.tenant_id = $1
              AND a.access_level <> 'BLOCKED'
              AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
              AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
            UNION
            SELECT d.customer_id
            FROM access_delegations d
            WHERE d.tenant_id = $1
              AND d.delegatee_user_id = $3
              AND d.status = 'ACTIVE'
              AND (d.valid_from IS NULL OR d.valid_from <= CURRENT_DATE)
              AND (d.valid_to IS NULL OR d.valid_to >= CURRENT_DATE)
        ),
        filtered AS (
            SELECT
                COALESCE(NULLIF(TRIM(c.industry_code), ''), 'UNSPECIFIED') AS industry_code,
                c.customer_id
            FROM {schema}.customers c
            WHERE c.status = 'ACTIVE'
              AND ($2::BOOL OR c.customer_id IN (SELECT customer_id FROM visible_customers))
        )
        SELECT industry_code,
               COUNT(DISTINCT customer_id)::BIGINT AS customer_count
        FROM filtered
        GROUP BY industry_code
        ORDER BY customer_count DESC, industry_code ASC
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(tenant.tenant_id)
        .bind(all_access)
        .bind(user.user_id)
        .fetch_all(pool)
        .await
        .context("failed to load dashboard industry distribution")?;
    let total_customers = rows
        .iter()
        .map(|row| row.get::<i64, _>("customer_count"))
        .sum::<i64>();
    let industries = rows
        .into_iter()
        .map(|row| {
            let industry_code = row.get::<String, _>("industry_code");
            let customer_count = row.get::<i64, _>("customer_count");
            let percentage_bps = if total_customers == 0 {
                0
            } else {
                customer_count * 10_000 / total_customers
            };
            DashboardIndustryDistributionItem {
                industry_name: industry_display_name(&industry_code).to_string(),
                industry_code,
                customer_count,
                percentage_bps,
                percentage_pct: percentage_bps as f64 / 100.0,
            }
        })
        .collect();
    Ok(DashboardIndustryDistributionSummary {
        industries,
        total_customers,
    })
}

pub async fn dashboard_loss_expiry_kpi(
    pool: &PgPool,
    tenant: &TenantRef,
    user: &AuthUser,
    years: i64,
) -> Result<DashboardLossExpiryKpiSummary> {
    let super_admin = user.roles.iter().any(|role| role == "SUPER_ADMIN");
    if user.tenant_id != tenant.tenant_id && !super_admin {
        anyhow::bail!("tenant access denied");
    }
    let all_access = super_admin
        || (user.tenant_id == tenant.tenant_id
            && user.roles.iter().any(|role| {
                matches!(
                    role.as_str(),
                    "TENANT_ADMIN" | "SYSTEM_ADMIN" | "SUPER_ADMIN"
                )
            }));
    let years = years.clamp(1, 10);
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        WITH visible_customers AS (
            SELECT a.customer_id
            FROM user_customer_access a
            WHERE a.user_id = $3
              AND a.tenant_id = $1
              AND a.access_level <> 'BLOCKED'
              AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
              AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
            UNION
            SELECT d.customer_id
            FROM access_delegations d
            WHERE d.tenant_id = $1
              AND d.delegatee_user_id = $3
              AND d.status = 'ACTIVE'
              AND (d.valid_from IS NULL OR d.valid_from <= CURRENT_DATE)
              AND (d.valid_to IS NULL OR d.valid_to >= CURRENT_DATE)
        ),
        eligible AS (
            SELECT
                l.loss_id,
                l.customer_id,
                l.expires_year,
                l.remaining_amount
            FROM {schema}.carryforward_loss l
            JOIN {schema}.customers c ON c.customer_id = l.customer_id
            WHERE l.remaining_amount > 0
              AND l.expires_year BETWEEN EXTRACT(YEAR FROM CURRENT_DATE)::INT
                                     AND EXTRACT(YEAR FROM CURRENT_DATE)::INT + $4::INT - 1
              AND ($2::BOOL OR l.customer_id IN (SELECT customer_id FROM visible_customers))
        )
        SELECT
            expires_year,
            SUM(remaining_amount)::BIGINT AS total_amount,
            COUNT(DISTINCT customer_id)::BIGINT AS customer_count,
            COUNT(*)::BIGINT AS loss_count
        FROM eligible
        GROUP BY expires_year
        ORDER BY expires_year ASC
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(tenant.tenant_id)
        .bind(all_access)
        .bind(user.user_id)
        .bind(years)
        .fetch_all(pool)
        .await
        .context("failed to load dashboard loss expiry KPI")?;
    let buckets = rows
        .into_iter()
        .map(|row| DashboardLossExpiryKpiBucket {
            expires_year: row.get("expires_year"),
            total_amount: row.get("total_amount"),
            customer_count: row.get("customer_count"),
            loss_count: row.get("loss_count"),
        })
        .collect::<Vec<_>>();
    let total_amount = buckets.iter().map(|row| row.total_amount).sum::<i64>();
    let total_loss_count = buckets.iter().map(|row| row.loss_count).sum::<i64>();
    let total_customer_count = if buckets.is_empty() {
        0
    } else {
        sqlx::query_scalar::<_, i64>(&format!(
            r#"
            WITH visible_customers AS (
                SELECT a.customer_id
                FROM user_customer_access a
                WHERE a.user_id = $3
                  AND a.tenant_id = $1
                  AND a.access_level <> 'BLOCKED'
                  AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
                  AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
                UNION
                SELECT d.customer_id
                FROM access_delegations d
                WHERE d.tenant_id = $1
                  AND d.delegatee_user_id = $3
                  AND d.status = 'ACTIVE'
                  AND (d.valid_from IS NULL OR d.valid_from <= CURRENT_DATE)
                  AND (d.valid_to IS NULL OR d.valid_to >= CURRENT_DATE)
            )
            SELECT COUNT(DISTINCT l.customer_id)::BIGINT
            FROM {schema}.carryforward_loss l
            WHERE l.remaining_amount > 0
              AND l.expires_year BETWEEN EXTRACT(YEAR FROM CURRENT_DATE)::INT
                                     AND EXTRACT(YEAR FROM CURRENT_DATE)::INT + $4::INT - 1
              AND ($2::BOOL OR l.customer_id IN (SELECT customer_id FROM visible_customers))
            "#
        ))
        .bind(tenant.tenant_id)
        .bind(all_access)
        .bind(user.user_id)
        .bind(years)
        .fetch_one(pool)
        .await
        .context("failed to count dashboard loss expiry customers")?
    };
    Ok(DashboardLossExpiryKpiSummary {
        years,
        buckets,
        total_amount,
        total_customer_count,
        total_loss_count,
    })
}

fn industry_display_name(industry_code: &str) -> &'static str {
    match industry_code {
        "62010" => "Software development",
        "101" => "Cash",
        "UNSPECIFIED" => "Unspecified",
        _ => "Other",
    }
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
    let sql = format!(
        r#"
        SELECT b.by_id,
               b.customer_id,
               c.customer_name,
               b.year_label,
               b.start_date,
               b.end_date,
               b.status,
               al.approver_login_id,
               we.actor AS requester_login_id,
               we.created_at AS submitted_at,
               GREATEST(0, FLOOR(EXTRACT(EPOCH FROM (NOW() - COALESCE(we.created_at, b.updated_at))) / 86400))::BIGINT AS pending_days,
               'ws/appr:inbox'::TEXT AS route_key
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
            SELECT actor, created_at
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

pub async fn business_year_progress(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Value> {
    let by = get_business_year(pool, tenant, by_id).await?;
    let events = list_workflow_events(pool, tenant, by_id)
        .await
        .unwrap_or_default();
    let progress = progress_for_status(&by.status);
    let active_index = active_step_index(&by.status);
    let steps = [
        ("ws-start", "0. Start", "ws/start:customer-pick", 0),
        ("ws-info", "1. Input", "ws/info:fs", 20),
        ("ws-adj", "2. Adjust", "ws/adj:B1", 45),
        ("ws-form", "3. Forms", "ws/form:form3", 60),
        ("ws-val", "4. Validate", "ws/val:run", 70),
        ("ws-appr", "5. Approve", "ws/appr:request", 85),
        ("ws-print", "6. Print", "ws/print:preview", 92),
        ("ws-file", "7. E-file", "ws/file:precheck", 100),
    ];
    let step_values = steps
        .iter()
        .enumerate()
        .map(|(index, (code, label, leaf, threshold))| {
            json!({
                "code": code,
                "label": label,
                "leaf_key": leaf,
                "required": index > 0,
                "done": by.status == "FILED" || progress >= *threshold && index < active_index,
                "active": index == active_index,
                "pending": by.status != "FILED" && index > active_index
            })
        })
        .collect::<Vec<_>>();
    let next_leaf = match by.status.as_str() {
        "FILED" => "ws/file:done",
        "APPROVED" => "ws/print:preview",
        "IN_REVIEW" => "ws/appr:inbox",
        "AMENDED" => "ws/info:fs",
        _ => "ws/info:fs",
    };
    Ok(json!({
        "tenant_code": tenant.tenant_code,
        "by_id": by.by_id,
        "status": by.status,
        "lock_mode": by.lock_mode,
        "locked": by.locked_at.is_some(),
        "progress": progress,
        "steps": step_values,
        "next_leaf": next_leaf,
        "recommendations": [
            {
                "leaf_key": next_leaf,
                "label": "Next step",
                "enabled": by.status != "FILED",
                "reason": if by.status == "FILED" { "Filed work is complete" } else { "Continue the filing workflow" }
            }
        ],
        "workflow_event_count": events.len()
    }))
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
    let (original_by_id, amendment_sequence, amendment_reason, version_mode) =
        load_amendment_meta(pool, tenant, by_id).await?;
    let baseline_by_id = original_by_id.unwrap_or(by_id);
    let events = list_workflow_events(pool, tenant, by_id).await?;
    let original_events = if baseline_by_id == by_id {
        events.clone()
    } else {
        list_workflow_events(pool, tenant, baseline_by_id)
            .await
            .unwrap_or_default()
    };
    let filed_event = original_events
        .iter()
        .find(|event| event.to_status == "FILED");
    let mut differences = vec![
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
    if baseline_by_id != by_id {
        append_form_amendment_differences(pool, tenant, baseline_by_id, by_id, &mut differences)
            .await?;
    }
    Ok(AmendmentPreview {
        tenant_code: tenant.tenant_code.clone(),
        by_id,
        original_by_id,
        amendment_sequence,
        amendment_reason,
        version_mode,
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
        if let Some(existing) = find_open_amendment(pool, tenant, by_id).await? {
            return Ok(existing);
        }
        return create_amended_business_year(
            pool,
            tenant,
            &current,
            &actor,
            &reason,
            &version_mode,
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

pub async fn business_year_amendment_metadata(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Value> {
    let by = get_business_year(pool, tenant, by_id).await?;
    let (original_by_id, amendment_sequence, amendment_reason, version_mode) =
        load_amendment_meta(pool, tenant, by_id).await?;
    Ok(json!({
        "by_id": by.by_id,
        "current_status": by.status,
        "locked": by.locked_at.is_some(),
        "original_by_id": original_by_id,
        "amendment_sequence": amendment_sequence,
        "amendment_reason": amendment_reason,
        "version_mode": version_mode
    }))
}

async fn load_amendment_meta(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<(Option<i64>, i32, Option<String>, Option<String>)> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT original_by_id, amendment_sequence, amendment_reason, version_mode
        FROM {schema}.business_years
        WHERE by_id = $1
        "#
    );
    sqlx::query_as::<_, (Option<i64>, i32, Option<String>, Option<String>)>(&sql)
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("failed to load amendment metadata")
}

async fn find_open_amendment(
    pool: &PgPool,
    tenant: &TenantRef,
    original_by_id: i64,
) -> Result<Option<BusinessYear>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT by_id, customer_id, year_label, start_date, end_date, status,
               locked_at, lock_mode, created_at, updated_at
        FROM {schema}.business_years
        WHERE original_by_id = $1
          AND status <> 'FILED'
        ORDER BY amendment_sequence DESC, by_id DESC
        LIMIT 1
        "#
    );
    sqlx::query_as::<_, BusinessYear>(&sql)
        .bind(original_by_id)
        .fetch_optional(pool)
        .await
        .context("failed to find open amendment")
}

async fn create_amended_business_year(
    pool: &PgPool,
    tenant: &TenantRef,
    original: &BusinessYear,
    actor: &str,
    reason: &str,
    version_mode: &str,
) -> Result<BusinessYear> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sequence_sql = format!(
        r#"
        SELECT COALESCE(MAX(amendment_sequence), 0) + 1
        FROM {schema}.business_years
        WHERE customer_id = $1
          AND year_label = $2
          AND (by_id = $3 OR original_by_id = $3)
        "#
    );
    let amendment_sequence = sqlx::query_scalar::<_, i32>(&sequence_sql)
        .bind(original.customer_id)
        .bind(original.year_label)
        .bind(original.by_id)
        .fetch_one(pool)
        .await
        .context("failed to calculate amendment sequence")?;

    let insert_sql = format!(
        r#"
        INSERT INTO {schema}.business_years (
            customer_id, year_label, start_date, end_date, status, locked_at, lock_mode,
            original_by_id, amendment_sequence, amendment_reason, version_mode
        )
        VALUES ($1, $2, $3, $4, 'AMENDED', NULL, 'AMENDMENT_UNLOCK', $5, $6, $7, $8)
        RETURNING by_id, customer_id, year_label, start_date, end_date, status,
                  locked_at, lock_mode, created_at, updated_at
        "#
    );
    let amended = sqlx::query_as::<_, BusinessYear>(&insert_sql)
        .bind(original.customer_id)
        .bind(original.year_label)
        .bind(original.start_date)
        .bind(original.end_date)
        .bind(original.by_id)
        .bind(amendment_sequence)
        .bind(reason)
        .bind(version_mode)
        .fetch_one(pool)
        .await
        .context("failed to create amended business year")?;

    clone_amendment_baseline(pool, tenant, original.by_id, amended.by_id).await?;
    append_workflow_event(
        pool,
        tenant,
        amended.by_id,
        WorkflowEventRequest {
            action: Some("START_AMENDMENT".to_string()),
            actor: Some(actor.to_string()),
            comment: Some(reason.to_string()),
            to_status: Some("AMENDED".to_string()),
            metadata: Some(json!({
                "original_by_id": original.by_id,
                "amendment_sequence": amendment_sequence,
                "version_mode": version_mode
            })),
        },
    )
    .await?;
    insert_audit_log(
        pool,
        tenant,
        AuditLogEntry {
            table_name: "business_years",
            record_id: amended.by_id.to_string(),
            action: "CREATE_AMENDMENT",
            old_data: Some(json!({
                "original_by_id": original.by_id,
                "status": original.status,
                "locked_at": original.locked_at
            })),
            new_data: json!({
                "by_id": amended.by_id,
                "status": amended.status,
                "amendment_sequence": amendment_sequence,
                "reason": reason,
                "version_mode": version_mode
            }),
            changed_by: actor,
        },
    )
    .await?;
    Ok(amended)
}

async fn clone_amendment_baseline(
    pool: &PgPool,
    tenant: &TenantRef,
    source_by_id: i64,
    target_by_id: i64,
) -> Result<()> {
    let snapshot = crate::tax::clone_law_snapshot(pool, tenant, source_by_id, target_by_id).await?;
    let schema = quote_ident(&tenant.schema_name)?;
    let copy_forms = format!(
        r#"
        INSERT INTO {schema}.form_data (
            by_id, form_code, form_version_id, data_json, snapshot_id, status
        )
        SELECT $2, form_code, form_version_id,
               jsonb_set(data_json, '{{snapshot_id}}', to_jsonb($3::BIGINT), true),
               $3, status
        FROM {schema}.form_data
        WHERE by_id = $1
        ON CONFLICT (by_id, form_code) DO NOTHING
        "#
    );
    sqlx::query(&copy_forms)
        .bind(source_by_id)
        .bind(target_by_id)
        .bind(snapshot.snapshot_id)
        .execute(pool)
        .await
        .context("failed to copy amendment form baseline")?;

    let copy_adjustments = format!(
        r#"
        INSERT INTO {schema}.tax_adjustments (
            by_id, adj_category, adj_code, amount, direction, description, snapshot_id, metadata, status
        )
        SELECT $2, adj_category, adj_code, amount, direction, description, $3,
               metadata || jsonb_build_object('amended_from_adjustment_id', adjustment_id),
               status
        FROM {schema}.tax_adjustments
        WHERE by_id = $1
        "#
    );
    sqlx::query(&copy_adjustments)
        .bind(source_by_id)
        .bind(target_by_id)
        .bind(snapshot.snapshot_id)
        .execute(pool)
        .await
        .context("failed to copy amendment adjustments")?;

    let copy_items = format!(
        r#"
        INSERT INTO {schema}.adjustment_items (
            by_id, adjustment_id, section, item_code, item_name, amount, direction,
            disposition, source_module, law_ref, metadata
        )
        SELECT $2, NULL, section, item_code, item_name, amount, direction,
               disposition, source_module, law_ref,
               metadata || jsonb_build_object('amended_from_adjustment_item_id', adjustment_item_id)
        FROM {schema}.adjustment_items
        WHERE by_id = $1
        "#
    );
    sqlx::query(&copy_items)
        .bind(source_by_id)
        .bind(target_by_id)
        .execute(pool)
        .await
        .context("failed to copy amendment adjustment items")?;

    let copy_reserves = format!(
        r#"
        INSERT INTO {schema}.reserves (
            by_id, adjustment_id, reserve_code, amount, direction, carryforward_to, source_module
        )
        SELECT $2, NULL, reserve_code, amount, direction, carryforward_to, source_module
        FROM {schema}.reserves
        WHERE by_id = $1
        "#
    );
    sqlx::query(&copy_reserves)
        .bind(source_by_id)
        .bind(target_by_id)
        .execute(pool)
        .await
        .context("failed to copy amendment reserves")?;

    let copy_assets = format!(
        r#"
        INSERT INTO {schema}.assets (
            by_id, batch_id, asset_code, asset_name, asset_category, is_business_vehicle,
            acquisition_date, acquisition_cost, useful_life_years
        )
        SELECT $2, NULL, asset_code, asset_name, asset_category, is_business_vehicle,
               acquisition_date, acquisition_cost, useful_life_years
        FROM {schema}.assets
        WHERE by_id = $1
        "#
    );
    sqlx::query(&copy_assets)
        .bind(source_by_id)
        .bind(target_by_id)
        .execute(pool)
        .await
        .context("failed to copy amendment asset baseline")?;

    let copy_transactions = format!(
        r#"
        INSERT INTO {schema}.transactions (
            by_id, batch_id, tx_date, partner_name, category, account_code,
            description, amount, evidence_type
        )
        SELECT $2, NULL, tx_date, partner_name, category, account_code,
               description, amount, evidence_type
        FROM {schema}.transactions
        WHERE by_id = $1
        "#
    );
    sqlx::query(&copy_transactions)
        .bind(source_by_id)
        .bind(target_by_id)
        .execute(pool)
        .await
        .context("failed to copy amendment transaction baseline")?;

    let copy_financial_statements = format!(
        r#"
        WITH source_fs AS (
            SELECT fs_id, statement_type, currency
            FROM {schema}.financial_statements
            WHERE by_id = $1
        ),
        inserted_fs AS (
            INSERT INTO {schema}.financial_statements (by_id, batch_id, statement_type, currency)
            SELECT $2, NULL, statement_type, currency
            FROM source_fs
            RETURNING fs_id, statement_type
        )
        INSERT INTO {schema}.fs_lines (
            fs_id, batch_id, row_no, account_code, account_name, standard_account_code,
            standard_account_name, amount, debit_credit
        )
        SELECT inserted_fs.fs_id, NULL, lines.row_no, lines.account_code, lines.account_name,
               lines.standard_account_code, lines.standard_account_name, lines.amount,
               lines.debit_credit
        FROM source_fs
        JOIN {schema}.fs_lines lines ON lines.fs_id = source_fs.fs_id
        JOIN inserted_fs ON inserted_fs.statement_type = source_fs.statement_type
        "#
    );
    sqlx::query(&copy_financial_statements)
        .bind(source_by_id)
        .bind(target_by_id)
        .execute(pool)
        .await
        .context("failed to copy amendment financial statement baseline")?;
    Ok(())
}

async fn append_form_amendment_differences(
    pool: &PgPool,
    tenant: &TenantRef,
    original_by_id: i64,
    current_by_id: i64,
    differences: &mut Vec<AmendmentDiff>,
) -> Result<()> {
    let original_forms = form_payloads_by_code(pool, tenant, original_by_id).await?;
    let current_forms = form_payloads_by_code(pool, tenant, current_by_id).await?;
    for (form_code, current_data) in current_forms {
        let Some(original_data) = original_forms.get(&form_code) else {
            differences.push(AmendmentDiff {
                area: "FORM".to_string(),
                field: form_code,
                original_value: Value::Null,
                current_value: current_data,
                description: "수정신고에서 새로 생성된 서식".to_string(),
            });
            continue;
        };
        let Some(current_object) = current_data.as_object() else {
            continue;
        };
        let original_object = original_data.as_object();
        for (field, current_value) in current_object {
            if field == "_meta" || field == "snapshot_id" {
                continue;
            }
            let original_value = original_object
                .and_then(|object| object.get(field))
                .cloned()
                .unwrap_or(Value::Null);
            if original_value != current_value.clone() {
                differences.push(AmendmentDiff {
                    area: "FORM".to_string(),
                    field: format!("{form_code}.{field}"),
                    original_value,
                    current_value: current_value.clone(),
                    description: "원신고 대비 수정신고 서식 값 변경".to_string(),
                });
            }
        }
    }
    Ok(())
}

async fn form_payloads_by_code(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<HashMap<String, Value>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT form_code, data_json
        FROM {schema}.form_data
        WHERE by_id = $1
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to load form payloads for amendment diff")?;
    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.get::<String, _>("form_code"),
                row.get::<Value, _>("data_json"),
            )
        })
        .collect())
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

fn progress_for_status(status: &str) -> i32 {
    match status {
        "DRAFT" => 20,
        "IN_REVIEW" => 85,
        "APPROVED" => 92,
        "FILED" => 100,
        "AMENDED" => 35,
        _ => 0,
    }
}

fn active_step_index(status: &str) -> usize {
    match status {
        "DRAFT" => 1,
        "IN_REVIEW" => 5,
        "APPROVED" => 6,
        "FILED" => 7,
        "AMENDED" => 1,
        _ => 0,
    }
}
