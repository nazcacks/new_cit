use std::env;

use axum::serve;
use cit_system::{db, router, AppState, Config};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client, StatusCode,
};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use uuid::Uuid;

#[tokio::test]
async fn multi_step_approval_advances_lines_and_returns_to_draft_on_rejection() {
    let (base_url, _state) = spawn_app().await;
    let token = login(&base_url).await;
    let client = authed_client(&token);
    let tenant_code = format!("appr{}", &Uuid::new_v4().simple().to_string()[..9]);
    post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Approval Test",
            "biz_reg_no": "1234567890",
            "contract_start": "2026-01-01",
            "contract_end": null,
            "max_users": 5
        }),
        StatusCode::CREATED,
    )
    .await;
    let customer = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/customers"),
        json!({
            "customer_code": "APR001",
            "customer_name": "Approval Customer",
            "biz_reg_no": "2208112345",
            "corp_reg_no": null,
            "industry_code": "62010",
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT"]
        }),
        StatusCode::CREATED,
    )
    .await;
    let by = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
        json!({
            "customer_id": customer["customer_id"].as_i64().unwrap(),
            "year_label": 2026,
            "start_date": "2026-01-01",
            "end_date": "2026-12-31"
        }),
        StatusCode::CREATED,
    )
    .await;
    let by_id = by["by_id"].as_i64().unwrap();
    post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
        json!({
            "status": "IN_REVIEW",
            "actor": "writer01",
            "approvers": ["reviewer01", "partner01"],
            "comment": "submit"
        }),
        StatusCode::OK,
    )
    .await;
    let workflow = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/workflow"),
    )
    .await;
    assert_eq!(workflow["approval_lines"].as_array().unwrap().len(), 2);

    let partial = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
        json!({"status": "APPROVED", "actor": "reviewer01", "approver": "reviewer01"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(partial["status"], "IN_REVIEW");

    let approved = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
        json!({"status": "APPROVED", "actor": "partner01", "approver": "partner01"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(approved["status"], "APPROVED");

    post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
        json!({"status": "IN_REVIEW", "actor": "partner01", "approver": "reviewer01"}),
        StatusCode::OK,
    )
    .await;
    let returned = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
        json!({"status": "DRAFT", "actor": "reviewer01", "comment": "needs work"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(returned["status"], "DRAFT");
    let final_workflow = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/workflow"),
    )
    .await;
    assert!(final_workflow["approval_lines"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line["status"] == "RETURNED"));
}

async fn spawn_app() -> (String, AppState) {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let pool = db::connect(&database_url).await.expect("db connection");
    db::migrate(&pool).await.expect("migrations");
    let state = AppState::new(pool, Config::test(database_url));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(state.clone());
    tokio::spawn(async move {
        serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), state)
}

async fn login(base_url: &str) -> String {
    let client = Client::new();
    let auth = post_json(
        &client,
        &format!("{base_url}/api/auth/login"),
        json!({"tenant_code": "demo", "login_id": "admin", "password": "ChangeMe123!"}),
        StatusCode::OK,
    )
    .await;
    auth["token"].as_str().unwrap().to_string()
}

fn authed_client(token: &str) -> Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    Client::builder().default_headers(headers).build().unwrap()
}

async fn post_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.post(url).json(&body).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn get_json(client: &Client, url: &str) -> Value {
    let response = client.get(url).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert!(status.is_success(), "{text}");
    serde_json::from_str(&text).unwrap()
}
