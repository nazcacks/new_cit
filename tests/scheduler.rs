use std::env;

use axum::serve;
use chrono::{Datelike, Duration, Utc};
use cit_system::{db, router, AppState, Config};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client, StatusCode,
};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use uuid::Uuid;

#[tokio::test]
async fn due_alert_scheduler_creates_d30_and_d7_once() {
    let (base_url, _state) = spawn_app().await;
    let token = login(&base_url).await;
    let client = authed_client(&token);
    let tenant_code = format!("sch{}", &Uuid::new_v4().simple().to_string()[..10]);
    post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Scheduler Test",
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
            "customer_code": "SCH001",
            "customer_name": "Scheduler Customer",
            "biz_reg_no": "2208112345",
            "corp_reg_no": null,
            "industry_code": "62010",
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "PRINT"]
        }),
        StatusCode::CREATED,
    )
    .await;
    let end = Utc::now().date_naive() + Duration::days(5);
    let start = end - Duration::days(30);
    post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
        json!({
            "customer_id": customer["customer_id"].as_i64().unwrap(),
            "year_label": end.year(),
            "start_date": start,
            "end_date": end
        }),
        StatusCode::CREATED,
    )
    .await;

    let first = post_json(
        &client,
        &format!("{base_url}/api/operations/scheduler/due-alerts/run"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    let created = first["created"].as_i64().unwrap();
    assert!(created >= 2, "{first}");
    let second = post_json(
        &client,
        &format!("{base_url}/api/operations/scheduler/due-alerts/run"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(second["created"], 0);
    let notifications = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/notifications"),
    )
    .await;
    let buckets = notifications
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["metadata"]["due_bucket"].as_str())
        .collect::<Vec<_>>();
    assert!(buckets.contains(&"D-30"));
    assert!(buckets.contains(&"D-7"));
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
