use std::env;

use chrono::NaiveDate;
use cit_system::{
    db,
    domain::{
        CreateBusinessYearRequest, CreateCustomerRequest, CreateTenantRequest,
        FinancialStatementLine, StdFsItem, StdFsItemVersion, StdFsMapping, StdFsStatement,
    },
    tax, tax_data, tenant, validation_rules,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
async fn std_fs_schema_models_fixture_and_snapshot_are_wired() {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let pool = db::connect(&database_url).await.expect("db connection");
    db::migrate(&pool).await.expect("migrations");

    let version = seeded_std_fs_version(&pool).await;
    let mandatory_standard_accounts = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM standard_accounts
        WHERE code = ANY($1)
          AND is_active = TRUE
        "#,
    )
    .bind(
        [
            "NET_INCOME",
            "REVENUE",
            "PPE_NET",
            "ENTERTAIN_EXP",
            "DONATION_EXP",
            "INTEREST_EXP",
            "PENSION_PROV",
            "BAD_DEBT_PROV",
            "ACCUM_DEPR",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>(),
    )
    .fetch_one(&pool)
    .await
    .expect("mandatory standard accounts");
    assert_eq!(mandatory_standard_accounts, 9);
    let seeded_items = sqlx::query_as::<_, StdFsItem>(
        r#"
        SELECT id, version_id, stmt_type, item_code, item_name, parent_code, level,
               account_class, normal_balance, is_subtotal, is_required, agg_formula,
               xml_field_id, sort_order, is_active
        FROM std_fs_items
        WHERE version_id = $1
        ORDER BY stmt_type, sort_order
        "#,
    )
    .bind(version.id)
    .fetch_all(&pool)
    .await
    .expect("seeded std fs items");
    assert!(
        seeded_items.iter().any(|item| item.item_code == "1010")
            && seeded_items.iter().any(|item| item.item_code == "9000"),
        "fixture must include baseline BS/IS leaf items"
    );

    let tenant_code = format!("stdfs{}", &Uuid::new_v4().simple().to_string()[..8]);
    let created = tenant::create_tenant(
        &pool,
        CreateTenantRequest {
            tenant_code: tenant_code.clone(),
            tenant_name: "표준재무제표 스키마 테스트".to_string(),
            biz_reg_no: "1108112345".to_string(),
            contract_start: date("2026-01-01"),
            contract_end: None,
            allowed_ips: None,
            max_users: Some(3),
            plan: Some("STANDARD".to_string()),
        },
    )
    .await
    .expect("create tenant");
    let tenant_ref = tenant::resolve_tenant(&pool, &tenant_code)
        .await
        .expect("resolve tenant");
    assert_eq!(created.schema_name, tenant_ref.schema_name);
    assert_std_fs_schema_contract(&pool, &tenant_ref.schema_name).await;

    let customer = tenant::create_customer(
        &pool,
        &tenant_ref,
        CreateCustomerRequest {
            customer_code: "C001".to_string(),
            customer_name: "표준재무 테스트 법인".to_string(),
            biz_reg_no: "1208111111".to_string(),
            corp_reg_no: Some("1101111111111".to_string()),
            corp_type: Some("DOMESTIC".to_string()),
            industry_code: Some("C25999".to_string()),
            is_sme: Some(true),
            work_scopes: None,
        },
    )
    .await
    .expect("create customer");
    let business_year = tenant::create_business_year(
        &pool,
        &tenant_ref,
        CreateBusinessYearRequest {
            customer_id: customer.customer_id,
            year_label: 2026,
            start_date: date("2026-01-01"),
            end_date: date("2026-12-31"),
            carry_forward_from_by_id: None,
        },
    )
    .await
    .expect("create business year");
    let snapshot = tax::ensure_law_snapshot(&pool, &tenant_ref, business_year.by_id)
        .await
        .expect("law snapshot");
    assert_eq!(snapshot.std_fs_version_id, Some(version.id));
    assert_eq!(
        snapshot.snapshot_data["std_fs"]["version_id"],
        json!(version.id)
    );

    let schema = db::quote_ident(&tenant_ref.schema_name).expect("schema quote");
    let fs_id = sqlx::query_scalar::<_, i64>(&format!(
        "INSERT INTO {schema}.financial_statements (by_id, statement_type) VALUES ($1, 'BS') RETURNING fs_id"
    ))
    .bind(business_year.by_id)
    .fetch_one(&pool)
    .await
    .expect("insert financial statement");
    sqlx::query(&format!(
        r#"
        INSERT INTO {schema}.fs_lines (
            fs_id, account_code, account_name, std_account_code, std_account_name,
            standard_account_code, standard_account_name, std_fs_item_code, amount, debit_credit
        )
        VALUES ($1, '10100', '현금', 'STD_CASH', '현금', 'STD_CASH', '현금', '1010', 1000000, 'DEBIT')
        "#
    ))
    .bind(fs_id)
    .execute(&pool)
    .await
    .expect("insert fs line");
    let lines = tax_data::list_financial_statement_lines(&pool, &tenant_ref, business_year.by_id)
        .await
        .expect("list financial statement lines");
    let line: &FinancialStatementLine = lines.first().expect("financial line");
    assert_eq!(line.std_account_code.as_deref(), Some("STD_CASH"));
    assert_eq!(line.std_fs_item_code.as_deref(), Some("1010"));

    let mapping = sqlx::query_as::<_, StdFsMapping>(&format!(
        r#"
        INSERT INTO {schema}.std_fs_mappings (
            tenant_id, customer_id, version_id, account_code, account_name,
            std_fs_item_code, is_auto_mapped, last_used_at
        )
        VALUES ($1, $2, $3, '10100', '현금', '1010', TRUE, NOW())
        RETURNING id, tenant_id, customer_id, version_id, account_code, account_name,
                  std_fs_item_code, is_auto_mapped, usage_count, last_used_at,
                  created_by, created_at, updated_at
        "#
    ))
    .bind(tenant_ref.tenant_id)
    .bind(customer.customer_id)
    .bind(version.id)
    .fetch_one(&pool)
    .await
    .expect("insert std fs mapping");
    assert_eq!(mapping.std_fs_item_code, "1010");

    let statement = sqlx::query_as::<_, StdFsStatement>(&format!(
        r#"
        INSERT INTO {schema}.std_fs_statements (
            tenant_id, business_year_id, version_id, stmt_type, status,
            item_code, amount, source_line_ids, total_check, confirmed_at
        )
        VALUES ($1, $2, $3, 'STD_BS', 'CONFIRMED', '1010', 1000000,
                jsonb_build_array($4::BIGINT), '{{"bs_balanced":true}}'::jsonb, NOW())
        RETURNING id, tenant_id, business_year_id, version_id, stmt_type, status,
                  item_code, amount, source_line_ids, total_check, confirmed_at, created_at
        "#
    ))
    .bind(tenant_ref.tenant_id)
    .bind(business_year.by_id)
    .bind(version.id)
    .bind(line.line_id)
    .fetch_one(&pool)
    .await
    .expect("insert std fs statement");
    assert_eq!(statement.status, "CONFIRMED");
    assert_eq!(statement.source_line_ids, json!([line.line_id]));
}

#[tokio::test]
async fn tax_standard_account_mapping_enforces_mandatory_tax_accounts() {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let pool = db::connect(&database_url).await.expect("db connection");
    db::migrate(&pool).await.expect("migrations");

    let tenant_code = format!("taxmap{}", &Uuid::new_v4().simple().to_string()[..8]);
    tenant::create_tenant(
        &pool,
        CreateTenantRequest {
            tenant_code: tenant_code.clone(),
            tenant_name: "세무 표준계정 매핑 테스트".to_string(),
            biz_reg_no: "1108112345".to_string(),
            contract_start: date("2026-01-01"),
            contract_end: None,
            allowed_ips: None,
            max_users: Some(3),
            plan: Some("STANDARD".to_string()),
        },
    )
    .await
    .expect("create tenant");
    let tenant_ref = tenant::resolve_tenant(&pool, &tenant_code)
        .await
        .expect("resolve tenant");
    let customer = tenant::create_customer(
        &pool,
        &tenant_ref,
        CreateCustomerRequest {
            customer_code: "TM001".to_string(),
            customer_name: "세무 매핑 테스트 법인".to_string(),
            biz_reg_no: "1208111111".to_string(),
            corp_reg_no: None,
            corp_type: Some("DOMESTIC".to_string()),
            industry_code: Some("C25999".to_string()),
            is_sme: Some(true),
            work_scopes: None,
        },
    )
    .await
    .expect("create customer");
    let business_year = tenant::create_business_year(
        &pool,
        &tenant_ref,
        CreateBusinessYearRequest {
            customer_id: customer.customer_id,
            year_label: 2026,
            start_date: date("2026-01-01"),
            end_date: date("2026-12-31"),
            carry_forward_from_by_id: None,
        },
    )
    .await
    .expect("create business year");

    let csv = "\
statement_type,account_code,account_name,debit,credit,std_account_code
IS,40100,Product revenue,0,1000,REVENUE
IS,53100,Donation expense,1000,0,STD_EXPENSE
";
    let imported = tax_data::import_tax_data(
        &pool,
        &tenant_ref,
        business_year.by_id,
        "financial-statements",
        Some("mandatory-tax-map.csv".to_string()),
        csv.as_bytes(),
    )
    .await
    .expect("import financial statements");
    assert_eq!(imported.batch.status, "IMPORTED");

    let summary = tax_data::validation_summary(&pool, &tenant_ref, business_year.by_id)
        .await
        .expect("tax data validation summary");
    assert_eq!(summary.mandatory_mapping_missing_count, 1);
    assert_eq!(
        summary.mandatory_mapping_missing_codes,
        vec!["DONATION_EXP".to_string()]
    );

    let validation = validation_rules::run_validation(&pool, &tenant_ref, business_year.by_id)
        .await
        .expect("run validation");
    assert!(validation.issues.iter().any(|issue| {
        issue.rule_code == "TD_TAX_REQUIRED_MAPPINGS" && issue.severity == "ERROR"
    }));
}

#[tokio::test]
async fn std_fs_snapshot_selects_active_industry_corp_version_with_general_domestic_fallback() {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let pool = db::connect(&database_url).await.expect("db connection");
    db::migrate(&pool).await.expect("migrations");

    let random = Uuid::new_v4().simple().to_string();
    let suffix = &random[..8];
    let base_year = 2030 + (Uuid::new_v4().as_u128() % 5000) as i32;
    let exact_finance = insert_std_fs_version(
        &pool,
        &format!("TEST-STDFS-FIN-{suffix}"),
        "FINANCE",
        "DOMESTIC",
        date(&format!("{base_year}-01-01")),
        "ACTIVE",
    )
    .await;
    let newer_general = insert_std_fs_version(
        &pool,
        &format!("TEST-STDFS-GEN-{suffix}"),
        "GENERAL",
        "DOMESTIC",
        date(&format!("{base_year}-06-01")),
        "ACTIVE",
    )
    .await;
    let inactive_finance = insert_std_fs_version(
        &pool,
        &format!("TEST-STDFS-FIN-RET-{suffix}"),
        "FINANCE",
        "DOMESTIC",
        date(&format!("{base_year}-07-01")),
        "RETIRED",
    )
    .await;

    let foreign_year = base_year + 1;
    let foreign_exact = insert_std_fs_version(
        &pool,
        &format!("TEST-STDFS-GEN-FOR-{suffix}"),
        "GENERAL",
        "FOREIGN",
        date(&format!("{foreign_year}-01-01")),
        "ACTIVE",
    )
    .await;
    let foreign_domestic_fallback = insert_std_fs_version(
        &pool,
        &format!("TEST-STDFS-GEN-DOM-{suffix}"),
        "GENERAL",
        "DOMESTIC",
        date(&format!("{foreign_year}-01-01")),
        "ACTIVE",
    )
    .await;

    let final_fallback_year = base_year + 2;
    let final_fallback = insert_std_fs_version(
        &pool,
        &format!("TEST-STDFS-GEN-FIN-{suffix}"),
        "GENERAL",
        "DOMESTIC",
        date(&format!("{final_fallback_year}-01-01")),
        "ACTIVE",
    )
    .await;

    let tenant_code = format!("stdfs{suffix}");
    tenant::create_tenant(
        &pool,
        CreateTenantRequest {
            tenant_code: tenant_code.clone(),
            tenant_name: "표준재무제표 버전 선택 테스트".to_string(),
            biz_reg_no: "1108112345".to_string(),
            contract_start: date("2026-01-01"),
            contract_end: None,
            allowed_ips: None,
            max_users: Some(3),
            plan: Some("STANDARD".to_string()),
        },
    )
    .await
    .expect("create tenant");
    let tenant_ref = tenant::resolve_tenant(&pool, &tenant_code)
        .await
        .expect("resolve tenant");

    let finance_snapshot = create_customer_year_snapshot(
        &pool,
        &tenant_ref,
        "FIN",
        Some("K64110"),
        Some("DOMESTIC"),
        base_year,
        &format!("{base_year}-08-01"),
        &format!("{base_year}-12-31"),
    )
    .await;
    assert_eq!(finance_snapshot.std_fs_version_id, Some(exact_finance));
    assert_ne!(finance_snapshot.std_fs_version_id, Some(newer_general));
    assert_ne!(finance_snapshot.std_fs_version_id, Some(inactive_finance));
    assert_eq!(
        finance_snapshot.snapshot_data["std_fs"]["industry_type"],
        "FINANCE"
    );
    assert_eq!(
        finance_snapshot.snapshot_data["std_fs"]["corp_type"],
        "DOMESTIC"
    );

    let foreign_snapshot = create_customer_year_snapshot(
        &pool,
        &tenant_ref,
        "FOR",
        Some("62010"),
        Some("FOREIGN"),
        foreign_year,
        &format!("{foreign_year}-02-01"),
        &format!("{foreign_year}-12-31"),
    )
    .await;
    assert_eq!(foreign_snapshot.std_fs_version_id, Some(foreign_exact));
    assert_ne!(
        foreign_snapshot.std_fs_version_id,
        Some(foreign_domestic_fallback)
    );
    assert_eq!(
        foreign_snapshot.snapshot_data["std_fs"]["industry_type"],
        "GENERAL"
    );
    assert_eq!(
        foreign_snapshot.snapshot_data["std_fs"]["corp_type"],
        "FOREIGN"
    );

    let fallback_snapshot = create_customer_year_snapshot(
        &pool,
        &tenant_ref,
        "CON",
        Some("F42100"),
        Some("CONSOLIDATED"),
        final_fallback_year,
        &format!("{final_fallback_year}-02-01"),
        &format!("{final_fallback_year}-12-31"),
    )
    .await;
    assert_eq!(fallback_snapshot.std_fs_version_id, Some(final_fallback));
    assert_eq!(
        fallback_snapshot.snapshot_data["std_fs"]["industry_type"],
        "CONSTRUCTION"
    );
    assert_eq!(
        fallback_snapshot.snapshot_data["std_fs"]["corp_type"],
        "CONSOLIDATED"
    );
}

async fn seeded_std_fs_version(pool: &PgPool) -> StdFsItemVersion {
    sqlx::query_as::<_, StdFsItemVersion>(
        r#"
        SELECT id, version_code, industry_type, corp_type, effective_from, effective_to,
               nts_doc_ref, status, xml_schema_ver, created_by, reviewed_by,
               created_at, activated_at
        FROM std_fs_item_versions
        WHERE version_code = 'NTS-2024-GENERAL'
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seeded std fs version")
}

async fn insert_std_fs_version(
    pool: &PgPool,
    version_code: &str,
    industry_type: &str,
    corp_type: &str,
    effective_from: NaiveDate,
    status: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO std_fs_item_versions (
            version_code, industry_type, corp_type, effective_from, status,
            nts_doc_ref, xml_schema_ver, activated_at
        )
        VALUES ($1, $2, $3, $4, $5, 'test', 'TEST', NOW())
        RETURNING id
        "#,
    )
    .bind(version_code)
    .bind(industry_type)
    .bind(corp_type)
    .bind(effective_from)
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("insert std fs version")
}

async fn create_customer_year_snapshot(
    pool: &PgPool,
    tenant_ref: &cit_system::domain::TenantRef,
    code_suffix: &str,
    industry_code: Option<&str>,
    corp_type: Option<&str>,
    year_label: i32,
    start_date: &str,
    end_date: &str,
) -> cit_system::domain::LawSnapshot {
    let customer = tenant::create_customer(
        pool,
        tenant_ref,
        CreateCustomerRequest {
            customer_code: format!("C{code_suffix}{year_label}"),
            customer_name: format!("표준재무 {code_suffix}"),
            biz_reg_no: format!("{}081{}", year_label % 900 + 100, year_label % 9000 + 1000),
            corp_reg_no: Some(format!("{}1111111111", year_label % 900 + 100)),
            corp_type: corp_type.map(str::to_string),
            industry_code: industry_code.map(str::to_string),
            is_sme: Some(true),
            work_scopes: None,
        },
    )
    .await
    .expect("create customer");
    let business_year = tenant::create_business_year(
        pool,
        tenant_ref,
        CreateBusinessYearRequest {
            customer_id: customer.customer_id,
            year_label,
            start_date: date(start_date),
            end_date: date(end_date),
            carry_forward_from_by_id: None,
        },
    )
    .await
    .expect("create business year");
    tax::get_law_snapshot(pool, tenant_ref, business_year.by_id)
        .await
        .expect("snapshot created with business year")
}

async fn assert_std_fs_schema_contract(pool: &PgPool, schema_name: &str) {
    let standard_accounts_exists =
        sqlx::query_scalar::<_, bool>("SELECT to_regclass('public.standard_accounts') IS NOT NULL")
            .fetch_one(pool)
            .await
            .expect("standard_accounts existence check");
    assert!(
        standard_accounts_exists,
        "standard_accounts must exist in public schema"
    );

    for table in ["std_fs_mappings", "std_fs_statements"] {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT to_regclass(format('%I.%I', $1::TEXT, $2::TEXT)) IS NOT NULL",
        )
        .bind(schema_name)
        .bind(table)
        .fetch_one(pool)
        .await
        .expect("table existence check");
        assert!(exists, "{table} must exist in tenant schema");
    }
    for (table, column) in [
        ("account_mappings", "std_account_code"),
        ("fs_lines", "std_fs_item_code"),
        ("fs_lines", "std_account_code"),
        ("by_law_snapshot", "std_fs_version_id"),
    ] {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM information_schema.columns
                WHERE table_schema = $1 AND table_name = $2 AND column_name = $3
            )
            "#,
        )
        .bind(schema_name)
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await
        .expect("column existence check");
        assert!(exists, "{table}.{column} must exist");
    }
    let fk_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM information_schema.referential_constraints rc
        JOIN information_schema.table_constraints tc
          ON tc.constraint_catalog = rc.constraint_catalog
         AND tc.constraint_schema = rc.constraint_schema
         AND tc.constraint_name = rc.constraint_name
        WHERE tc.constraint_schema = $1
          AND tc.table_name IN ('by_law_snapshot', 'std_fs_mappings', 'std_fs_statements')
        "#,
    )
    .bind(schema_name)
    .fetch_one(pool)
    .await
    .expect("fk count");
    assert!(
        fk_count >= 3,
        "std fs tenant tables must have FK constraints"
    );
}

fn date(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("date")
}
