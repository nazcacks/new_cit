use std::env;

use axum::serve;
use cit_system::{
    db, router,
    seed::{self, DemoSeedOptions},
    AppState, Config,
};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    multipart::{Form, Part},
    Client, StatusCode,
};
use serde_json::{json, Value};
use tokio::net::TcpListener;

#[tokio::test]
async fn workspace_std_fs_mappings_save_bulk_validate_leaf_and_carry_forward() {
    let (base_url, seed_result) = spawn_seeded_app().await;
    let login = login_as(&base_url, "demo", "admin", "ChangeMe123!").await;
    let client = authenticated_client(login["token"].as_str().unwrap());
    let by_id = seed_result.main_by_id;

    let root = format!("{base_url}/api/workspace/{by_id}/std-fs/mappings");
    let initial = get_json(&client, &root).await;
    let initial_rows = initial.as_array().expect("std-fs mappings");
    assert!(initial_rows.len() >= 20);
    assert_mapping_code(&initial, "10100", Some("1010"));

    let subtotal_rejected = put_json(
        &client,
        &format!("{root}/10100"),
        json!({"std_fs_item_code": "1000"}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert!(subtotal_rejected["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("leaf item is required"));

    let saved = put_json(
        &client,
        &format!("{root}/10100"),
        json!({"std_fs_item_code": "1030"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(saved["updated_count"], 1);
    assert_mapping_code(&saved["mappings"], "10100", Some("1030"));

    let bulk = post_json(
        &client,
        &format!("{root}/bulk"),
        json!({
            "mappings": [
                {"account_code": "10200", "std_fs_item_code": "1010"},
                {"account_code": "10300", "std_fs_item_code": "1200"}
            ]
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(bulk["updated_count"], 2);
    assert_mapping_code(&bulk["mappings"], "10200", Some("1010"));
    assert_mapping_code(&bulk["mappings"], "10300", Some("1200"));

    let tenant_list = get_json(
        &client,
        &format!("{base_url}/api/tenants/demo/business-years/{by_id}/std-fs/mappings"),
    )
    .await;
    assert_mapping_code(&tenant_list, "10200", Some("1010"));

    let years = get_json(
        &client,
        &format!("{base_url}/api/tenants/demo/business-years"),
    )
    .await;
    let main_year = business_year_by_id(&years, by_id);
    let customer_id = main_year["customer_id"].as_i64().expect("customer_id");
    let target_by_id = years
        .as_array()
        .expect("business years")
        .iter()
        .find(|year| {
            year["customer_id"].as_i64() == Some(customer_id)
                && year["year_label"].as_i64() == Some(2025)
        })
        .and_then(|year| year["by_id"].as_i64())
        .expect("target business year");

    let target_csv = "\
statement_type,account_code,account_name,debit,credit
BS,10100,Cash,1000,0
BS,20100,Accounts payable,0,1000
";
    post_csv_file(
        &client,
        &format!(
            "{base_url}/api/tenants/demo/business-years/{target_by_id}/tax-data/financial-statements/import"
        ),
        "target-fs.csv",
        target_csv,
        StatusCode::CREATED,
    )
    .await;

    let before_lines = get_json(
        &client,
        &format!(
            "{base_url}/api/tenants/demo/business-years/{target_by_id}/tax-data/financial-statements"
        ),
    )
    .await;
    assert_line_std_fs_code(&before_lines, "10100", None);

    let carried = post_json(
        &client,
        &format!("{base_url}/api/workspace/{target_by_id}/std-fs/mappings/carry-forward"),
        json!({"source_by_id": by_id}),
        StatusCode::OK,
    )
    .await;
    assert!(carried["copied_count"].as_u64().unwrap_or_default() > 0);
    assert_eq!(carried["source_by_id"], by_id);

    let after_lines = get_json(
        &client,
        &format!(
            "{base_url}/api/tenants/demo/business-years/{target_by_id}/tax-data/financial-statements"
        ),
    )
    .await;
    assert_line_std_fs_code(&after_lines, "10100", Some("1030"));

    let agg_customer = post_json(
        &client,
        &format!("{base_url}/api/tenants/demo/customers"),
        json!({
            "customer_code": "STDFSAGG01",
            "customer_name": "Std FS Aggregate",
            "biz_reg_no": "2208119999",
            "corp_reg_no": null,
            "industry_code": "62010",
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"]
        }),
        StatusCode::CREATED,
    )
    .await;
    let agg_customer_id = agg_customer["customer_id"].as_i64().unwrap();
    let agg_year = post_json(
        &client,
        &format!("{base_url}/api/tenants/demo/business-years"),
        json!({
            "customer_id": agg_customer_id,
            "year_label": 2026,
            "start_date": "2026-01-01",
            "end_date": "2026-12-31"
        }),
        StatusCode::CREATED,
    )
    .await;
    let agg_by_id = agg_year["by_id"].as_i64().unwrap();

    let agg_csv = "\
statement_type,account_code,account_name,debit,credit
BS,10100,Cash,1000,0
BS,20100,Accounts payable,0,400
BS,30100,Capital stock,0,600
IS,40100,Sales,0,500
IS,50100,Cost of sales,300,0
IS,51100,Salaries,200,0
";
    post_csv_file(
        &client,
        &format!(
            "{base_url}/api/tenants/demo/business-years/{agg_by_id}/tax-data/financial-statements/import"
        ),
        "std-fs-aggregate.csv",
        agg_csv,
        StatusCode::CREATED,
    )
    .await;

    let agg_root = format!("{base_url}/api/workspace/{agg_by_id}/std-fs");
    post_json(
        &client,
        &format!("{agg_root}/mappings/bulk"),
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

    let aggregated = post_json(
        &client,
        &format!("{agg_root}/aggregate"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert!(aggregated["validation"]["valid"].as_bool().unwrap());
    assert_statement_amount(&aggregated["statements"], "1000", 1000);
    assert_statement_amount(&aggregated["statements"], "2000", 400);
    assert_statement_amount(&aggregated["statements"], "3000", 600);
    assert_validation_passed(&aggregated["validation"], "CHK_STDBS_BALANCE");
    assert_validation_passed(&aggregated["validation"], "CHK_STDBS_VS_FS");
    assert_validation_passed(&aggregated["validation"], "CHK_STDIS_VS_FS");
    assert_validation_passed(&aggregated["validation"], "CHK_STDFS_UNMAPPED");

    let validation = get_json(&client, &format!("{agg_root}/validate")).await;
    assert!(validation["valid"].as_bool().unwrap());
    assert_eq!(validation["unmapped_count"].as_i64(), Some(0));

    let confirmed = post_json(
        &client,
        &format!("{agg_root}/confirm"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert!(confirmed["validation"]["valid"].as_bool().unwrap());
    assert!(confirmed["confirmed_count"].as_u64().unwrap_or_default() > 0);

    let statements = get_json(&client, &format!("{agg_root}/statements?stmtType=STD_BS")).await;
    assert_statement_amount(&statements, "1000", 1000);
    assert_statement_amount(&statements, "1010", 1000);
    assert!(find_statement_line(&statements, "1010")["confirmed"]
        .as_bool()
        .unwrap());

    put_json(
        &client,
        &format!("{agg_root}/mappings/10100"),
        json!({"std_fs_item_code": "1030"}),
        StatusCode::OK,
    )
    .await;
    let frozen = get_json(&client, &format!("{agg_root}/statements?stmtType=STD_BS")).await;
    assert_statement_amount(&frozen, "1010", 1000);
    assert_statement_amount(&frozen, "1030", 0);

    for status in ["IN_REVIEW", "APPROVED", "FILED"] {
        post_json(
            &client,
            &format!("{base_url}/api/tenants/demo/business-years/{agg_by_id}/status"),
            json!({"status": status, "actor": "std-fs-test", "approver": "reviewer01"}),
            StatusCode::OK,
        )
        .await;
    }
    let locked_confirm = post_json(
        &client,
        &format!("{agg_root}/confirm"),
        json!({}),
        StatusCode::CONFLICT,
    )
    .await;
    assert!(locked_confirm["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("locked after FILED"));
    post_json(
        &client,
        &format!("{agg_root}/aggregate"),
        json!({}),
        StatusCode::CONFLICT,
    )
    .await;
}

async fn spawn_seeded_app() -> (String, seed::DemoSeedResult) {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let pool = db::connect(&database_url).await.expect("db connection");
    db::migrate(&pool).await.expect("migrations");
    let seed_result = seed::run_demo_seed(
        &pool,
        DemoSeedOptions {
            reset: true,
            ..DemoSeedOptions::default()
        },
    )
    .await
    .expect("demo seed");
    let state = AppState::new(pool, Config::test(database_url));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(state);
    tokio::spawn(async move {
        serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), seed_result)
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

async fn post_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.post(url).json(&body).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn put_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.put(url).json(&body).send().await.unwrap();
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

fn assert_mapping_code(mappings: &Value, account_code: &str, expected: Option<&str>) {
    let row = mappings
        .as_array()
        .expect("mapping rows")
        .iter()
        .find(|row| row["account_code"] == account_code)
        .unwrap_or_else(|| panic!("missing mapping row for {account_code}"));
    assert_eq!(row["std_fs_item_code"].as_str(), expected);
}

fn business_year_by_id(years: &Value, by_id: i64) -> &Value {
    years
        .as_array()
        .expect("business years")
        .iter()
        .find(|year| year["by_id"].as_i64() == Some(by_id))
        .unwrap_or_else(|| panic!("missing business year {by_id}"))
}

fn assert_line_std_fs_code(lines: &Value, account_code: &str, expected: Option<&str>) {
    let row = lines
        .as_array()
        .expect("financial statement lines")
        .iter()
        .find(|row| row["account_code"] == account_code)
        .unwrap_or_else(|| panic!("missing fs line {account_code}"));
    assert_eq!(row["std_fs_item_code"].as_str(), expected);
}

fn find_statement_line<'a>(statements: &'a Value, item_code: &str) -> &'a Value {
    statements
        .as_array()
        .expect("statement lines")
        .iter()
        .find(|row| row["item_code"] == item_code)
        .unwrap_or_else(|| panic!("missing statement line {item_code}: {statements}"))
}

fn assert_statement_amount(statements: &Value, item_code: &str, expected: i64) {
    let row = find_statement_line(statements, item_code);
    assert_eq!(row["amount"].as_i64(), Some(expected), "{row}");
}

fn assert_validation_passed(validation: &Value, rule_code: &str) {
    let issue = validation["issues"]
        .as_array()
        .expect("validation issues")
        .iter()
        .find(|issue| issue["rule_code"] == rule_code)
        .unwrap_or_else(|| panic!("missing validation issue {rule_code}: {validation}"));
    assert_eq!(issue["passed"].as_bool(), Some(true), "{issue}");
}
