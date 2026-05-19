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
async fn law_version_review_activation_history_and_impact_work() {
    let (base_url, _state) = spawn_app().await;
    let token = login(&base_url).await;
    let client = authed_client(&token);
    let version_code = format!("CIT-{}", &Uuid::new_v4().simple().to_string()[..8]);
    let law = post_json(
        &client,
        &format!("{base_url}/api/tax-laws"),
        json!({
            "version_code": version_code,
            "law_name": "Workflow Test Law",
            "effective_from": "2026-01-01",
            "effective_to": null,
            "metadata": {"source": "law_workflow_test"}
        }),
        StatusCode::CREATED,
    )
    .await;
    let law_version_id = law["law_version_id"].as_i64().unwrap();
    post_json(
        &client,
        &format!("{base_url}/api/tax-laws/{law_version_id}/status"),
        json!({"status": "REVIEWED", "change_summary": "reviewed", "approved_by": "reviewer01"}),
        StatusCode::OK,
    )
    .await;
    let active = post_json(
        &client,
        &format!("{base_url}/api/tax-laws/{law_version_id}/status"),
        json!({"status": "ACTIVE", "change_summary": "activated", "approved_by": "partner01"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(active["status"], "ACTIVE");
    let history = get_json(
        &client,
        &format!("{base_url}/api/law-amendments?law_version_id={law_version_id}"),
    )
    .await;
    assert!(history.as_array().unwrap().len() >= 2);
    let impact = post_json(
        &client,
        &format!("{base_url}/api/law-versioning/impact"),
        json!({"law_version_id": law_version_id, "include_locked": true}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(impact["law"]["law_version_id"], law_version_id);
    assert!(impact["tenant_impacts"].as_array().is_some());
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
