use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use serde_json::json;
use sqlx::PgPool;

use crate::{
    db::quote_ident,
    domain::{
        AssetBasedAdjustmentRequest, CalculateAdjustmentRequest, CapitalChangeInput,
        ConsolidatedEntityInput, ConsolidationEliminationInput, CreateIncomeAdjustmentRequest,
        CreateVehicleUsageLogRequest, EvaluationAdjustmentRequest, ForeignIncomeInput,
        IncomeAdjustmentItemInput, LossCarryforwardInput, PenaltyTaxInput,
        SpecialTaxAdjustmentRequest, TaxAmountAdjustmentRequest, TaxCreditInput, TenantRef,
        TransactionBasedAdjustmentRequest, ValuationPositionInput,
    },
    efiling, menu, tax, tax_data, tenant, validation_rules,
};

const DEMO_PASSWORD: &str = "ChangeMe123!";
const ALL_WORK_SCOPES: &[&str] = &[
    "INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST",
];

#[derive(Debug, Clone)]
pub struct DemoSeedOptions {
    pub tenant_code: String,
    pub reset: bool,
    pub admin_password: String,
}

impl Default for DemoSeedOptions {
    fn default() -> Self {
        Self {
            tenant_code: "demo".to_string(),
            reset: false,
            admin_password: DEMO_PASSWORD.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DemoSeedResult {
    pub tenant_code: String,
    pub main_by_id: i64,
    pub filed_by_id: i64,
    pub customer_count: i64,
    pub business_year_count: i64,
    pub user_count: i64,
    pub menu_node_count: usize,
    pub efiling_id: i64,
    pub validation_issue_count: usize,
    pub validation_error_count: usize,
}

pub async fn run_demo_seed(pool: &PgPool, options: DemoSeedOptions) -> Result<DemoSeedResult> {
    let tenant_code = tenant::normalize_tenant_code(&options.tenant_code)?;
    if tenant_code != "demo" {
        return Err(anyhow!("demo seed is restricted to tenant_code=demo"));
    }

    let tenant_ref = ensure_demo_tenant(pool, &tenant_code).await?;
    ensure_sample_tenant(pool).await?;
    tenant::provision_tenant_schema(pool, &tenant_ref.schema_name).await?;

    if options.reset {
        reset_demo_schema(pool, &tenant_ref).await?;
    }

    ensure_seed_roles(pool).await?;
    let users = ensure_seed_users(pool, &tenant_ref, &options.admin_password).await?;
    let customers = ensure_seed_customers(pool, &tenant_ref).await?;
    ensure_user_customer_access(pool, &tenant_ref, &users, &customers).await?;
    let years = ensure_seed_business_years(pool, &tenant_ref, &customers).await?;
    let main_by_id = *years
        .get("CUST01-2026")
        .context("main demo business year missing")?;
    let filed_by_id = *years
        .get("CUST01-2024")
        .context("filed demo business year missing")?;

    for by_id in years.values() {
        tax::ensure_law_snapshot(pool, &tenant_ref, *by_id).await?;
    }
    clear_seeded_business_year_data(pool, &tenant_ref, &years).await?;
    set_seed_filed_lock(pool, &tenant_ref, filed_by_id, false).await?;

    seed_tax_data(pool, &tenant_ref, main_by_id).await?;
    run_adjustment_suite(pool, &tenant_ref, main_by_id).await?;
    seed_report_years(pool, &tenant_ref, &years).await?;
    set_seed_filed_lock(pool, &tenant_ref, filed_by_id, true).await?;
    seed_workflow(pool, &tenant_ref, main_by_id, filed_by_id).await?;
    seed_notifications(pool, &tenant_ref, main_by_id, filed_by_id).await?;
    seed_audit_logs(pool, &tenant_ref, main_by_id).await?;

    let efile = efiling::generate_efiling(pool, &tenant_ref, main_by_id).await?;
    let validation = validation_rules::run_validation(pool, &tenant_ref, main_by_id).await?;
    let menu_nodes = menu::list_menu_nodes(pool).await?;

    let customer_count = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT COUNT(*) FROM {}.customers",
        quote_ident(&tenant_ref.schema_name)?
    ))
    .fetch_one(pool)
    .await
    .context("failed to count seeded customers")?;
    let business_year_count = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT COUNT(*) FROM {}.business_years",
        quote_ident(&tenant_ref.schema_name)?
    ))
    .fetch_one(pool)
    .await
    .context("failed to count seeded business years")?;

    Ok(DemoSeedResult {
        tenant_code,
        main_by_id,
        filed_by_id,
        customer_count,
        business_year_count,
        user_count: users.len() as i64,
        menu_node_count: menu_nodes.len(),
        efiling_id: efile.efiling_id,
        validation_issue_count: validation.issues.len(),
        validation_error_count: validation.error_count,
    })
}

async fn set_seed_filed_lock(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    by_id: i64,
    locked: bool,
) -> Result<()> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.business_years
        SET status = CASE WHEN $2 THEN 'FILED' ELSE 'APPROVED' END,
            locked_at = CASE WHEN $2 THEN COALESCE(locked_at, NOW()) ELSE NULL END,
            lock_mode = CASE WHEN $2 THEN 'FILED_LOCK' ELSE 'SEED_UNLOCK' END,
            updated_at = NOW()
        WHERE by_id = $1
        "#
    );
    sqlx::query(&sql)
        .bind(by_id)
        .bind(locked)
        .execute(pool)
        .await
        .context("failed to update seed filed lock")?;
    Ok(())
}

async fn ensure_demo_tenant(pool: &PgPool, tenant_code: &str) -> Result<TenantRef> {
    let schema_name = format!("tenant_{tenant_code}");
    sqlx::query_as::<_, TenantRef>(
        r#"
        INSERT INTO tenants (
            tenant_code, tenant_name, biz_reg_no, contract_start,
            contract_end, schema_name, max_users, status
        )
        VALUES ($1, 'Demo Corporate Tax Workspace', '1108112345', DATE '2026-01-01',
                NULL, $2, 20, 'ACTIVE')
        ON CONFLICT (tenant_code) DO UPDATE
        SET tenant_name = EXCLUDED.tenant_name,
            biz_reg_no = EXCLUDED.biz_reg_no,
            schema_name = EXCLUDED.schema_name,
            max_users = EXCLUDED.max_users,
            status = 'ACTIVE',
            updated_at = NOW()
        RETURNING tenant_id, tenant_code, schema_name
        "#,
    )
    .bind(tenant_code)
    .bind(schema_name)
    .fetch_one(pool)
    .await
    .context("failed to upsert demo tenant")
}

async fn ensure_sample_tenant(pool: &PgPool) -> Result<()> {
    let tenant = sqlx::query_as::<_, TenantRef>(
        r#"
        INSERT INTO tenants (
            tenant_code, tenant_name, biz_reg_no, contract_start,
            contract_end, schema_name, max_users, status
        )
        VALUES ('samplefirm', 'Sample Advisory Firm', '2208112345', DATE '2026-01-01',
                NULL, 'tenant_samplefirm', 10, 'ACTIVE')
        ON CONFLICT (tenant_code) DO UPDATE
        SET tenant_name = EXCLUDED.tenant_name,
            status = 'ACTIVE',
            updated_at = NOW()
        RETURNING tenant_id, tenant_code, schema_name
        "#,
    )
    .fetch_one(pool)
    .await
    .context("failed to upsert sample tenant")?;
    tenant::provision_tenant_schema(pool, &tenant.schema_name).await
}

async fn reset_demo_schema(pool: &PgPool, tenant_ref: &TenantRef) -> Result<()> {
    if tenant_ref.tenant_code != "demo" || tenant_ref.schema_name != "tenant_demo" {
        return Err(anyhow!("refusing to reset non-demo tenant schema"));
    }
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let tables = sqlx::query_scalar::<_, String>(
        "SELECT tablename FROM pg_tables WHERE schemaname = $1 ORDER BY tablename",
    )
    .bind(&tenant_ref.schema_name)
    .fetch_all(pool)
    .await
    .context("failed to enumerate demo schema tables")?;
    if !tables.is_empty() {
        let qualified = tables
            .iter()
            .map(|table| quote_ident(table).map(|table| format!("{schema}.{table}")))
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        sqlx::query(&format!(
            "TRUNCATE TABLE {qualified} RESTART IDENTITY CASCADE"
        ))
        .execute(pool)
        .await
        .context("failed to truncate demo tenant schema")?;
    }
    sqlx::query("DELETE FROM user_customer_access WHERE tenant_id = $1")
        .bind(tenant_ref.tenant_id)
        .execute(pool)
        .await
        .context("failed to reset demo customer access")?;
    Ok(())
}

async fn ensure_seed_roles(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO roles (role_code, role_name, description, system_role)
        VALUES
            ('TAX_WRITER', 'Tax Writer', 'Demo data entry and form writer', TRUE),
            ('TAX_REVIEWER', 'Tax Reviewer', 'Demo reviewer and approver', TRUE),
            ('TAX_EXPERT', 'Tax Expert', 'Demo tax adjustment expert', TRUE),
            ('TENANT_ADMIN', 'Tenant Admin', 'Demo tenant administrator', TRUE),
            ('SUPER_ADMIN', 'Super Admin', 'Demo super administrator', TRUE)
        ON CONFLICT (role_code) DO UPDATE
        SET role_name = EXCLUDED.role_name,
            description = EXCLUDED.description,
            system_role = EXCLUDED.system_role
        "#,
    )
    .execute(pool)
    .await
    .context("failed to upsert seed roles")?;

    for role in [
        "TAX_WRITER",
        "TAX_REVIEWER",
        "TAX_EXPERT",
        "TENANT_ADMIN",
        "SUPER_ADMIN",
    ] {
        for (module, function) in [
            ("customer", "READ"),
            ("tax-data", "WRITE"),
            ("adjustment", "WRITE"),
            ("forms", "WRITE"),
            ("validation", "RUN"),
            ("workflow", "APPROVE"),
            ("efiling", "EFILE"),
            ("reports", "READ"),
            ("admin", "USER"),
        ] {
            sqlx::query(
                r#"
                INSERT INTO role_permissions (role_code, module_code, function_code, effect)
                VALUES ($1, $2, $3, 'ALLOW')
                ON CONFLICT (role_code, module_code, function_code) DO UPDATE
                SET effect = 'ALLOW', updated_at = NOW()
                "#,
            )
            .bind(role)
            .bind(module)
            .bind(function)
            .execute(pool)
            .await
            .context("failed to upsert role permission")?;
        }
    }
    Ok(())
}

async fn ensure_seed_users(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    admin_password: &str,
) -> Result<HashMap<String, i64>> {
    let user_seeds = [
        (
            "admin",
            "Demo Admin",
            "admin.demo@example.test",
            admin_password,
            vec!["SUPER_ADMIN", "TENANT_ADMIN"],
        ),
        (
            "writer01",
            "Demo Writer",
            "writer01.demo@example.test",
            DEMO_PASSWORD,
            vec!["TAX_WRITER", "TAX_EXPERT"],
        ),
        (
            "reviewer01",
            "Demo Reviewer",
            "reviewer01.demo@example.test",
            DEMO_PASSWORD,
            vec!["TAX_REVIEWER"],
        ),
        (
            "tax01",
            "Demo Tax Expert",
            "tax01.demo@example.test",
            DEMO_PASSWORD,
            vec!["TAX_EXPERT"],
        ),
    ];
    let mut users = HashMap::new();
    for (login_id, name, email, password, roles) in user_seeds {
        let user_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO users (
                tenant_id, login_id, password_hash, user_name, email,
                use_2fa, status, pwd_changed_at, pwd_fail_count, locked
            )
            VALUES ($1, $2, crypt($3, gen_salt('bf')), $4, $5,
                    FALSE, 'ACTIVE', NOW(), 0, FALSE)
            ON CONFLICT (tenant_id, login_id) DO UPDATE
            SET password_hash = EXCLUDED.password_hash,
                user_name = EXCLUDED.user_name,
                email = EXCLUDED.email,
                use_2fa = FALSE,
                status = 'ACTIVE',
                pwd_fail_count = 0,
                locked = FALSE,
                pwd_changed_at = NOW()
            RETURNING user_id
            "#,
        )
        .bind(tenant_ref.tenant_id)
        .bind(login_id)
        .bind(password)
        .bind(name)
        .bind(email)
        .fetch_one(pool)
        .await
        .context("failed to upsert seed user")?;
        for role in roles {
            sqlx::query(
                r#"
                INSERT INTO user_roles (user_id, role_code, granted_by)
                VALUES ($1, $2, 'seed-demo')
                ON CONFLICT (user_id, role_code) DO NOTHING
                "#,
            )
            .bind(user_id)
            .bind(role)
            .execute(pool)
            .await
            .context("failed to grant seed role")?;
        }
        users.insert(login_id.to_string(), user_id);
    }
    Ok(users)
}

async fn ensure_seed_customers(
    pool: &PgPool,
    tenant_ref: &TenantRef,
) -> Result<HashMap<String, i64>> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let scopes = ALL_WORK_SCOPES
        .iter()
        .map(|scope| (*scope).to_string())
        .collect::<Vec<_>>();
    let customer_seeds = [
        (
            "CUST01",
            "Alpha Manufacturing Co.",
            "1208111111",
            "1101111111111",
            "C25999",
            true,
        ),
        (
            "CUST02",
            "Beta Platform Services",
            "2208122222",
            "2202222222222",
            "J58222",
            false,
        ),
        (
            "CUST03",
            "Gamma Bio Labs",
            "3208133333",
            "3303333333333",
            "M70113",
            true,
        ),
    ];
    let sql = format!(
        r#"
        INSERT INTO {schema}.customers (
            tenant_id, customer_code, customer_name, biz_reg_no,
            corp_reg_no, industry_code, is_sme, work_scopes, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'ACTIVE')
        ON CONFLICT (tenant_id, customer_code) DO UPDATE
        SET customer_name = EXCLUDED.customer_name,
            biz_reg_no = EXCLUDED.biz_reg_no,
            corp_reg_no = EXCLUDED.corp_reg_no,
            industry_code = EXCLUDED.industry_code,
            is_sme = EXCLUDED.is_sme,
            work_scopes = EXCLUDED.work_scopes,
            status = 'ACTIVE',
            updated_at = NOW()
        RETURNING customer_id
        "#
    );
    let mut customers = HashMap::new();
    for (code, name, biz_reg_no, corp_reg_no, industry_code, is_sme) in customer_seeds {
        let customer_id = sqlx::query_scalar::<_, i64>(&sql)
            .bind(tenant_ref.tenant_id)
            .bind(code)
            .bind(name)
            .bind(biz_reg_no)
            .bind(corp_reg_no)
            .bind(industry_code)
            .bind(is_sme)
            .bind(scopes.clone())
            .fetch_one(pool)
            .await
            .context("failed to upsert demo customer")?;
        customers.insert(code.to_string(), customer_id);
    }
    Ok(customers)
}

async fn ensure_user_customer_access(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    users: &HashMap<String, i64>,
    customers: &HashMap<String, i64>,
) -> Result<()> {
    sqlx::query("DELETE FROM user_customer_access WHERE tenant_id = $1")
        .bind(tenant_ref.tenant_id)
        .execute(pool)
        .await
        .context("failed to clear demo user access")?;
    for (login_id, user_id) in users {
        for customer_id in customers.values() {
            let access_level = if login_id == "reviewer01" {
                "REVIEWER"
            } else if login_id == "admin" {
                "OWNER"
            } else {
                "CO_WORKER"
            };
            let access_id = sqlx::query_scalar::<_, i64>(
                r#"
                INSERT INTO user_customer_access (
                    user_id, tenant_id, customer_id, access_level, is_primary, valid_from
                )
                VALUES ($1, $2, $3, $4, $5, DATE '2026-01-01')
                ON CONFLICT (user_id, tenant_id, customer_id) DO UPDATE
                SET access_level = EXCLUDED.access_level,
                    is_primary = EXCLUDED.is_primary,
                    updated_at = NOW()
                RETURNING access_id
                "#,
            )
            .bind(user_id)
            .bind(tenant_ref.tenant_id)
            .bind(customer_id)
            .bind(access_level)
            .bind(*customer_id == customers["CUST01"])
            .fetch_one(pool)
            .await
            .context("failed to upsert user customer access")?;
            for scope in ALL_WORK_SCOPES {
                sqlx::query(
                    r#"
                    INSERT INTO user_customer_work_scope (access_id, work_scope)
                    VALUES ($1, $2)
                    ON CONFLICT (access_id, work_scope) DO NOTHING
                    "#,
                )
                .bind(access_id)
                .bind(scope)
                .execute(pool)
                .await
                .context("failed to grant customer work scope")?;
            }
        }
    }
    Ok(())
}

async fn ensure_seed_business_years(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    customers: &HashMap<String, i64>,
) -> Result<HashMap<String, i64>> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let year_seeds = [
        (
            "CUST01-2024",
            "CUST01",
            2024,
            "2024-01-01",
            "2024-12-31",
            "FILED",
            true,
        ),
        (
            "CUST01-2025",
            "CUST01",
            2025,
            "2025-01-01",
            "2025-12-31",
            "APPROVED",
            false,
        ),
        (
            "CUST01-2026",
            "CUST01",
            2026,
            "2026-01-01",
            "2026-06-10",
            "IN_REVIEW",
            false,
        ),
        (
            "CUST02-2025",
            "CUST02",
            2025,
            "2025-01-01",
            "2025-12-31",
            "DRAFT",
            false,
        ),
        (
            "CUST03-2025",
            "CUST03",
            2025,
            "2025-01-01",
            "2025-12-31",
            "APPROVED",
            false,
        ),
    ];
    let sql = format!(
        r#"
        INSERT INTO {schema}.business_years (
            customer_id, year_label, start_date, end_date, status, locked_at
        )
        VALUES ($1, $2, $3, $4, $5, CASE WHEN $6 THEN NOW() ELSE NULL END)
        ON CONFLICT (customer_id, year_label) DO UPDATE
        SET start_date = EXCLUDED.start_date,
            end_date = EXCLUDED.end_date,
            status = EXCLUDED.status,
            locked_at = EXCLUDED.locked_at,
            updated_at = NOW()
        RETURNING by_id
        "#
    );
    let mut years = HashMap::new();
    for (key, customer_code, year_label, start_date, end_date, status, locked) in year_seeds {
        let by_id = sqlx::query_scalar::<_, i64>(&sql)
            .bind(customers[customer_code])
            .bind(year_label)
            .bind(date(start_date)?)
            .bind(date(end_date)?)
            .bind(status)
            .bind(locked)
            .fetch_one(pool)
            .await
            .context("failed to upsert demo business year")?;
        years.insert(key.to_string(), by_id);
    }
    Ok(years)
}

async fn clear_seeded_business_year_data(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    years: &HashMap<String, i64>,
) -> Result<()> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let by_ids = years.values().copied().collect::<Vec<_>>();
    if by_ids.is_empty() {
        return Ok(());
    }

    for sql in [
        format!(
            "DELETE FROM {schema}.efiling_validation v USING {schema}.efiling_history h WHERE v.efiling_id = h.efiling_id AND h.by_id = ANY($1)"
        ),
        format!(
            "DELETE FROM {schema}.efiling_files f USING {schema}.efiling_history h WHERE f.efiling_id = h.efiling_id AND h.by_id = ANY($1)"
        ),
        format!("DELETE FROM {schema}.efiling_history WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.form_data_history WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.form_data_migration_history WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.form_data WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.validation_issues WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.approval_lines WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.workflow_events WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.notifications WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.vehicle_usage_logs WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.depreciation WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.assets WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.transactions WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.fs_lines WHERE fs_id IN (SELECT fs_id FROM {schema}.financial_statements WHERE by_id = ANY($1))"),
        format!("DELETE FROM {schema}.financial_statements WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.import_batches WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.donation_carryforwards WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.entertainment_revenue_breakdowns WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.loan_interest_facts WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.valuation_positions WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.capital_changes WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.tax_credit_claims WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.minimum_tax_results WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.penalty_tax_items WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.foreign_income_items WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.consolidated_entities WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.consolidation_eliminations WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.reserves WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.adjustment_items WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.tax_adjustments WHERE by_id = ANY($1)"),
        format!("DELETE FROM {schema}.by_law_snapshot WHERE by_id = ANY($1)"),
    ] {
        sqlx::query(&sql)
            .bind(&by_ids)
            .execute(pool)
            .await
            .with_context(|| format!("failed to clear seeded data with statement: {sql}"))?;
    }

    let customer_ids = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT DISTINCT customer_id FROM {schema}.business_years WHERE by_id = ANY($1)"
    ))
    .bind(&by_ids)
    .fetch_all(pool)
    .await
    .context("failed to resolve demo customer ids for loss reset")?;
    if !customer_ids.is_empty() {
        sqlx::query(&format!(
            "DELETE FROM {schema}.carryforward_loss WHERE customer_id = ANY($1)"
        ))
        .bind(customer_ids)
        .execute(pool)
        .await
        .context("failed to clear demo loss carryforwards")?;
    }
    Ok(())
}

async fn seed_tax_data(pool: &PgPool, tenant_ref: &TenantRef, by_id: i64) -> Result<()> {
    for (data_type, file_name, csv) in [
        (
            "financial-statements",
            "demo-financial-statements.csv",
            financial_statement_csv(),
        ),
        ("assets", "demo-assets.csv", asset_csv()),
        ("transactions", "demo-transactions.csv", transaction_csv()),
    ] {
        let result = tax_data::import_tax_data(
            pool,
            tenant_ref,
            by_id,
            data_type,
            Some(file_name.to_string()),
            csv.as_bytes(),
        )
        .await
        .with_context(|| format!("failed to import demo {data_type}"))?;
        if !result.errors.is_empty() {
            return Err(anyhow!(
                "demo {data_type} import produced validation errors"
            ));
        }
    }

    let vehicles = tax_data::list_assets(pool, tenant_ref, by_id)
        .await?
        .into_iter()
        .filter(|asset| asset.is_business_vehicle)
        .take(3)
        .collect::<Vec<_>>();
    for (index, asset) in vehicles.iter().enumerate() {
        tax::create_vehicle_usage_log(
            pool,
            tenant_ref,
            by_id,
            CreateVehicleUsageLogRequest {
                asset_id: asset.asset_id,
                usage_month: date(&format!("2026-0{}-01", index + 1))?,
                total_distance_km: 2200.0 + (index as f64 * 300.0),
                business_distance_km: 1700.0 + (index as f64 * 240.0),
            },
        )
        .await?;
    }
    Ok(())
}

async fn run_adjustment_suite(pool: &PgPool, tenant_ref: &TenantRef, by_id: i64) -> Result<()> {
    calculate_core(pool, tenant_ref, by_id, 320_000_000, 2_450_000_000).await?;
    tax::calculate_income_adjustment(
        pool,
        tenant_ref,
        by_id,
        CreateIncomeAdjustmentRequest {
            accounting_income: Some(320_000_000),
            items: vec![
                income_item(
                    "GROSS_INCLUSION",
                    "B1_BONUS_ACCRUAL",
                    "Accrued officer bonus",
                    18_000_000,
                    true,
                ),
                income_item(
                    "LOSS_DISALLOWANCE",
                    "B1_NONBUSINESS_EXPENSE",
                    "Non-business expense",
                    7_500_000,
                    false,
                ),
                income_item(
                    "GROSS_EXCLUSION",
                    "B1_TAX_EXEMPT_INCOME",
                    "Tax-exempt interest income",
                    3_000_000,
                    false,
                ),
                income_item(
                    "LOSS_INCLUSION",
                    "B1_PRIOR_RESERVE_REVERSAL",
                    "Prior reserve reversal",
                    4_000_000,
                    false,
                ),
            ],
        },
    )
    .await?;
    tax::calculate_transaction_based_adjustment(
        pool,
        tenant_ref,
        by_id,
        "B2",
        TransactionBasedAdjustmentRequest {
            accounting_income: Some(320_000_000),
            taxable_income_before_donation: Some(80_000_000),
            gross_revenue: Some(2_450_000_000),
            revenue_breakdowns: None,
            weighted_average_loan_balance: None,
            weighted_average_interest_rate_bps: None,
            manual_interest_disallowance: None,
        },
    )
    .await?;
    tax::calculate_transaction_based_adjustment(
        pool,
        tenant_ref,
        by_id,
        "B3",
        TransactionBasedAdjustmentRequest {
            accounting_income: Some(320_000_000),
            taxable_income_before_donation: None,
            gross_revenue: Some(2_450_000_000),
            revenue_breakdowns: None,
            weighted_average_loan_balance: None,
            weighted_average_interest_rate_bps: None,
            manual_interest_disallowance: None,
        },
    )
    .await?;
    for (module, request) in [
        ("B4", empty_asset_request()),
        (
            "B5",
            AssetBasedAdjustmentRequest {
                book_reserve: Some(60_000_000),
                estimated_liability: Some(80_000_000),
                external_fund: Some(10_000_000),
                ..empty_asset_request()
            },
        ),
        (
            "B6",
            AssetBasedAdjustmentRequest {
                book_reserve: Some(9_000_000),
                receivable_balance: Some(420_000_000),
                rate_bps: Some(100),
                actual_bad_debt: Some(1_000_000),
                ..empty_asset_request()
            },
        ),
        (
            "B10",
            AssetBasedAdjustmentRequest {
                business_use_bps: Some(7_500),
                ..empty_asset_request()
            },
        ),
    ] {
        tax::calculate_asset_based_adjustment(pool, tenant_ref, by_id, module, request).await?;
    }
    tax::calculate_evaluation_adjustment(
        pool,
        tenant_ref,
        by_id,
        "B7",
        EvaluationAdjustmentRequest {
            positions: Some(vec![ValuationPositionInput {
                item_code: "USD_AR".to_string(),
                item_name: "USD accounts receivable".to_string(),
                position_type: Some("RECEIVABLE".to_string()),
                monetary: Some(true),
                valuation_method: Some("CLOSING_RATE".to_string()),
                book_amount: 105_000_000,
                tax_amount: Some(96_000_000),
                foreign_amount: Some(80_000.0),
                book_rate: Some(1312.5),
                closing_rate: Some(1200.0),
            }]),
            taxable_income_before_loss: None,
            loss_carryforwards: None,
            capital_changes: None,
        },
    )
    .await?;
    tax::calculate_evaluation_adjustment(
        pool,
        tenant_ref,
        by_id,
        "B8",
        EvaluationAdjustmentRequest {
            positions: Some(vec![ValuationPositionInput {
                item_code: "INV_OBS".to_string(),
                item_name: "Inventory valuation reserve".to_string(),
                position_type: Some("INVENTORY".to_string()),
                monetary: Some(false),
                valuation_method: Some("LOWER_OF_COST_OR_MARKET".to_string()),
                book_amount: 180_000_000,
                tax_amount: Some(162_000_000),
                foreign_amount: None,
                book_rate: None,
                closing_rate: None,
            }]),
            taxable_income_before_loss: None,
            loss_carryforwards: None,
            capital_changes: None,
        },
    )
    .await?;
    tax::calculate_transaction_based_adjustment(
        pool,
        tenant_ref,
        by_id,
        "B9",
        TransactionBasedAdjustmentRequest {
            accounting_income: Some(320_000_000),
            taxable_income_before_donation: None,
            gross_revenue: None,
            revenue_breakdowns: None,
            weighted_average_loan_balance: Some(900_000_000),
            weighted_average_interest_rate_bps: Some(460),
            manual_interest_disallowance: Some(5_000_000),
        },
    )
    .await?;
    tax::calculate_evaluation_adjustment(
        pool,
        tenant_ref,
        by_id,
        "B11",
        EvaluationAdjustmentRequest {
            positions: None,
            taxable_income_before_loss: Some(360_000_000),
            loss_carryforwards: Some(vec![LossCarryforwardInput {
                origin_year: 2023,
                original_amount: 120_000_000,
                remaining_amount: Some(60_000_000),
                expires_year: Some(2033),
            }]),
            capital_changes: None,
        },
    )
    .await?;
    tax::calculate_tax_amount_adjustment(
        pool,
        tenant_ref,
        by_id,
        "B12",
        TaxAmountAdjustmentRequest {
            tax_base: Some(360_000_000),
            calculated_tax: Some(48_400_000),
            regular_tax_after_credits: None,
            minimum_tax_rate_bps: None,
            credits: Some(vec![
                TaxCreditInput {
                    credit_type: "R_AND_D".to_string(),
                    base_amount: 80_000_000,
                    rate_bps: Some(250),
                    requested_amount: None,
                },
                TaxCreditInput {
                    credit_type: "ENERGY".to_string(),
                    base_amount: 40_000_000,
                    rate_bps: Some(100),
                    requested_amount: None,
                },
            ]),
            penalties: None,
        },
    )
    .await?;
    tax::calculate_tax_amount_adjustment(
        pool,
        tenant_ref,
        by_id,
        "B13",
        TaxAmountAdjustmentRequest {
            tax_base: Some(360_000_000),
            calculated_tax: Some(48_400_000),
            regular_tax_after_credits: Some(5_000_000),
            minimum_tax_rate_bps: Some(1_000),
            credits: None,
            penalties: None,
        },
    )
    .await?;
    tax::calculate_tax_amount_adjustment(
        pool,
        tenant_ref,
        by_id,
        "B14",
        TaxAmountAdjustmentRequest {
            tax_base: Some(22_000_000),
            calculated_tax: None,
            regular_tax_after_credits: None,
            minimum_tax_rate_bps: None,
            credits: None,
            penalties: Some(vec![PenaltyTaxInput {
                penalty_type: "LATE_PAYMENT".to_string(),
                tax_base: 22_000_000,
                rate_bps: 3,
                days_late: Some(45),
                reduction_bps: Some(0),
            }]),
        },
    )
    .await?;
    tax::calculate_evaluation_adjustment(
        pool,
        tenant_ref,
        by_id,
        "B15",
        EvaluationAdjustmentRequest {
            positions: None,
            taxable_income_before_loss: None,
            loss_carryforwards: None,
            capital_changes: Some(vec![
                CapitalChangeInput {
                    change_date: date("2026-03-31")?,
                    change_type: "PAID_IN_CAPITAL".to_string(),
                    amount: 150_000_000,
                    description: Some("New preferred share issuance".to_string()),
                },
                CapitalChangeInput {
                    change_date: date("2026-04-30")?,
                    change_type: "TREASURY_STOCK".to_string(),
                    amount: 35_000_000,
                    description: Some("Treasury stock acquisition".to_string()),
                },
            ]),
        },
    )
    .await?;
    tax::calculate_special_tax_adjustment(
        pool,
        tenant_ref,
        by_id,
        "B16",
        SpecialTaxAdjustmentRequest {
            foreign_incomes: Some(vec![ForeignIncomeInput {
                income_type: "ROYALTY".to_string(),
                gross_amount: 90_000_000,
                attributable_expense: Some(12_000_000),
                pe_allocation_bps: Some(8_000),
                withholding_tax: Some(7_000_000),
            }]),
            consolidated_entities: None,
            eliminations: None,
        },
    )
    .await?;
    tax::calculate_special_tax_adjustment(
        pool,
        tenant_ref,
        by_id,
        "B17",
        SpecialTaxAdjustmentRequest {
            foreign_incomes: None,
            consolidated_entities: Some(vec![
                ConsolidatedEntityInput {
                    entity_code: "ALPHA".to_string(),
                    entity_name: "Alpha Manufacturing Co.".to_string(),
                    ownership_bps: 10_000,
                    taxable_income: 360_000_000,
                    standalone_tax: Some(48_400_000),
                },
                ConsolidatedEntityInput {
                    entity_code: "ALPHA_RND".to_string(),
                    entity_name: "Alpha R&D Subsidiary".to_string(),
                    ownership_bps: 10_000,
                    taxable_income: 110_000_000,
                    standalone_tax: Some(18_900_000),
                },
            ]),
            eliminations: Some(vec![ConsolidationEliminationInput {
                elimination_type: "INTERCOMPANY_PROFIT".to_string(),
                amount: 25_000_000,
                direction: "DEDUCT".to_string(),
                description: Some("Inventory profit elimination".to_string()),
            }]),
        },
    )
    .await?;

    calculate_core(pool, tenant_ref, by_id, 320_000_000, 2_450_000_000).await?;
    for form_code in [
        "FORM15", "FORM22", "FORM3", "FORM32", "FORM50", "ATT01", "ATT02", "ATT03", "ATT04",
        "ATT05", "ATT06", "ATT07", "ATT08", "ATT09", "ATT10",
    ] {
        tax::generate_form(pool, tenant_ref, by_id, form_code).await?;
    }
    Ok(())
}

async fn seed_report_years(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    years: &HashMap<String, i64>,
) -> Result<()> {
    for (key, income, revenue) in [
        ("CUST01-2024", 245_000_000, 1_880_000_000),
        ("CUST01-2025", 280_000_000, 2_120_000_000),
        ("CUST03-2025", 155_000_000, 940_000_000),
    ] {
        calculate_core(pool, tenant_ref, years[key], income, revenue).await?;
    }
    Ok(())
}

async fn calculate_core(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    by_id: i64,
    accounting_income: i64,
    gross_revenue: i64,
) -> Result<()> {
    tax::calculate_adjustments(
        pool,
        tenant_ref,
        by_id,
        CalculateAdjustmentRequest {
            accounting_income,
            gross_revenue: Some(gross_revenue),
            donations: Some(58_000_000),
            entertainment_expense: Some(75_000_000),
            depreciation_book: Some(155_000_000),
            depreciation_tax_limit: Some(118_000_000),
            carryforward_loss: Some(24_000_000),
            tax_credits: Some(8_000_000),
        },
    )
    .await?;
    Ok(())
}

async fn seed_workflow(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    main_by_id: i64,
    filed_by_id: i64,
) -> Result<()> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    for (by_id, from_status, to_status, action, actor, comment) in [
        (
            main_by_id,
            "DRAFT",
            "IN_REVIEW",
            "SUBMIT_REVIEW",
            "writer01",
            "Demo package submitted for review",
        ),
        (
            filed_by_id,
            "DRAFT",
            "IN_REVIEW",
            "SUBMIT_REVIEW",
            "writer01",
            "Prior year review request",
        ),
        (
            filed_by_id,
            "IN_REVIEW",
            "APPROVED",
            "APPROVE",
            "reviewer01",
            "Prior year approved",
        ),
        (
            filed_by_id,
            "APPROVED",
            "FILED",
            "FILE",
            "tax01",
            "Prior year filed",
        ),
    ] {
        sqlx::query(&format!(
            r#"
            INSERT INTO {schema}.workflow_events (
                by_id, from_status, to_status, action, actor, comment, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#
        ))
        .bind(by_id)
        .bind(from_status)
        .bind(to_status)
        .bind(action)
        .bind(actor)
        .bind(comment)
        .bind(json!({ "seed": true }))
        .execute(pool)
        .await
        .context("failed to seed workflow event")?;
    }
    sqlx::query(&format!(
        r#"
        INSERT INTO {schema}.approval_lines (by_id, step_order, approver_login_id, status, comment)
        VALUES ($1, 1, 'reviewer01', 'PENDING', 'Demo review queue')
        "#
    ))
    .bind(main_by_id)
    .execute(pool)
    .await
    .context("failed to seed pending approval line")?;
    sqlx::query(&format!(
        r#"
        INSERT INTO {schema}.approval_lines (
            by_id, step_order, approver_login_id, status, acted_at, comment
        )
        VALUES ($1, 1, 'reviewer01', 'APPROVED', NOW(), 'Prior year approval')
        "#
    ))
    .bind(filed_by_id)
    .execute(pool)
    .await
    .context("failed to seed filed approval line")?;
    Ok(())
}

async fn seed_notifications(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    main_by_id: i64,
    filed_by_id: i64,
) -> Result<()> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    for (by_id, title, message, severity) in [
        (
            Some(main_by_id),
            "Review package submitted",
            "Reviewer action is required.",
            "INFO",
        ),
        (
            Some(main_by_id),
            "Filing due soon",
            "The demo 2026 filing closes within 30 days.",
            "WARN",
        ),
        (
            Some(filed_by_id),
            "Filed return locked",
            "Prior year return is locked for amendment preview.",
            "INFO",
        ),
        (
            None,
            "Menu smoke dataset ready",
            "All prototype menus have demo data.",
            "WARN",
        ),
    ] {
        sqlx::query(&format!(
            r#"
            INSERT INTO {schema}.notifications (by_id, title, message, severity, status, metadata)
            VALUES ($1, $2, $3, $4, 'UNREAD', $5)
            "#
        ))
        .bind(by_id)
        .bind(title)
        .bind(message)
        .bind(severity)
        .bind(json!({ "seed": true }))
        .execute(pool)
        .await
        .context("failed to seed notification")?;
    }
    Ok(())
}

async fn seed_audit_logs(pool: &PgPool, tenant_ref: &TenantRef, by_id: i64) -> Result<()> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    for index in 1..=5 {
        sqlx::query(&format!(
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
            SELECT 'seed_demo', $1, 'CREATE', NULL, $2, 'seed-demo',
                   CURRENT_DATE, prev.hash_current,
                   md5(COALESCE(prev.hash_current, '') || $1 || $2::text)
            FROM (SELECT 1) seed
            LEFT JOIN prev ON TRUE
            "#
        ))
        .bind(format!("{by_id}-{index}"))
        .bind(json!({ "sequence": index, "by_id": by_id }))
        .execute(pool)
        .await
        .context("failed to seed audit log")?;
    }
    Ok(())
}

fn income_item(
    section: &str,
    item_code: &str,
    item_name: &str,
    amount: i64,
    temporary: bool,
) -> IncomeAdjustmentItemInput {
    IncomeAdjustmentItemInput {
        section: section.to_string(),
        item_code: item_code.to_string(),
        item_name: item_name.to_string(),
        amount,
        disposition: None,
        temporary: Some(temporary),
        law_ref: Some("Demo seed".to_string()),
        metadata: Some(json!({ "seed": true })),
    }
}

fn empty_asset_request() -> AssetBasedAdjustmentRequest {
    AssetBasedAdjustmentRequest {
        book_reserve: None,
        estimated_liability: None,
        external_fund: None,
        receivable_balance: None,
        rate_bps: None,
        actual_bad_debt: None,
        business_use_bps: None,
    }
}

fn date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .with_context(|| format!("invalid seed date {value}"))
}

fn financial_statement_csv() -> String {
    [
        "statement_type,account_code,account_name,debit,credit,standard_account_code,standard_account_name",
        "BS,10100,Cash,350000000,0,STD_CASH,Cash",
        "BS,10200,Accounts receivable,420000000,0,STD_AR,Accounts receivable",
        "BS,10300,Inventory,180000000,0,STD_INVENTORY,Inventory",
        "BS,10400,Prepaid expense,125000000,0,STD_PREPAID,Prepaid expense",
        "BS,11100,Land,300000000,0,STD_LAND,Land",
        "BS,11200,Buildings,900000000,0,STD_BUILDING,Buildings",
        "BS,11300,Vehicles,120000000,0,STD_VEHICLE,Vehicles",
        "BS,11400,Machinery,650000000,0,STD_MACHINERY,Machinery",
        "BS,11500,Software,90000000,0,STD_SOFTWARE,Software",
        "IS,50100,Cost of goods sold,1100000000,0,STD_COGS,Cost of goods sold",
        "IS,51100,Salaries,360000000,0,STD_SALARY,Salaries",
        "IS,52100,Rent,95000000,0,STD_RENT,Rent",
        "IS,53100,Donations,28000000,0,STD_DONATION,Donations",
        "IS,53200,Entertainment,35000000,0,STD_ENTERTAINMENT,Entertainment",
        "IS,53300,Interest expense,42000000,0,STD_INTEREST_EXPENSE,Interest expense",
        "IS,54100,Depreciation,160000000,0,STD_DEPRECIATION,Depreciation",
        "IS,55100,R&D expense,80000000,0,STD_RND,R&D expense",
        "IS,55200,Foreign service expense,26000000,0,STD_FOREIGN_EXPENSE,Foreign service expense",
        "IS,55300,Tax expense,70000000,0,STD_TAX_EXPENSE,Tax expense",
        "BS,20100,Accounts payable,0,270000000,STD_AP,Accounts payable",
        "BS,20200,Bank loans,0,480000000,STD_LOAN,Bank loans",
        "BS,20300,Tax payable,0,30000000,STD_TAX_PAYABLE,Tax payable",
        "BS,20400,Accrued expense,0,95000000,STD_ACCRUAL,Accrued expense",
        "BS,30100,Capital stock,0,1000000000,STD_CAPITAL,Capital stock",
        "BS,30200,Retained earnings,0,650000000,STD_RETAINED_EARNINGS,Retained earnings",
        "IS,40100,Product revenue,0,1800000000,STD_PRODUCT_REVENUE,Product revenue",
        "IS,40200,Service revenue,0,620000000,STD_SERVICE_REVENUE,Service revenue",
        "IS,40300,Interest income,0,25000000,STD_INTEREST_INCOME,Interest income",
        "IS,40400,FX gain,0,18000000,STD_FX_GAIN,FX gain",
        "IS,NET_INCOME,Net income,0,143000000,ACCOUNTING_INCOME,Accounting income",
    ]
    .join("\n")
}

fn asset_csv() -> String {
    [
        "asset_code,asset_name,asset_category,acquisition_date,acquisition_cost,useful_life_years",
        "CAR001,Executive vehicle,VEHICLE,2026-01-10,85000000,3",
        "CAR002,Sales vehicle,VEHICLE,2026-02-14,72000000,4",
        "CAR003,Service van,VEHICLE,2026-03-05,68000000,4",
        "MACH001,CNC machine,MACHINERY,2026-01-20,480000000,4",
        "MACH002,Packaging line,MACHINERY,2026-02-15,240000000,4",
        "SW001,ERP software,SOFTWARE,2026-01-01,120000000,3",
        "BLD001,Factory building,BUILDING,2026-01-01,900000000,20",
        "FIX001,Office fixtures,GENERAL,2026-04-01,50000000,4",
    ]
    .join("\n")
}

fn transaction_csv() -> String {
    [
        "tx_date,partner_name,category,account_code,description,amount,evidence_type",
        "2026-02-03,National Relief Fund,DONATION,53100,special statutory donation,42000000,RECEIPT",
        "2026-03-18,Local Community Fund,DONATION,53100,general donation,16000000,RECEIPT",
        "2026-01-22,Client Dinner A,ENTERTAINMENT,53200,client dinner with card,25000000,CARD",
        "2026-02-10,Client Event B,ENTERTAINMENT,53200,cash entertainment without qualified evidence,20000000,CASH",
        "2026-04-11,Partner Workshop,ENTERTAINMENT,53200,workshop meal with receipt,30000000,RECEIPT",
        "2026-03-01,Unknown Creditor,INTEREST,53300,unidentified creditor short term interest,12000000,TRANSFER",
        "2026-03-29,Unknown Recipient,INTEREST,53300,unidentified recipient interest,9000000,TRANSFER",
        "2026-04-15,Construction Bank,INTEREST,53300,construction financing interest,21000000,TRANSFER",
        "2026-05-02,Office Supplier,OTHER,54000,office supplies,7000000,CARD",
        "2026-05-17,Foreign Vendor,OTHER,55200,foreign service fee,19000000,INVOICE",
    ]
    .join("\n")
}
