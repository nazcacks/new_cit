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
async fn v15_leaf_records_crud_roundtrips_for_crud_capable_leaves() {
    let (base_url, _) = spawn_seeded_app().await;
    let login = login_as(&base_url, "admin").await;
    let client = authenticated_client(login["token"].as_str().unwrap());
    let tree = get_json(&client, &format!("{base_url}/api/modules/tree")).await;
    let crud_leaves: Vec<String> = collect_leaves(&tree)
        .into_iter()
        .filter(|leaf| crud_capable_leaf(leaf))
        .take(60)
        .collect();
    assert_eq!(
        crud_leaves.len(),
        60,
        "v1.5 expects at least 60 CRUD-capable leaves"
    );

    for leaf_key in crud_leaves {
        let created = post_json(
            &client,
            &format!("{base_url}/api/tenants/demo/leaf-records"),
            json!({
                "leaf_key": leaf_key,
                "data": {"title": format!("{leaf_key} row"), "status": "DRAFT"}
            }),
            StatusCode::OK,
        )
        .await;
        let record_id = created["row"]["record_id"].as_i64().expect("record_id");

        let listed = get_json(
            &client,
            &format!(
                "{base_url}/api/tenants/demo/leaf-records?leaf_key={}",
                encode_leaf_key(&leaf_key)
            ),
        )
        .await;
        assert!(listed["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["record_id"] == record_id));

        let updated = patch_json(
            &client,
            &format!("{base_url}/api/tenants/demo/leaf-records/{record_id}"),
            json!({"data": {"title": format!("{leaf_key} row updated"), "status": "ACTIVE"}}),
            StatusCode::OK,
        )
        .await;
        assert_eq!(updated["row"]["status"], "ACTIVE");

        let deleted = delete_json(
            &client,
            &format!("{base_url}/api/tenants/demo/leaf-records/{record_id}"),
            StatusCode::OK,
        )
        .await;
        assert_eq!(deleted["row_id"], record_id);

        let after_delete = get_json(
            &client,
            &format!(
                "{base_url}/api/tenants/demo/leaf-records?leaf_key={}",
                encode_leaf_key(&leaf_key)
            ),
        )
        .await;
        assert!(!after_delete["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["record_id"] == record_id));
    }
}

fn crud_capable_leaf(leaf: &str) -> bool {
    !(leaf.starts_with("dashboard:")
        || leaf.starts_with("report:")
        || leaf.starts_with("admin/audit:")
        || leaf == "ws/file:done")
}

fn encode_leaf_key(leaf: &str) -> String {
    leaf.replace('/', "%2F").replace(':', "%3A")
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

async fn get_json(client: &Client, url: &str) -> Value {
    let response = client.get(url).send().await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    serde_json::from_str(&body).unwrap()
}

async fn post_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.post(url).json(&body).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn patch_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.patch(url).json(&body).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn delete_json(client: &Client, url: &str, expected: StatusCode) -> Value {
    let response = client.delete(url).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}
