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
async fn workflow_queue_events_and_unlock_follow_phase2_contract() {
    let (base_url, _state) = spawn_app().await;
    let token = login(&base_url).await;
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    let client = Client::builder().default_headers(headers).build().unwrap();

    let tenant_code = format!("wf{}", &Uuid::new_v4().simple().to_string()[..10]);
    post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Workflow Test",
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
            "customer_code": "WF001",
            "customer_name": "Workflow Customer",
            "biz_reg_no": "2208112345",
            "corp_reg_no": null,
            "industry_code": "62010",
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"]
        }),
        StatusCode::CREATED,
    )
    .await;
    let customer_id = customer["customer_id"].as_i64().unwrap();
    let by = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
        json!({
            "customer_id": customer_id,
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
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/adjustments"),
        json!({
            "accounting_income": 100_000_000_i64,
            "gross_revenue": 1_000_000_000_i64,
            "tax_credits": 1_000_000_i64
        }),
        StatusCode::OK,
    )
    .await;
    let original_form = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/forms/FORM3"),
        json!({}),
        StatusCode::OK,
    )
    .await;

    let rejected = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
        json!({"status": "FILED", "actor": "workflow-test"}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert_eq!(rejected["error"]["code"], "BAD_REQUEST");

    post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
        json!({"status": "IN_REVIEW", "actor": "workflow-test", "approver": "reviewer01"}),
        StatusCode::OK,
    )
    .await;
    let queue = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/workflow/queue?assignee=reviewer01"),
    )
    .await;
    let queue_item = queue
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["by_id"] == by_id)
        .unwrap_or_else(|| panic!("workflow queue missing business year {by_id}: {queue}"));
    assert_eq!(queue_item["customer_id"], customer_id);
    assert_eq!(queue_item["customer_name"], "Workflow Customer");
    assert_eq!(queue_item["year_label"], 2026);
    assert_eq!(queue_item["start_date"], "2026-01-01");
    assert_eq!(queue_item["end_date"], "2026-12-31");
    assert_eq!(queue_item["requester_login_id"], "workflow-test");
    assert_eq!(queue_item["approver_login_id"], "reviewer01");
    assert_eq!(queue_item["route_key"], "ws/appr:inbox");
    assert!(queue_item["pending_days"].as_i64().unwrap() >= 0);

    let event = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/workflow/events"),
        json!({"action": "COMMENT", "actor": "reviewer01", "comment": "review started"}),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(event["action"], "COMMENT");

    for status in ["APPROVED", "FILED"] {
        post_json(
            &client,
            &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
            json!({"status": status, "actor": "workflow-test", "approver": "reviewer01"}),
            StatusCode::OK,
        )
        .await;
    }

    let unlocked = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/unlock"),
        json!({"reason": "amendment", "actor": "workflow-test", "version_mode": "FILED_VERSION"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(unlocked["status"], "AMENDED");
    assert_ne!(unlocked["by_id"], by_id);
    assert!(unlocked["locked_at"].is_null());
    let amended_by_id = unlocked["by_id"].as_i64().unwrap();

    let original_workflow = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/workflow"),
    )
    .await;
    assert_eq!(original_workflow["business_year"]["status"], "FILED");
    let amendment_preview = get_json(
        &client,
        &format!(
            "{base_url}/api/tenants/{tenant_code}/business-years/{}/amendment-preview",
            amended_by_id
        ),
    )
    .await;
    assert_eq!(amendment_preview["original_by_id"].as_i64(), Some(by_id));
    assert_eq!(amendment_preview["amendment_sequence"].as_i64(), Some(1));
    let amended_form = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{amended_by_id}/forms/FORM3"),
    )
    .await;
    assert_eq!(
        amended_form["data_json"]["total_tax_due"],
        original_form["data_json"]["total_tax_due"]
    );
    put_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{amended_by_id}/forms/FORM3"),
        json!({
            "fields": {"tax_credits": 123456_i64},
            "reason": "amendment diff test",
            "changed_by": "workflow-test"
        }),
        StatusCode::OK,
    )
    .await;
    let changed_preview = get_json(
        &client,
        &format!(
            "{base_url}/api/tenants/{tenant_code}/business-years/{amended_by_id}/amendment-preview"
        ),
    )
    .await;
    assert!(changed_preview["differences"]
        .as_array()
        .expect("amendment differences")
        .iter()
        .any(|diff| diff["field"] == "FORM3.tax_credits"));
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

async fn get_json(client: &Client, url: &str) -> Value {
    let response = client.get(url).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert!(status.is_success(), "{text}");
    serde_json::from_str(&text).unwrap()
}
