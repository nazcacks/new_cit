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
async fn v15_all_leaf_screens_have_five_block_workbench_contract() {
    let (base_url, _) = spawn_seeded_app().await;
    let login = login_as(&base_url, "admin").await;
    let client = authenticated_client(login["token"].as_str().unwrap());

    let tree = get_json(&client, &format!("{base_url}/api/modules/tree")).await;
    let leaves = collect_leaves(&tree);
    assert_eq!(leaves.len(), 100);

    let screens = client
        .get(format!("{base_url}/app/screens.js"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    for leaf in &leaves {
        assert!(
            screens.contains(&format!("\"{leaf}\":")),
            "{leaf} missing screen registration"
        );
    }

    for block in ["summary", "filters", "table", "row-actions", "toolbar"] {
        assert!(
            screens.contains(&format!("data-leaf-block=\"{block}\"")),
            "{block} block missing"
        );
    }
    assert!(screens.contains("data-leaf-create"));
    assert!(screens.contains("data-leaf-row-action=\"edit\""));
    assert!(screens.contains("data-leaf-row-action=\"delete\""));
    assert!(screens.contains("/leaf-records"));
    assert!(!screens.contains("기능 실행"));
    assert!(!screens.contains("data-leaf-action"));

    let menu = client
        .get(format!("{base_url}/app/menu.js"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(menu.contains("menu-progress-dot"));
    assert!(menu.contains("groupProgress("));
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
