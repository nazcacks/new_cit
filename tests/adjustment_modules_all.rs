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
async fn all_seventeen_adjustment_modules_have_items_and_history() {
    let (base_url, by_id) = spawn_seeded_app().await;
    let token = login(&base_url).await;
    let client = authed_client(&token);
    let tenant = "demo";
    let endpoints = [
        "adjustments/income",
        "adjustments/transactions/B2",
        "adjustments/transactions/B3",
        "adjustments/assets/B4",
        "adjustments/assets/B5",
        "adjustments/assets/B6",
        "adjustments/evaluation/B7",
        "adjustments/evaluation/B8",
        "adjustments/transactions/B9",
        "adjustments/assets/B10",
        "adjustments/evaluation/B11",
        "adjustments/tax/B12",
        "adjustments/tax/B13",
        "adjustments/tax/B14",
        "adjustments/evaluation/B15",
        "adjustments/special/B16",
        "adjustments/special/B17",
    ];
    for endpoint in endpoints {
        let value = get_json(
            &client,
            &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/{endpoint}"),
        )
        .await;
        assert!(
            !value.as_array().unwrap().is_empty(),
            "{endpoint} should expose item rows"
        );
    }
    let history = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/adjustments/history"),
    )
    .await;
    assert!(
        !history.as_array().unwrap().is_empty(),
        "adjustment history should be populated by calculations"
    );
}

async fn spawn_seeded_app() -> (String, i64) {
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
    (format!("http://{addr}"), seed_result.main_by_id)
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
