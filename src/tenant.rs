use anyhow::{Context, Result};
use sqlx::PgPool;

use crate::{
    db::{execute_batch, quote_ident},
    domain::{
        BusinessYear, CreateBusinessYearRequest, CreateCustomerRequest, CreateTenantRequest,
        Customer, Tenant, TenantRef,
    },
};

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
            status          VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(tenant_id, customer_code)
        );

        CREATE TABLE IF NOT EXISTS {schema}.business_years (
            by_id           BIGSERIAL PRIMARY KEY,
            customer_id     BIGINT NOT NULL REFERENCES {schema}.customers(customer_id),
            year_label      INT NOT NULL,
            start_date      DATE NOT NULL,
            end_date        DATE NOT NULL,
            status          VARCHAR(20) NOT NULL DEFAULT 'OPEN',
            locked_at       TIMESTAMPTZ,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CHECK (start_date <= end_date),
            UNIQUE(customer_id, year_label)
        );

        CREATE TABLE IF NOT EXISTS {schema}.financial_statements (
            fs_id           BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            statement_type  VARCHAR(30) NOT NULL,
            currency        VARCHAR(3) NOT NULL DEFAULT 'KRW',
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS {schema}.fs_lines (
            line_id         BIGSERIAL PRIMARY KEY,
            fs_id           BIGINT NOT NULL REFERENCES {schema}.financial_statements(fs_id),
            account_code    VARCHAR(50) NOT NULL,
            account_name    VARCHAR(200) NOT NULL,
            amount          BIGINT NOT NULL,
            debit_credit    VARCHAR(10) NOT NULL
        );

        CREATE TABLE IF NOT EXISTS {schema}.assets (
            asset_id        BIGSERIAL PRIMARY KEY,
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            asset_code      VARCHAR(50) NOT NULL,
            asset_name      VARCHAR(200) NOT NULL,
            acquisition_date DATE NOT NULL,
            acquisition_cost BIGINT NOT NULL,
            useful_life_years INT NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS {schema}.depreciation (
            depreciation_id BIGSERIAL PRIMARY KEY,
            asset_id        BIGINT NOT NULL REFERENCES {schema}.assets(asset_id),
            by_id           BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            book_amount     BIGINT NOT NULL,
            tax_limit       BIGINT NOT NULL,
            adjustment_amount BIGINT NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

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

        CREATE TABLE IF NOT EXISTS {schema}.reserves (
            reserve_id     BIGSERIAL PRIMARY KEY,
            by_id          BIGINT NOT NULL REFERENCES {schema}.business_years(by_id),
            adjustment_id  BIGINT REFERENCES {schema}.tax_adjustments(adjustment_id),
            reserve_code   VARCHAR(50) NOT NULL,
            amount         BIGINT NOT NULL,
            direction      VARCHAR(20) NOT NULL,
            carryforward_to INT,
            created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS {schema}.carryforward_loss (
            loss_id        BIGSERIAL PRIMARY KEY,
            customer_id    BIGINT NOT NULL REFERENCES {schema}.customers(customer_id),
            origin_year    INT NOT NULL,
            original_amount BIGINT NOT NULL,
            remaining_amount BIGINT NOT NULL,
            expires_year   INT NOT NULL,
            created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
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

        CREATE TABLE IF NOT EXISTS {schema}.audit_logs (
            audit_id        BIGSERIAL PRIMARY KEY,
            table_name      VARCHAR(100) NOT NULL,
            record_id       VARCHAR(100) NOT NULL,
            action          VARCHAR(20) NOT NULL,
            old_data        JSONB,
            new_data        JSONB,
            changed_by      VARCHAR(100) NOT NULL DEFAULT 'system',
            changed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
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
    let sql = format!(
        r#"
        INSERT INTO {schema}.customers (
            tenant_id, customer_code, customer_name, biz_reg_no, corp_reg_no, industry_code, is_sme
        )
        VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, FALSE))
        RETURNING customer_id, tenant_id, customer_code, customer_name, biz_reg_no, corp_reg_no,
                  industry_code, is_sme, status, created_at, updated_at
        "#
    );

    sqlx::query_as::<_, Customer>(&sql)
        .bind(tenant.tenant_id)
        .bind(request.customer_code.trim())
        .bind(request.customer_name.trim())
        .bind(request.biz_reg_no.trim())
        .bind(request.corp_reg_no)
        .bind(request.industry_code)
        .bind(request.is_sme)
        .fetch_one(pool)
        .await
        .context("failed to create customer")
}

pub async fn list_customers(pool: &PgPool, tenant: &TenantRef) -> Result<Vec<Customer>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT customer_id, tenant_id, customer_code, customer_name, biz_reg_no, corp_reg_no,
               industry_code, is_sme, status, created_at, updated_at
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

    sqlx::query_as::<_, BusinessYear>(&sql)
        .bind(request.customer_id)
        .bind(request.year_label)
        .bind(request.start_date)
        .bind(request.end_date)
        .fetch_one(pool)
        .await
        .context("failed to create business year")
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
