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
async fn transaction_phase_d_and_vehicle_phase_e_reconcile_with_std_is_and_b10() {
    let (base_url, _) = spawn_seeded_app().await;
    let login = login_as(&base_url, "demo", "admin", "ChangeMe123!").await;
    let client = authenticated_client(login["token"].as_str().unwrap());

    let customer = post_json(
        &client,
        &format!("{base_url}/api/tenants/demo/customers"),
        json!({
            "customer_code": "TXVEH01",
            "customer_name": "Transaction Vehicle Phase D E",
            "biz_reg_no": "2218123456",
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

    let fs_csv = "\
statement_type,account_code,account_name,debit,credit,standard_account_code,standard_account_name
BS,10100,Cash,1000,0,STD_CASH,Cash
BS,11300,Company car,100000000,0,STD_VEHICLE,Vehicles
BS,20200,Borrowings,0,100000000,STD_LOAN,Borrowings
BS,30100,Capital,0,1000,STD_CAPITAL,Capital
IS,40100,Revenue,0,6000000,STD_PRODUCT_REVENUE,Revenue
IS,53100,Donation expense,1000000,0,STD_DONATION,Donation expense
IS,53200,Entertainment expense,2000000,0,STD_ENTERTAINMENT,Entertainment expense
IS,53300,Interest expense,3000000,0,STD_INTEREST_EXPENSE,Interest expense
";
    post_csv_file(
        &client,
        &format!("{root}/tax-data/financial-statements/import"),
        "phase-d-e-fs.csv",
        fs_csv,
        StatusCode::CREATED,
    )
    .await;

    post_json(
        &client,
        &format!("{base_url}/api/workspace/{by_id}/std-fs/mappings/bulk"),
        json!({
            "mappings": [
                {"account_code": "10100", "std_fs_item_code": "1010"},
                {"account_code": "11300", "std_fs_item_code": "1523"},
                {"account_code": "20200", "std_fs_item_code": "2020"},
                {"account_code": "30100", "std_fs_item_code": "3010"},
                {"account_code": "40100", "std_fs_item_code": "4010"},
                {"account_code": "53100", "std_fs_item_code": "5130"},
                {"account_code": "53200", "std_fs_item_code": "5140"},
                {"account_code": "53300", "std_fs_item_code": "5150"}
            ]
        }),
        StatusCode::OK,
    )
    .await;

    let tx_csv = "\
tx_date,partner_name,category,account_code,description,amount,evidence_type
2026-03-01,Special Charity,DONATION,53100,Special donation receipt,1000000,RECEIPT
2026-04-05,Client Dinner,ENTERTAINMENT,53200,Dinner meeting,2000000,CARD
2026-05-01,Trade Customer,RECEIVABLE,10200,Receivable source for B6,12000000,AR_LEDGER
2026-06-01,Main Bank,INTEREST,53300,General loan interest,3000000,WIRE
";
    let transactions_imported = post_csv_file(
        &client,
        &format!("{root}/tax-data/transactions/import"),
        "phase-d-transactions.csv",
        tx_csv,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(transactions_imported["batch"]["status"], "IMPORTED");

    let reconcile = get_json(
        &client,
        &format!("{root}/tax-data/transactions/is-reconcile"),
    )
    .await;
    assert_eq!(reconcile["valid"].as_bool(), Some(true), "{reconcile}");
    assert_transaction_issue_passed(&reconcile, "CHK_DONATION_TXN", 1_000_000);
    assert_transaction_issue_passed(&reconcile, "CHK_ENTERTAIN_TXN", 2_000_000);
    assert_transaction_issue_passed(&reconcile, "CHK_INTEREST_TXN", 3_000_000);
    assert_eq!(
        reconcile["totals"]["receivable_total"].as_i64(),
        Some(12_000_000)
    );

    let asset_csv = "\
asset_code,asset_name,asset_category,acquisition_date,acquisition_cost,useful_life_years
CARPHASEDE,Company car,VEHICLE,2026-01-01,100000000,5
";
    post_csv_file(
        &client,
        &format!("{root}/tax-data/assets/import"),
        "phase-e-assets.csv",
        asset_csv,
        StatusCode::CREATED,
    )
    .await;
    let assets = get_json(&client, &format!("{root}/tax-data/assets")).await;
    let vehicle_id = assets
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["asset_code"] == "CARPHASEDE")
        .and_then(|row| row["asset_id"].as_i64())
        .unwrap();
    let usage = post_json(
        &client,
        &format!("{root}/vehicle-usage-logs"),
        json!({
            "asset_id": vehicle_id,
            "usage_month": "2026-01-01",
            "total_distance_km": 1000.0,
            "business_distance_km": 600.0
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(usage["business_use_bps"], 6000);

    let before_b10 = get_json(&client, &format!("{root}/vehicle-usage-logs/b10-reconcile")).await;
    assert_eq!(before_b10["valid"].as_bool(), Some(false), "{before_b10}");
    assert_eq!(
        before_b10["rows"][0]["expected_addback"].as_i64(),
        Some(11_000_000)
    );
    assert_eq!(before_b10["rows"][0]["b10_item_amount"].as_i64(), Some(0));

    let b10 = post_json(
        &client,
        &format!("{root}/adjustments/assets/B10"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(b10["module_code"], "B10");
    assert_eq!(b10["addbacks"].as_i64(), Some(11_000_000));
    assert_eq!(
        b10["details"]["vehicles"][0]["business_use_bps"].as_i64(),
        Some(6000)
    );

    let after_b10 = get_json(&client, &format!("{root}/vehicle-usage-logs/b10-reconcile")).await;
    assert_eq!(after_b10["valid"].as_bool(), Some(true), "{after_b10}");
    assert_eq!(
        after_b10["rows"][0]["b10_item_amount"].as_i64(),
        Some(11_000_000)
    );

    let validation = post_json(
        &client,
        &format!("{root}/validation/run"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    for rule_code in [
        "CHK_DONATION_TXN",
        "CHK_ENTERTAIN_TXN",
        "CHK_INTEREST_TXN",
        "CHK_VEHICLE_USAGE_BPS",
        "CHK_B10_LINK",
    ] {
        assert_no_validation_issue(&validation, rule_code);
    }
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

fn assert_transaction_issue_passed(result: &Value, rule_code: &str, amount: i64) {
    let issue = result["issues"]
        .as_array()
        .unwrap()
        .iter()
        .find(|issue| issue["rule_code"] == rule_code)
        .unwrap_or_else(|| panic!("{rule_code} issue missing"));
    assert_eq!(issue["passed"].as_bool(), Some(true), "{issue}");
    assert_eq!(issue["transaction_total"].as_i64(), Some(amount));
    assert_eq!(issue["is_total"].as_i64(), Some(amount));
    assert_eq!(issue["std_is_total"].as_i64(), Some(amount));
}

fn assert_no_validation_issue(result: &Value, rule_code: &str) {
    let found = result["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["rule_code"] == rule_code);
    assert!(!found, "{rule_code} should not be emitted: {result}");
}
