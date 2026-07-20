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
async fn phase_f_requires_confirmed_standard_financial_statements() {
    let (base_url, _) = spawn_seeded_app().await;
    let login = login_as(&base_url, "demo", "admin", "ChangeMe123!").await;
    let client = authenticated_client(login["token"].as_str().unwrap());

    let customer = post_json(
        &client,
        &format!("{base_url}/api/tenants/demo/customers"),
        json!({
            "customer_code": "PHASEF01",
            "customer_name": "Consistency Phase F",
            "biz_reg_no": "2228123456",
            "corp_reg_no": null,
            "industry_code": "62010",
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"]
        }),
        StatusCode::CREATED,
    )
    .await;
    let customer_id = customer["customer_id"].as_i64().unwrap();
    let year = post_json(
        &client,
        &format!("{base_url}/api/tenants/demo/business-years"),
        json!({
            "customer_id": customer_id,
            "year_label": 2026,
            "start_date": "2026-01-01",
            "end_date": "2026-12-31"
        }),
        StatusCode::CREATED,
    )
    .await;
    let by_id = year["by_id"].as_i64().unwrap();
    let root = format!("{base_url}/api/tenants/demo/business-years/{by_id}");
    let workspace_std = format!("{base_url}/api/workspace/{by_id}/std-fs");

    let fs_csv = "\
statement_type,account_code,account_name,debit,credit,standard_account_code,standard_account_name
BS,10100,Cash,1000,0,STD_CASH,Cash
BS,20100,Accounts payable,0,400,STD_PAYABLE,Accounts payable
BS,30100,Capital,0,600,STD_CAPITAL,Capital
IS,40100,Revenue,0,600,STD_PRODUCT_REVENUE,Revenue
IS,50100,Cost of goods sold,400,0,STD_COGS,Cost of goods sold
IS,51100,Salary expense,200,0,STD_SALARY,Salary expense
";
    post_csv_file(
        &client,
        &format!("{root}/tax-data/financial-statements/import"),
        "phase-f-fs.csv",
        fs_csv,
        StatusCode::CREATED,
    )
    .await;

    post_json(
        &client,
        &format!("{workspace_std}/mappings/bulk"),
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

    let std_validation = get_json(&client, &format!("{workspace_std}/validate")).await;
    assert_eq!(
        std_validation["valid"].as_bool(),
        Some(true),
        "{std_validation}"
    );
    assert_eq!(std_validation["confirmed"].as_bool(), Some(false));
    assert_validation_passed(&std_validation, "CHK_STDBS_BALANCE");
    assert_validation_passed(&std_validation, "CHK_STDBS_VS_FS");
    assert_validation_passed(&std_validation, "CHK_STDIS_VS_FS");
    assert_validation_passed(&std_validation, "CHK_STDFS_UNMAPPED");

    let before_confirm = post_json(
        &client,
        &format!("{root}/validation/run"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert_validation_issue(&before_confirm, "CHK_STDFS_CONFIRMED", "ERROR");

    let confirmed = post_json(
        &client,
        &format!("{workspace_std}/confirm"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert!(confirmed["confirmed_count"].as_u64().unwrap_or_default() > 0);
    assert_eq!(confirmed["validation"]["confirmed"].as_bool(), Some(true));

    let after_confirm = post_json(
        &client,
        &format!("{root}/validation/run"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert_validation_issue_absent(&after_confirm, "CHK_STDFS_CONFIRMED");
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

fn assert_validation_passed(result: &Value, rule_code: &str) {
    let issue = result["issues"]
        .as_array()
        .unwrap()
        .iter()
        .find(|issue| issue["rule_code"] == rule_code)
        .unwrap_or_else(|| panic!("{rule_code} issue missing"));
    assert_eq!(issue["passed"].as_bool(), Some(true), "{issue}");
}

fn assert_validation_issue(result: &Value, rule_code: &str, severity: &str) {
    let issue = result["issues"]
        .as_array()
        .unwrap()
        .iter()
        .find(|issue| issue["rule_code"] == rule_code)
        .unwrap_or_else(|| panic!("{rule_code} issue missing: {result}"));
    assert_eq!(issue["severity"].as_str(), Some(severity), "{issue}");
}

fn assert_validation_issue_absent(result: &Value, rule_code: &str) {
    let found = result["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["rule_code"] == rule_code);
    assert!(!found, "{rule_code} should not be emitted: {result}");
}
