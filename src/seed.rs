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
    efiling, menu, std_fs, tax, tax_data, tenant, validation_rules,
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
        VALUES ($1, '데모 법인세 신고 작업장', '1108112345', DATE '2026-01-01',
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
        VALUES ('samplefirm', '샘플 세무법인', '2208112345', DATE '2026-01-01',
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
            ('TAX_WRITER', '작성 담당자', '데모 자료 입력 및 서식 작성 담당자', TRUE),
            ('TAX_REVIEWER', '검토 담당자', '데모 검토 및 승인 담당자', TRUE),
            ('TAX_EXPERT', '세무조정 전문가', '데모 세무조정 전문가', TRUE),
            ('TENANT_ADMIN', '테넌트 관리자', '데모 테넌트 관리자', TRUE),
            ('SUPER_ADMIN', '슈퍼 관리자', '데모 전체 관리자', TRUE)
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
            "데모 관리자",
            "admin.demo@example.test",
            admin_password,
            vec!["SUPER_ADMIN", "TENANT_ADMIN"],
        ),
        (
            "writer01",
            "데모 작성자",
            "writer01.demo@example.test",
            DEMO_PASSWORD,
            vec!["TAX_WRITER", "TAX_EXPERT"],
        ),
        (
            "reviewer01",
            "데모 검토자",
            "reviewer01.demo@example.test",
            DEMO_PASSWORD,
            vec!["TAX_REVIEWER"],
        ),
        (
            "tax01",
            "데모 세무전문가",
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
            "알파 제조 주식회사",
            "1208111111",
            "1101111111111",
            "C25999",
            true,
        ),
        (
            "CUST02",
            "베타 플랫폼 서비스",
            "2208122222",
            "2202222222222",
            "J58222",
            false,
        ),
        (
            "CUST03",
            "감마 바이오 연구소",
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
            customer_id, year_label, start_date, end_date, status, locked_at, amendment_sequence
        )
        VALUES ($1, $2, $3, $4, $5, CASE WHEN $6 THEN NOW() ELSE NULL END, 0)
        ON CONFLICT (customer_id, year_label, amendment_sequence) DO UPDATE
        SET start_date = EXCLUDED.start_date,
            end_date = EXCLUDED.end_date,
            status = EXCLUDED.status,
            locked_at = EXCLUDED.locked_at,
            original_by_id = NULL,
            amendment_reason = NULL,
            version_mode = NULL,
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
        format!("DELETE FROM {schema}.std_fs_statements WHERE business_year_id = ANY($1)"),
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
    seed_std_fs_fixture(pool, tenant_ref, by_id).await?;

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

async fn seed_std_fs_fixture(pool: &PgPool, tenant_ref: &TenantRef, by_id: i64) -> Result<()> {
    let snapshot = tax::ensure_law_snapshot(pool, tenant_ref, by_id).await?;
    let Some(version_id) = snapshot.std_fs_version_id else {
        return Ok(());
    };
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let customer_id = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT customer_id FROM {schema}.business_years WHERE by_id = $1"
    ))
    .bind(by_id)
    .fetch_one(pool)
    .await
    .context("failed to resolve std fs fixture customer")?;

    let update_lines = format!(
        r#"
        UPDATE {schema}.fs_lines l
        SET std_fs_item_code = mapping.std_fs_item_code
        FROM {schema}.financial_statements f,
             (VALUES
                ('10100', '1010'),
                ('10200', '1030'),
                ('10300', '1200'),
                ('10400', '1050'),
                ('11100', '1521'),
                ('11200', '1522'),
                ('11300', '1523'),
                ('11400', '1524'),
                ('11500', '1530'),
                ('20100', '2010'),
                ('20200', '2020'),
                ('20300', '2030'),
                ('20400', '2040'),
                ('30100', '3010'),
                ('30200', '3020'),
                ('40100', '4010'),
                ('40200', '4020'),
                ('40300', '4030'),
                ('40400', '4040'),
                ('50100', '4510'),
                ('51100', '5110'),
                ('52100', '5120'),
                ('53100', '5130'),
                ('53200', '5140'),
                ('53300', '5150'),
                ('54100', '5170'),
                ('55100', '5180'),
                ('55200', '5190'),
                ('55300', '8500'),
                ('NET_INCOME', '9000')
             ) AS mapping(account_code, std_fs_item_code)
        WHERE f.fs_id = l.fs_id
          AND f.by_id = $1
          AND l.account_code = mapping.account_code
        "#
    );
    sqlx::query(&update_lines)
        .bind(by_id)
        .execute(pool)
        .await
        .context("failed to seed std fs line mappings")?;

    let insert_mappings = format!(
        r#"
        INSERT INTO {schema}.std_fs_mappings (
            tenant_id, customer_id, version_id, account_code, account_name,
            std_fs_item_code, is_auto_mapped, last_used_at
        )
        SELECT $1, $2, $3, l.account_code, MAX(l.account_name), l.std_fs_item_code,
               TRUE, NOW()
        FROM {schema}.financial_statements f
        JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
        WHERE f.by_id = $4
          AND l.std_fs_item_code IS NOT NULL
        GROUP BY l.account_code, l.std_fs_item_code
        ON CONFLICT (customer_id, version_id, account_code) DO UPDATE
        SET account_name = EXCLUDED.account_name,
            std_fs_item_code = EXCLUDED.std_fs_item_code,
            is_auto_mapped = TRUE,
            usage_count = {schema}.std_fs_mappings.usage_count + 1,
            last_used_at = NOW(),
            updated_at = NOW()
        "#
    );
    sqlx::query(&insert_mappings)
        .bind(tenant_ref.tenant_id)
        .bind(customer_id)
        .bind(version_id)
        .bind(by_id)
        .execute(pool)
        .await
        .context("failed to seed std fs mappings")?;

    std_fs::confirm_workspace_statements(pool, tenant_ref, by_id)
        .await
        .context("failed to seed confirmed std fs statements")?;
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
                    "임원 상여 미지급액",
                    18_000_000,
                    true,
                ),
                income_item(
                    "LOSS_DISALLOWANCE",
                    "B1_NONBUSINESS_EXPENSE",
                    "업무무관 비용",
                    7_500_000,
                    false,
                ),
                income_item(
                    "GROSS_EXCLUSION",
                    "B1_TAX_EXEMPT_INCOME",
                    "비과세 이자수익",
                    3_000_000,
                    false,
                ),
                income_item(
                    "LOSS_INCLUSION",
                    "B1_PRIOR_RESERVE_REVERSAL",
                    "전기 유보 환입",
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
                item_name: "USD 매출채권".to_string(),
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
                item_name: "재고평가충당금".to_string(),
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
                    description: Some("신종 우선주 발행".to_string()),
                },
                CapitalChangeInput {
                    change_date: date("2026-04-30")?,
                    change_type: "TREASURY_STOCK".to_string(),
                    amount: 35_000_000,
                    description: Some("자기주식 취득".to_string()),
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
                    entity_name: "알파 제조 주식회사".to_string(),
                    ownership_bps: 10_000,
                    taxable_income: 360_000_000,
                    standalone_tax: Some(48_400_000),
                },
                ConsolidatedEntityInput {
                    entity_code: "ALPHA_RND".to_string(),
                    entity_name: "알파 연구개발 자회사".to_string(),
                    ownership_bps: 10_000,
                    taxable_income: 110_000_000,
                    standalone_tax: Some(18_900_000),
                },
            ]),
            eliminations: Some(vec![ConsolidationEliminationInput {
                elimination_type: "INTERCOMPANY_PROFIT".to_string(),
                amount: 25_000_000,
                direction: "DEDUCT".to_string(),
                description: Some("재고 내부이익 제거".to_string()),
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
            "데모 신고 패키지 검토 요청",
        ),
        (
            filed_by_id,
            "DRAFT",
            "IN_REVIEW",
            "SUBMIT_REVIEW",
            "writer01",
            "전년도 신고 검토 요청",
        ),
        (
            filed_by_id,
            "IN_REVIEW",
            "APPROVED",
            "APPROVE",
            "reviewer01",
            "전년도 신고 승인",
        ),
        (
            filed_by_id,
            "APPROVED",
            "FILED",
            "FILE",
            "tax01",
            "전년도 신고 제출 완료",
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
        VALUES ($1, 1, 'reviewer01', 'PENDING', '데모 검토 대기')
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
        VALUES ($1, 1, 'reviewer01', 'APPROVED', NOW(), '전년도 신고 승인')
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
            "검토 패키지 제출",
            "검토 담당자 처리가 필요합니다.",
            "INFO",
        ),
        (
            Some(main_by_id),
            "신고 마감 임박",
            "데모 2026 신고 마감이 30일 이내입니다.",
            "WARN",
        ),
        (
            Some(filed_by_id),
            "신고 완료본 잠금",
            "전년도 신고서는 수정신고 미리보기를 위해 잠겨 있습니다.",
            "INFO",
        ),
        (
            None,
            "메뉴 점검 데이터 준비",
            "전체 프로토타입 메뉴에 데모 데이터가 준비되었습니다.",
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
                   md5(COALESCE(prev.hash_current, '') || 'seed_demo' || $1 || 'CREATE' ||
                       COALESCE($2::jsonb::text, '') || 'seed-demo')
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
        law_ref: Some("데모 시드".to_string()),
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
        "BS,10100,현금,350000000,0,STD_CASH,현금",
        "BS,10200,매출채권,420000000,0,STD_AR,매출채권",
        "BS,10300,재고자산,180000000,0,STD_INVENTORY,재고자산",
        "BS,10400,선급비용,125000000,0,STD_PREPAID,선급비용",
        "BS,11100,토지,300000000,0,STD_LAND,토지",
        "BS,11200,건물,900000000,0,STD_BUILDING,건물",
        "BS,11300,차량운반구,225000000,0,STD_VEHICLE,차량운반구",
        "BS,11400,기계장치,720000000,0,STD_MACHINERY,기계장치",
        "BS,11500,소프트웨어,120000000,0,STD_INTANGIBLE,소프트웨어",
        "IS,50100,매출원가,1100000000,0,STD_COGS,매출원가",
        "IS,51100,급여,360000000,0,STD_SALARY,급여",
        "IS,52100,임차료,95000000,0,STD_RENT,임차료",
        "IS,53100,기부금,58000000,0,STD_DONATION,기부금",
        "IS,53200,접대비,75000000,0,STD_ENTERTAINMENT,접대비",
        "IS,53300,이자비용,42000000,0,STD_INTEREST_EXPENSE,이자비용",
        "IS,54100,감가상각비,160000000,0,STD_DEPRECIATION,감가상각비",
        "IS,55100,연구개발비,80000000,0,STD_RND,연구개발비",
        "IS,55200,해외용역비,26000000,0,STD_FOREIGN_EXPENSE,해외용역비",
        "IS,55300,법인세비용,70000000,0,STD_TAX_EXPENSE,법인세비용",
        "BS,20100,매입채무,0,270000000,STD_AP,매입채무",
        "BS,20200,은행차입금,0,480000000,STD_LOAN,은행차입금",
        "BS,20300,미지급세금,0,30000000,STD_TAX_PAYABLE,미지급세금",
        "BS,20400,미지급비용,0,95000000,STD_ACCRUAL,미지급비용",
        "BS,30100,자본금,0,1000000000,STD_CAPITAL,자본금",
        "BS,30200,이익잉여금,0,1465000000,STD_RETAINED_EARNINGS,이익잉여금",
        "IS,40100,제품매출,0,1190000000,STD_PRODUCT_REVENUE,제품매출",
        "IS,40200,용역매출,0,620000000,STD_SERVICE_REVENUE,용역매출",
        "IS,40300,이자수익,0,25000000,STD_INTEREST_INCOME,이자수익",
        "IS,40400,외환차익,0,18000000,STD_FX_GAIN,외환차익",
        "IS,NET_INCOME,당기순이익,0,213000000,ACCOUNTING_INCOME,회계상 소득",
    ]
    .join("\n")
}

fn asset_csv() -> String {
    [
        "asset_code,asset_name,asset_category,acquisition_date,acquisition_cost,useful_life_years",
        "CAR001,임원 업무용 차량,VEHICLE,2026-01-10,85000000,3",
        "CAR002,영업 업무용 차량,VEHICLE,2026-02-14,72000000,4",
        "CAR003,서비스 밴,VEHICLE,2026-03-05,68000000,4",
        "MACH001,CNC 기계,MACHINERY,2026-01-20,480000000,4",
        "MACH002,포장 라인,MACHINERY,2026-02-15,240000000,4",
        "SW001,ERP 소프트웨어,SOFTWARE,2026-01-01,120000000,3",
        "BLD001,공장 건물,BUILDING,2026-01-01,900000000,20",
        "LAND001,공장 토지,LAND,2026-01-01,300000000,99",
    ]
    .join("\n")
}

fn transaction_csv() -> String {
    [
        "tx_date,partner_name,category,account_code,description,amount,evidence_type",
        "2026-02-03,국민구호기금,DONATION,53100,특례 법정기부금,42000000,RECEIPT",
        "2026-03-18,지역공동체기금,DONATION,53100,일반기부금,16000000,RECEIPT",
        "2026-01-22,거래처 만찬 A,ENTERTAINMENT,53200,카드 사용 접대 식사,25000000,CARD",
        "2026-02-10,거래처 행사 B,ENTERTAINMENT,53200,적격증빙 없는 현금 접대비,20000000,CASH",
        "2026-04-11,협력사 워크숍,ENTERTAINMENT,53200,영수증 있는 워크숍 식사,30000000,RECEIPT",
        "2026-03-01,불명 채권자,INTEREST,53300,채권자 불분명 단기이자,12000000,TRANSFER",
        "2026-03-29,불명 수령자,INTEREST,53300,수령자 불분명 이자,9000000,TRANSFER",
        "2026-04-15,건설은행,INTEREST,53300,건설자금 이자,21000000,TRANSFER",
        "2026-05-02,사무용품 공급사,OTHER,54000,사무용품 구입,7000000,CARD",
        "2026-05-17,해외 공급사,OTHER,55200,해외 용역 수수료,19000000,INVOICE",
    ]
    .join("\n")
}
