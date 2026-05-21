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
async fn tenant_suggest_and_super_admin_switch_tenant_issue_new_session() {
    let (base_url, _) = spawn_seeded_app().await;
    let client = Client::new();

    let suggest = client
        .get(format!("{base_url}/api/public/tenant-suggest?q=demo"))
        .send()
        .await
        .unwrap();
    assert_eq!(suggest.status(), StatusCode::OK);
    let suggest_body = suggest.json::<Value>().await.unwrap();
    assert!(suggest_body
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["tenant_code"] == "demo"));

    let login = login_as(&base_url, "admin").await;
    assert_eq!(collect_leaves(&login["modules"]).len(), 100);
    assert!(login["accessible_tenants"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["tenant_code"] == "samplefirm"));
    let old_token = login["token"].as_str().unwrap();
    let admin_client = authenticated_client(old_token);

    let switched = admin_client
        .post(format!("{base_url}/api/auth/switch-tenant"))
        .json(&json!({"tenant_code": "samplefirm"}))
        .send()
        .await
        .unwrap();
    let status = switched.status();
    let body = switched.text().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    let switched = serde_json::from_str::<Value>(&body).unwrap();
    assert_ne!(switched["token"].as_str().unwrap(), old_token);
    assert_eq!(switched["user"]["tenant_code"], "samplefirm");
    assert_eq!(collect_leaves(&switched["modules"]).len(), 100);
    assert!(switched["accessible_tenants"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["tenant_code"] == "samplefirm" && item["current"] == true));

    let writer = login_as(&base_url, "writer01").await;
    let writer_client = authenticated_client(writer["token"].as_str().unwrap());
    let denied = writer_client
        .post(format!("{base_url}/api/auth/switch-tenant"))
        .json(&json!({"tenant_code": "samplefirm"}))
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
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

fn collect_leaves(node: &Value) -> Vec<String> {
    match node["children"].as_array() {
        Some(children) if !children.is_empty() => {
            children.iter().flat_map(collect_leaves).collect()
        }
        _ => vec![node["code"].as_str().unwrap_or_default().to_string()],
    }
}
