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
use uuid::Uuid;

#[tokio::test]
async fn admin_tenant_leaf_and_super_admin_crud_routes_work() {
    let (base_url, _) = spawn_seeded_app().await;
    let admin = login_as(&base_url, "admin").await;
    let admin_client = authenticated_client(admin["token"].as_str().unwrap());

    let tree = get_json(&admin_client, &format!("{base_url}/api/modules/tree")).await;
    assert_eq!(collect_leaves(&tree).len(), 100);
    assert!(find_node(&tree, "admin/tenant").is_some());
    assert!(find_node(&tree, "admin/tenant:list").is_some());

    let tenants = get_json(&admin_client, &format!("{base_url}/api/tenants")).await;
    assert!(tenants.as_array().unwrap().len() >= 2);

    let tenant_code = format!("ta{}", &Uuid::new_v4().simple().to_string()[..10]);
    let created = admin_client
        .post(format!("{base_url}/api/tenants"))
        .json(&json!({
            "tenant_code": tenant_code,
            "tenant_name": "Tenant Admin Routing",
            "biz_reg_no": "9988112345",
            "contract_start": "2026-01-01",
            "contract_end": null,
            "allowed_ips": null,
            "max_users": 7,
            "plan": "PRO"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = created.json::<Value>().await.unwrap();
    assert_eq!(created["plan"], "PRO");

    let suspended = admin_client
        .patch(format!("{base_url}/api/tenants/{tenant_code}/status"))
        .json(&json!({"status": "SUSPENDED"}))
        .send()
        .await
        .unwrap();
    assert_eq!(suspended.status(), StatusCode::OK);
    let suspended = suspended.json::<Value>().await.unwrap();
    assert_eq!(suspended["status"], "SUSPENDED");

    let upgraded = admin_client
        .patch(format!("{base_url}/api/tenants/{tenant_code}/plan"))
        .json(&json!({"plan": "ENTERPRISE"}))
        .send()
        .await
        .unwrap();
    assert_eq!(upgraded.status(), StatusCode::OK);
    let upgraded = upgraded.json::<Value>().await.unwrap();
    assert_eq!(upgraded["plan"], "ENTERPRISE");

    let writer = login_as(&base_url, "writer01").await;
    let writer_client = authenticated_client(writer["token"].as_str().unwrap());
    let denied = writer_client
        .get(format!("{base_url}/api/tenants"))
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

async fn get_json(client: &Client, url: &str) -> Value {
    let response = client.get(url).send().await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    serde_json::from_str(&body).unwrap()
}

fn collect_leaves(node: &Value) -> Vec<String> {
    match node["children"].as_array() {
        Some(children) if !children.is_empty() => {
            children.iter().flat_map(collect_leaves).collect()
        }
        _ => vec![node["code"].as_str().unwrap_or_default().to_string()],
    }
}

fn find_node<'a>(node: &'a Value, key: &str) -> Option<&'a Value> {
    if node["code"] == key || node["key"] == key {
        return Some(node);
    }
    node["children"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|child| find_node(child, key))
}
