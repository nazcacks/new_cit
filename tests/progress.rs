use std::env;

use axum::serve;
use cit_system::{
    db, router,
    seed::{self, DemoSeedOptions},
    AppState, Config,
};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client, StatusCode,
};
use serde_json::{json, Value};
use tokio::net::TcpListener;

#[tokio::test]
async fn business_year_progress_exposes_steps_status_and_next_leaf() {
    let (base_url, seed_result) = spawn_seeded_app().await;
    let login = login_as(&base_url, "admin").await;
    let client = authenticated_client(login["token"].as_str().unwrap());

    let progress = get_json(
        &client,
        &format!(
            "{base_url}/api/tenants/demo/business-years/{}/progress",
            seed_result.main_by_id
        ),
    )
    .await;
    assert_eq!(progress["tenant_code"], "demo");
    assert_eq!(progress["steps"].as_array().unwrap().len(), 8);
    assert!(progress["next_leaf"].as_str().unwrap().starts_with("ws/"));
    assert_eq!(
        progress["steps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|step| step["active"] == true)
            .count(),
        1
    );
    assert!(progress["recommendations"][0]["leaf_key"]
        .as_str()
        .is_some());

    let filed = get_json(
        &client,
        &format!(
            "{base_url}/api/tenants/demo/business-years/{}/progress",
            seed_result.filed_by_id
        ),
    )
    .await;
    assert_eq!(filed["status"], "FILED");
    assert_eq!(filed["progress"], 100);
    assert_eq!(filed["next_leaf"], "ws/file:done");
    assert!(filed["steps"]
        .as_array()
        .unwrap()
        .iter()
        .all(|step| step["done"] == true || step["code"] == "ws-file"));
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

async fn login_as(base_url: &str, login_id: &str) -> Value {
    let client = Client::new();
    let response = client
        .post(format!("{base_url}/api/auth/login"))
        .json(&json!({"tenant_code": "demo", "login_id": login_id, "password": "ChangeMe123!"}))
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
    let body = response.text().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    serde_json::from_str(&body).unwrap()
}
