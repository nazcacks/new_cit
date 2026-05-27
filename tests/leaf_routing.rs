use std::env;

use axum::serve;
use cit_system::{
    db, router,
    seed::{self, DemoSeedOptions},
    AppState, Config,
};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client, Method, StatusCode,
};
use serde_json::{json, Value};
use tokio::net::TcpListener;

#[tokio::test]
async fn v13_leaf_routes_and_new_backend_routes_are_reachable() {
    let (base_url, seed_result) = spawn_seeded_app().await;
    let token = login(&base_url).await;
    let client = authenticated_client(&token);
    let tenant = "demo";
    let by_id = seed_result.main_by_id;

    let tree = get_json(&client, &format!("{base_url}/api/modules/tree")).await;
    let leaves = collect_leaves(&tree);
    assert_eq!(leaves.len(), 100, "v1.4 must expose the active 100 leaf IA");

    let routes = v13_routes(&base_url, tenant, by_id);
    assert!(
        routes.len() >= 25,
        "v1.3 must register at least 25 new backend route cases"
    );
    for case in routes {
        let response = match case.method {
            Method::GET => client.get(&case.url).send().await.unwrap(),
            Method::POST => client
                .post(&case.url)
                .json(&json!({}))
                .send()
                .await
                .unwrap(),
            Method::PUT => client.put(&case.url).json(&json!({})).send().await.unwrap(),
            Method::PATCH => {
                let payload = if case.url.ends_with("/status") {
                    json!({"status": "ACTIVE"})
                } else if case.url.ends_with("/plan") {
                    json!({"plan": "STANDARD"})
                } else {
                    json!({})
                };
                client.patch(&case.url).json(&payload).send().await.unwrap()
            }
            _ => unreachable!("unsupported method"),
        };
        let status = response.status();
        let body = response.text().await.unwrap();
        let gated_action = case.url.ends_with("/submit");
        assert!(
            status == StatusCode::OK || (gated_action && status == StatusCode::FORBIDDEN),
            "{} {} failed: {}",
            case.method,
            case.url,
            body
        );
        assert!(
            serde_json::from_str::<Value>(&body).is_ok(),
            "{} {} did not return JSON: {}",
            case.method,
            case.url,
            body
        );
    }
}

struct RouteCase {
    method: Method,
    url: String,
}

fn v13_routes(base_url: &str, tenant: &str, by_id: i64) -> Vec<RouteCase> {
    let get = |path: String| RouteCase {
        method: Method::GET,
        url: path,
    };
    let post = |path: String| RouteCase {
        method: Method::POST,
        url: path,
    };
    let put = |path: String| RouteCase {
        method: Method::PUT,
        url: path,
    };
    let patch = |path: String| RouteCase {
        method: Method::PATCH,
        url: path,
    };
    vec![
        get(format!("{base_url}/api/tenants")),
        patch(format!("{base_url}/api/tenants/{tenant}/status")),
        patch(format!("{base_url}/api/tenants/{tenant}/plan")),
        get(format!(
            "{base_url}/api/tenants/{tenant}/business-years/{by_id}/progress"
        )),
        get(format!(
            "{base_url}/api/tenants/{tenant}/reports/industry-stats"
        )),
        post(format!("{base_url}/api/tenants/{tenant}/reports/custom")),
        get(format!("{base_url}/api/tenants/{tenant}/reports/custom/1")),
        post(format!(
            "{base_url}/api/tenants/{tenant}/business-years/{by_id}/resubmit"
        )),
        get(format!(
            "{base_url}/api/tenants/{tenant}/business-years/{by_id}/amendment-version-mode"
        )),
        get(format!("{base_url}/api/tenants/{tenant}/correction-claims")),
        post(format!("{base_url}/api/tenants/{tenant}/correction-claims")),
        post(format!(
            "{base_url}/api/tenants/{tenant}/business-years/{by_id}/workflow/request"
        )),
        get(format!(
            "{base_url}/api/tenants/{tenant}/workflow/events?status=REJECTED"
        )),
        get(format!(
            "{base_url}/api/tenants/{tenant}/business-years/{by_id}/print/history"
        )),
        post(format!(
            "{base_url}/api/tenants/{tenant}/business-years/{by_id}/efilings/1/submit"
        )),
        get(format!("{base_url}/api/form-versioning/versions/1/fields")),
        put(format!("{base_url}/api/form-versioning/versions/1/fields")),
        get(format!(
            "{base_url}/api/form-versioning/versions/1/validations"
        )),
        put(format!(
            "{base_url}/api/form-versioning/versions/1/validations"
        )),
        get(format!("{base_url}/api/form-versioning/field-references")),
        get(format!("{base_url}/api/form-versioning/efile-map")),
        get(format!("{base_url}/api/form-versioning/by-set")),
        post(format!("{base_url}/api/form-versioning/impact")),
        get(format!("{base_url}/api/admin/functions")),
        get(format!("{base_url}/api/admin/field-masking")),
        put(format!("{base_url}/api/admin/field-masking")),
        get(format!("{base_url}/api/admin/data-scope")),
        put(format!("{base_url}/api/admin/data-scope")),
        get(format!("{base_url}/api/admin/customer-groups")),
        post(format!("{base_url}/api/admin/customer-groups")),
        get(format!("{base_url}/api/admin/customer-rules")),
        post(format!("{base_url}/api/admin/customer-rules")),
        get(format!("{base_url}/api/admin/access-delegations")),
        post(format!("{base_url}/api/admin/access-delegations")),
        get(format!("{base_url}/api/admin/customer-access/override")),
        post(format!("{base_url}/api/admin/customer-access/override")),
        get(format!("{base_url}/api/login-history")),
        get(format!("{base_url}/api/permission-change-history")),
        get(format!("{base_url}/api/system-settings")),
        get(format!("{base_url}/api/tenants/{tenant}/tax-agents")),
        post(format!("{base_url}/api/tenants/{tenant}/tax-agents")),
        get(format!("{base_url}/api/tenants/{tenant}/codes?group=ALL")),
        get(format!(
            "{base_url}/api/tenants/{tenant}/business-years/{by_id}/forms/linkage-check"
        )),
        get(format!(
            "{base_url}/api/tenants/{tenant}/business-years/{by_id}/validation/issues"
        )),
        get(format!(
            "{base_url}/api/tenants/{tenant}/business-years/{by_id}/efilings/latest"
        )),
        post(format!("{base_url}/api/tenants/{tenant}/leaf-actions")),
    ]
}

fn collect_leaves(node: &Value) -> Vec<String> {
    match node["children"].as_array() {
        Some(children) if !children.is_empty() => {
            children.iter().flat_map(collect_leaves).collect()
        }
        _ => vec![node["code"].as_str().unwrap_or_default().to_string()],
    }
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

async fn login(base_url: &str) -> String {
    let client = Client::new();
    let response = client
        .post(format!("{base_url}/api/auth/login"))
        .json(&json!({"tenant_code": "demo", "login_id": "admin", "password": "ChangeMe123!"}))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    serde_json::from_str::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string()
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
