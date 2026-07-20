use std::env;

use axum::serve;
use cit_system::{db, queue, router, seed, AppState, Config};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    multipart::{Form, Part},
    Client, StatusCode,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

#[tokio::test]
async fn phase_g_generates_confirmed_std_fs_xml_and_blocks_precheck_failures() {
    let (base_url, state) = spawn_seeded_app().await;
    let login = login_as(&base_url, "demo", "admin", "ChangeMe123!").await;
    let client = authenticated_client(login["token"].as_str().unwrap());

    let missing_master_by_id =
        create_phase_g_year(&client, &base_url, "PHASEG_NOMASTER", 2025, "62010").await;
    let missing_master_root =
        format!("{base_url}/api/tenants/demo/business-years/{missing_master_by_id}");
    let missing_master =
        get_json(&client, &format!("{missing_master_root}/efilings/precheck")).await;
    assert_eq!(
        missing_master["valid"].as_bool(),
        Some(false),
        "{missing_master}"
    );
    assert_eq!(missing_master["record_count"].as_i64(), Some(0));
    assert_precheck_issue(&missing_master, "EFILE_MASTER_MISSING");

    let by_id = create_phase_g_year(&client, &base_url, "PHASEG_OK", 2026, "62010").await;
    let root = format!("{base_url}/api/tenants/demo/business-years/{by_id}");
    import_phase_g_fs(&client, &root, 1000, 400, 600).await;
    calculate_positive_tax(&client, &root).await;
    map_seed_std_fs(&client, &root).await;

    let unconfirmed = get_json(&client, &format!("{root}/efilings/precheck")).await;
    assert_eq!(unconfirmed["valid"].as_bool(), Some(false), "{unconfirmed}");
    assert_precheck_issue(&unconfirmed, "EFILE_STDFS_CONFIRMED");

    let confirmed = post_json(
        &client,
        &format!("{root}/std-fs/confirm"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert!(confirmed["confirmed_count"].as_u64().unwrap_or_default() > 0);

    let ready = get_json(&client, &format!("{root}/efilings/precheck")).await;
    assert_eq!(ready["valid"].as_bool(), Some(true), "{ready}");
    assert!(ready["record_count"].as_i64().unwrap_or_default() > 3);

    let job = post_json(
        &client,
        &format!("{root}/efilings"),
        json!({"max_attempts": 1}),
        StatusCode::ACCEPTED,
    )
    .await;
    run_until_job_status(&state, job["job_id"].as_str().unwrap(), "succeeded").await;
    let histories = get_json(&client, &format!("{root}/efilings")).await;
    let efiling_id = histories[0]["efiling_id"].as_i64().unwrap();
    let file = get_bytes(
        &client,
        &format!("{base_url}/api/tenants/demo/business-years/{by_id}/efilings/{efiling_id}/file"),
    )
    .await;
    let file_text = String::from_utf8_lossy(&file);
    assert!(file_text.contains("<stdFsRecord"), "{file_text}");
    assert!(file_text.contains("stmtType=\"STD_BS\""), "{file_text}");
    assert!(
        file_text.contains("xmlFieldId=\"BS_ASSET_TOTAL\""),
        "{file_text}"
    );
    assert!(file_text.contains("stmtType=\"STD_IS\""), "{file_text}");
    assert!(
        file_text.contains("xmlFieldId=\"IS_REVENUE_TOTAL\""),
        "{file_text}"
    );

    import_phase_g_fs(&client, &root, 1200, 400, 800).await;
    let stale_totals = get_json(&client, &format!("{root}/efilings/precheck")).await;
    assert_eq!(
        stale_totals["valid"].as_bool(),
        Some(false),
        "{stale_totals}"
    );
    assert_precheck_issue(&stale_totals, "EFILE_STDFS_TOTALS");

    let missing_xml_year = 2030 + (Uuid::new_v4().as_u128() % 5000) as i32;
    insert_missing_xml_std_fs_version(&state.pool, missing_xml_year).await;
    let missing_by_id =
        create_phase_g_year(&client, &base_url, "PHASEG_XML", missing_xml_year, "K64110").await;
    let missing_root = format!("{base_url}/api/tenants/demo/business-years/{missing_by_id}");
    import_phase_g_fs(&client, &missing_root, 1000, 400, 600).await;
    calculate_positive_tax(&client, &missing_root).await;
    map_custom_std_fs(&client, &missing_root).await;
    post_json(
        &client,
        &format!("{missing_root}/std-fs/confirm"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    let missing_xml = get_json(&client, &format!("{missing_root}/efilings/precheck")).await;
    assert_eq!(missing_xml["valid"].as_bool(), Some(false), "{missing_xml}");
    assert_precheck_issue(&missing_xml, "EFILE_STDFS_XML_FIELD");
}

async fn spawn_seeded_app() -> (String, AppState) {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let pool = db::connect(&database_url).await.expect("db connection");
    db::migrate(&pool).await.expect("migrations");
    seed::run_demo_seed(
        &pool,
        seed::DemoSeedOptions {
            reset: true,
            ..seed::DemoSeedOptions::default()
        },
    )
    .await
    .expect("demo seed");
    let state = AppState::new(pool, Config::test(database_url));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(state.clone());
    tokio::spawn(async move {
        serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), state)
}

async fn create_phase_g_year(
    client: &Client,
    base_url: &str,
    customer_code: &str,
    year_label: i32,
    industry_code: &str,
) -> i64 {
    let suffix = Uuid::new_v4().simple().to_string();
    let customer = post_json(
        client,
        &format!("{base_url}/api/tenants/demo/customers"),
        json!({
            "customer_code": format!("{customer_code}{}", &suffix[..6]),
            "customer_name": format!("Phase G {customer_code}"),
            "biz_reg_no": "2208112345",
            "corp_reg_no": null,
            "industry_code": industry_code,
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"]
        }),
        StatusCode::CREATED,
    )
    .await;
    let customer_id = customer["customer_id"].as_i64().unwrap();
    let year = post_json(
        client,
        &format!("{base_url}/api/tenants/demo/business-years"),
        json!({
            "customer_id": customer_id,
            "year_label": year_label,
            "start_date": format!("{year_label}-01-01"),
            "end_date": format!("{year_label}-12-31")
        }),
        StatusCode::CREATED,
    )
    .await;
    year["by_id"].as_i64().unwrap()
}

async fn import_phase_g_fs(client: &Client, root: &str, asset: i64, liability: i64, equity: i64) {
    let csv = format!(
        "\
statement_type,account_code,account_name,debit,credit,standard_account_code,standard_account_name
BS,10100,Cash,{asset},0,STD_CASH,Cash
BS,20100,Accounts payable,0,{liability},STD_PAYABLE,Accounts payable
BS,30100,Capital,0,{equity},STD_CAPITAL,Capital
IS,40100,Revenue,0,600,STD_PRODUCT_REVENUE,Revenue
IS,50100,Cost of goods sold,400,0,STD_COGS,Cost of goods sold
IS,51100,Salary expense,200,0,STD_SALARY,Salary expense
"
    );
    post_csv_file(
        client,
        &format!("{root}/tax-data/financial-statements/import"),
        "phase-g-fs.csv",
        &csv,
        StatusCode::CREATED,
    )
    .await;
}

async fn calculate_positive_tax(client: &Client, root: &str) {
    post_json(
        client,
        &format!("{root}/adjustments"),
        json!({
            "accounting_income": 100000000,
            "gross_revenue": 200000000,
            "donations": 0,
            "entertainment_expense": 0,
            "depreciation_book": 0,
            "depreciation_tax_limit": 0,
            "carryforward_loss": 0,
            "tax_credits": 0
        }),
        StatusCode::OK,
    )
    .await;
}

async fn map_seed_std_fs(client: &Client, root: &str) {
    post_json(
        client,
        &format!("{root}/std-fs/mappings/bulk"),
        json!({
            "mappings": [
                {"account_code": "10100", "std_fs_item_code": "1010"},
                {"account_code": "20100", "std_fs_item_code": "2010"},
                {"account_code": "30100", "std_fs_item_code": "3010"},
                {"account_code": "40100", "std_fs_item_code": "4010"},
                {"account_code": "50100", "std_fs_item_code": "4510"},
                {"account_code": "51100", "std_fs_item_code": "5110"}
            ]
        }),
        StatusCode::OK,
    )
    .await;
}

async fn map_custom_std_fs(client: &Client, root: &str) {
    post_json(
        client,
        &format!("{root}/std-fs/mappings/bulk"),
        json!({
            "mappings": [
                {"account_code": "10100", "std_fs_item_code": "1000"},
                {"account_code": "20100", "std_fs_item_code": "2000"},
                {"account_code": "30100", "std_fs_item_code": "3000"},
                {"account_code": "40100", "std_fs_item_code": "4010"},
                {"account_code": "50100", "std_fs_item_code": "4510"},
                {"account_code": "51100", "std_fs_item_code": "5110"}
            ]
        }),
        StatusCode::OK,
    )
    .await;
}

async fn insert_missing_xml_std_fs_version(pool: &PgPool, year_label: i32) {
    let suffix = Uuid::new_v4().simple().to_string();
    let version_code = format!("PG-MISS-{}", &suffix[..12]);
    let version_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO std_fs_item_versions (
            version_code, industry_type, corp_type, effective_from, status,
            nts_doc_ref, xml_schema_ver, activated_at
        )
        VALUES ($1, 'FINANCE', 'DOMESTIC', $2, 'ACTIVE',
                'phase-g missing xml fixture', 'PHASE-G', NOW())
        RETURNING id
        "#,
    )
    .bind(version_code)
    .bind(
        format!("{year_label}-01-01")
            .parse::<chrono::NaiveDate>()
            .unwrap(),
    )
    .fetch_one(pool)
    .await
    .expect("insert phase-g std-fs version");

    let rows = [
        ("STD_BS", "1000", "Assets", "ASSET", "DEBIT", None, 100),
        (
            "STD_BS",
            "2000",
            "Liabilities",
            "LIABILITY",
            "CREDIT",
            Some("BS_LIABILITY_TOTAL"),
            200,
        ),
        (
            "STD_BS",
            "3000",
            "Equity",
            "EQUITY",
            "CREDIT",
            Some("BS_EQUITY_TOTAL"),
            300,
        ),
        (
            "STD_IS",
            "4010",
            "Revenue",
            "REVENUE",
            "CREDIT",
            Some("IS_REVENUE_TOTAL"),
            400,
        ),
        (
            "STD_IS",
            "4510",
            "Cost of goods sold",
            "EXPENSE",
            "DEBIT",
            Some("IS_COGS"),
            451,
        ),
        (
            "STD_IS",
            "5110",
            "Salary",
            "EXPENSE",
            "DEBIT",
            Some("IS_SALARY"),
            511,
        ),
    ];
    for (
        stmt_type,
        item_code,
        item_name,
        account_class,
        normal_balance,
        xml_field_id,
        sort_order,
    ) in rows
    {
        sqlx::query(
            r#"
            INSERT INTO std_fs_items (
                version_id, stmt_type, item_code, item_name, level, account_class,
                normal_balance, is_subtotal, is_required, xml_field_id, sort_order, is_active
            )
            VALUES ($1, $2, $3, $4, 1, $5, $6, FALSE, TRUE, $7, $8, TRUE)
            "#,
        )
        .bind(version_id)
        .bind(stmt_type)
        .bind(item_code)
        .bind(item_name)
        .bind(account_class)
        .bind(normal_balance)
        .bind(xml_field_id)
        .bind(sort_order)
        .execute(pool)
        .await
        .expect("insert phase-g std-fs item");
    }
}

async fn run_until_job_status(state: &AppState, job_id: &str, expected: &str) -> Value {
    let id = job_id.parse::<Uuid>().expect("valid job id");
    for _ in 0..50 {
        queue::run_once(state.clone())
            .await
            .expect("worker iteration");
        let job = queue::get_job(&state.pool, id).await.expect("job exists");
        if job.status == expected {
            return serde_json::to_value(job).expect("job json");
        }
    }
    let job = queue::get_job(&state.pool, id).await.expect("job exists");
    panic!(
        "job {job_id} did not reach {expected}; current status={}",
        job.status
    );
}

async fn login_as(base_url: &str, tenant_code: &str, login_id: &str, password: &str) -> Value {
    let client = Client::new();
    let response = client
        .post(format!("{base_url}/api/auth/login"))
        .json(&json!({
            "tenant_code": tenant_code,
            "login_id": login_id,
            "password": password
        }))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    serde_json::from_str(&body).unwrap()
}

fn authenticated_client(token: &str) -> Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    Client::builder().default_headers(headers).build().unwrap()
}

async fn get_json(client: &Client, url: &str) -> Value {
    let response = client.get(url).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert!(status.is_success(), "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn get_bytes(client: &Client, url: &str) -> Vec<u8> {
    let response = client.get(url).send().await.unwrap();
    let status = response.status();
    let bytes = response.bytes().await.unwrap();
    assert!(status.is_success(), "{}", String::from_utf8_lossy(&bytes));
    bytes.to_vec()
}

async fn post_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.post(url).json(&body).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn post_csv_file(
    client: &Client,
    url: &str,
    file_name: &str,
    csv: &str,
    expected: StatusCode,
) -> Value {
    let form = Form::new().part(
        "file",
        Part::text(csv.to_string())
            .file_name(file_name.to_string())
            .mime_str("text/csv")
            .unwrap(),
    );
    let response = client.post(url).multipart(form).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}

fn assert_precheck_issue(result: &Value, validation_code: &str) {
    assert!(
        result["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |issue| issue["validation_code"] == validation_code && issue["severity"] == "ERROR"
            ),
        "{validation_code} missing from {result}"
    );
}
