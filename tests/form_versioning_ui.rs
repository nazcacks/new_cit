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
async fn form_version_migration_relationships_and_cycle_check_work() {
    let (base_url, _state) = spawn_app().await;
    let token = login(&base_url).await;
    let client = authed_client(&token);
    let tenant_code = format!("form{}", &Uuid::new_v4().simple().to_string()[..9]);
    post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Form Test",
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
            "customer_code": "FRM001",
            "customer_name": "Form Customer",
            "biz_reg_no": "2208112345",
            "corp_reg_no": null,
            "industry_code": "62010",
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "PRINT"]
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
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/adjustments"),
        json!({"accounting_income": 50000000}),
        StatusCode::OK,
    )
    .await;
    post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/forms/FORM3"),
        json!({}),
        StatusCode::OK,
    )
    .await;

    let version = post_json(
        &client,
        &format!("{base_url}/api/form-versioning/versions"),
        json!({
            "form_code": "FORM3",
            "form_name": "Form 3 Test Version",
            "version_no": format!("2026.{}", &Uuid::new_v4().simple().to_string()[..6]),
            "effective_from": "2026-01-01",
            "effective_to": null,
            "template_json": {"fields": ["snapshot_id", "taxable_income", "total_tax_due", "new_field"]},
            "status": "DRAFT"
        }),
        StatusCode::CREATED,
    )
    .await;
    let version_id = version["form_version_id"].as_i64().unwrap();
    post_json(
        &client,
        &format!("{base_url}/api/form-versioning/versions/{version_id}/status"),
        json!({"status": "REVIEWED"}),
        StatusCode::OK,
    )
    .await;
    post_json(
        &client,
        &format!("{base_url}/api/form-versioning/versions/{version_id}/status"),
        json!({"status": "ACTIVE"}),
        StatusCode::OK,
    )
    .await;
    post_json(
        &client,
        &format!("{base_url}/api/form-versioning/relationships"),
        json!({
            "source_form": "FORM15",
            "source_field": "taxable_income",
            "target_form": "FORM3",
            "target_field": "taxable_income",
            "rule_json": {"operation": "COPY"},
            "effective_from": "2026-01-01",
            "effective_to": null
        }),
        StatusCode::CREATED,
    )
    .await;
    let cycle = get_json(
        &client,
        &format!("{base_url}/api/form-versioning/cycle-check"),
    )
    .await;
    assert_eq!(cycle["valid"], true);

    let migration = json!({
        "tenant_code": tenant_code,
        "by_id": by_id,
        "form_code": "FORM3",
        "to_version_id": version_id
    });
    let dry_run = post_json(
        &client,
        &format!("{base_url}/api/form-versioning/migrations/dry-run"),
        migration.clone(),
        StatusCode::OK,
    )
    .await;
    assert_eq!(dry_run["executable"], true);
    let executed = post_json(
        &client,
        &format!("{base_url}/api/form-versioning/migrations/execute"),
        migration.clone(),
        StatusCode::OK,
    )
    .await;
    assert_eq!(executed["mode"], "EXECUTE");
    let rollback = post_json(
        &client,
        &format!("{base_url}/api/form-versioning/migrations/rollback"),
        migration,
        StatusCode::OK,
    )
    .await;
    assert_eq!(rollback["mode"], "ROLLBACK");
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
