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
async fn demo_seed_makes_all_prototype_menus_non_empty() {
    let (base_url, seed_result) = spawn_seeded_app().await;
    let token = login(&base_url).await;
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    let client = Client::builder().default_headers(headers).build().unwrap();
    let tenant = "demo";
    let by_id = seed_result.main_by_id;
    let filed_by_id = seed_result.filed_by_id;

    let dashboard = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/dashboard"),
    )
    .await;
    assert_positive(&dashboard, "customer_count");
    assert_positive(&dashboard, "business_year_count");
    assert_positive(&dashboard, "filed_count");
    assert_positive(&dashboard, "pending_review_count");
    assert_positive(&dashboard, "due_soon_count");

    let customers = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/customers"),
    )
    .await;
    assert_min_len(&customers, 3, "customers");
    let business_years = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years"),
    )
    .await;
    assert_min_len(&business_years, 5, "business years");

    let tax_validation = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/tax-data/validation"),
    )
    .await;
    assert_eq!(tax_validation["balanced"], true);
    assert!(tax_validation["fs_line_count"].as_i64().unwrap_or_default() >= 30);
    assert!(tax_validation["asset_count"].as_i64().unwrap_or_default() >= 8);
    assert!(
        tax_validation["business_vehicle_count"]
            .as_i64()
            .unwrap_or_default()
            >= 3
    );
    assert!(
        tax_validation["transaction_count"]
            .as_i64()
            .unwrap_or_default()
            >= 10
    );

    for (path, minimum, label) in [
        ("tax-data/financial-statements", 30, "financial statements"),
        ("tax-data/assets", 8, "assets"),
        ("tax-data/transactions", 10, "transactions"),
        ("vehicle-usage-logs", 3, "vehicle usage logs"),
        ("tax-data/import-batches", 3, "import batches"),
    ] {
        let value = get_json(
            &client,
            &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/{path}"),
        )
        .await;
        assert_min_len(&value, minimum, label);
    }

    let adjustments = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/adjustments"),
    )
    .await;
    assert_min_len(&adjustments, 8, "adjustments");
    let reserves = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/reserves"),
    )
    .await;
    assert_min_len(&reserves, 3, "reserves");
    assert_adjustment_modules(&client, &base_url, tenant, by_id).await;

    let attachments = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/forms/attachments"),
    )
    .await;
    assert_min_len(&attachments, 15, "form attachments");
    assert!(attachments
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["generated"] == true));
    let preview = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/forms/FORM3/preview"),
    )
    .await;
    assert_min_len(&preview["fields"], 4, "FORM3 preview fields");

    let rules = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/validation/rules"),
    )
    .await;
    assert_min_len(&rules, 50, "validation rules");
    let validation = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/validation/run"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(validation["error_count"], 0);
    assert_min_len(&validation["issues"], 1, "validation issues");

    let queue = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/workflow/queue?assignee=me"),
    )
    .await;
    assert_min_len(&queue, 1, "workflow queue");
    let workflow = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/workflow"),
    )
    .await;
    assert_min_len(&workflow["events"], 1, "workflow events");
    post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/status"),
        json!({"status": "APPROVED", "actor": "reviewer01", "approver": "reviewer01", "comment": "menu smoke approval"}),
        StatusCode::OK,
    )
    .await;

    let bundle = get_bytes(
        &client,
        &format!(
            "{base_url}/api/tenants/{tenant}/business-years/{by_id}/forms/pdf-bundle/download"
        ),
    )
    .await;
    assert!(bundle.len() > 1000, "PDF bundle should not be empty");

    let format_spec = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/efilings/format-spec"),
    )
    .await;
    assert_min_len(&format_spec, 10, "e-filing format spec");
    let precheck = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/efilings/precheck"),
    )
    .await;
    assert_eq!(precheck["valid"], true);
    post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/efilings"),
        json!({"max_attempts": 1}),
        StatusCode::ACCEPTED,
    )
    .await;
    let efilings = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/efilings"),
    )
    .await;
    assert_min_len(&efilings, 1, "e-filing history");
    let efiling_id = efilings.as_array().unwrap()[0]["efiling_id"]
        .as_i64()
        .unwrap();
    let efile = get_bytes(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/efilings/{efiling_id}/file"),
    )
    .await;
    assert!(efile.len() > 100, "e-filing file should not be empty");

    let filed_history = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/efilings"),
    )
    .await;
    assert_min_len(&filed_history, 1, "post history e-filings");
    let amendment = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/business-years/{filed_by_id}/amendment-preview"),
    )
    .await;
    assert_min_len(&amendment["differences"], 1, "amendment differences");

    let notifications = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/notifications"),
    )
    .await;
    assert_min_len(&notifications, 4, "notifications");
    let notification_id = notifications.as_array().unwrap()[0]["notification_id"]
        .as_i64()
        .unwrap();
    patch_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant}/notifications/{notification_id}"),
        json!({"status": "READ"}),
        StatusCode::OK,
    )
    .await;
    assert_report_data(&client, &base_url, tenant).await;
    assert_admin_data(&client, &base_url, tenant).await;
}

async fn assert_adjustment_modules(client: &Client, base_url: &str, tenant: &str, by_id: i64) {
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
            client,
            &format!("{base_url}/api/tenants/{tenant}/business-years/{by_id}/{endpoint}"),
        )
        .await;
        assert_min_len(&value, 1, endpoint);
    }
}

async fn assert_report_data(client: &Client, base_url: &str, tenant: &str) {
    let compare = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant}/reports/year-comparison"),
    )
    .await;
    assert_min_len(&compare, 5, "year comparison report");
    assert!(compare
        .as_array()
        .unwrap()
        .iter()
        .any(|row| { row["total_adjustment_amount"].as_i64().unwrap_or_default() > 0 }));
    let burden = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant}/reports/tax-burden"),
    )
    .await;
    assert_min_len(&burden, 5, "tax burden report");
    assert!(burden
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["total_tax_due"].as_i64().unwrap_or_default() > 0));
    let reserve = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant}/reports/reserve-trend"),
    )
    .await;
    assert_min_len(&reserve, 3, "reserve trend report");
}

async fn assert_admin_data(client: &Client, base_url: &str, tenant: &str) {
    assert_min_len(
        &get_json(client, &format!("{base_url}/api/tenants")).await,
        2,
        "tenants",
    );
    assert_min_len(
        &get_json(
            client,
            &format!("{base_url}/api/admin/tenants/{tenant}/users"),
        )
        .await,
        4,
        "admin users",
    );
    assert_min_len(
        &get_json(client, &format!("{base_url}/api/admin/roles")).await,
        5,
        "roles",
    );
    assert_min_len(
        &get_json(client, &format!("{base_url}/api/admin/role-permissions")).await,
        10,
        "role permissions",
    );
    assert_min_len(
        &get_json(client, &format!("{base_url}/api/admin/menus")).await,
        99,
        "admin menu nodes",
    );
    assert_min_len(
        &get_json(
            client,
            &format!("{base_url}/api/tenants/{tenant}/audit-logs"),
        )
        .await,
        5,
        "audit logs",
    );
    assert_min_len(
        &get_json(client, &format!("{base_url}/api/tax-laws")).await,
        6,
        "tax laws",
    );
    let law_summary = get_json(client, &format!("{base_url}/api/law-versioning/summary")).await;
    assert_positive(&law_summary, "laws");
    assert_min_len(
        &get_json(client, &format!("{base_url}/api/tax-rates")).await,
        20,
        "tax rates",
    );
    assert_min_len(
        &get_json(client, &format!("{base_url}/api/tax-limits")).await,
        2,
        "tax limits",
    );
    assert_min_len(
        &get_json(client, &format!("{base_url}/api/form-versioning/forms")).await,
        5,
        "tax forms",
    );
    assert_min_len(
        &get_json(client, &format!("{base_url}/api/form-versioning/versions")).await,
        5,
        "form versions",
    );
    assert_min_len(
        &get_json(
            client,
            &format!("{base_url}/api/form-versioning/relationships"),
        )
        .await,
        2,
        "form relationships",
    );
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
    let auth = post_json(
        &client,
        &format!("{base_url}/api/auth/login"),
        json!({"tenant_code": "demo", "login_id": "admin", "password": "ChangeMe123!"}),
        StatusCode::OK,
    )
    .await;
    auth["token"].as_str().unwrap().to_string()
}

async fn get_json(client: &Client, url: &str) -> Value {
    let response = client.get(url).send().await.unwrap();
    response_json(response, StatusCode::OK).await
}

async fn post_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.post(url).json(&body).send().await.unwrap();
    response_json(response, expected).await
}

async fn patch_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.patch(url).json(&body).send().await.unwrap();
    response_json(response, expected).await
}

async fn response_json(response: reqwest::Response, expected: StatusCode) -> Value {
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn get_bytes(client: &Client, url: &str) -> Vec<u8> {
    let response = client.get(url).send().await.unwrap();
    let status = response.status();
    let bytes = response.bytes().await.unwrap();
    assert_eq!(status, StatusCode::OK, "binary endpoint failed");
    bytes.to_vec()
}

fn assert_min_len(value: &Value, minimum: usize, label: &str) {
    let len = value
        .as_array()
        .unwrap_or_else(|| panic!("{label} is not an array"))
        .len();
    assert!(
        len >= minimum,
        "{label} length {len} is less than {minimum}"
    );
}

fn assert_positive(value: &Value, key: &str) {
    assert!(
        value[key].as_i64().unwrap_or_default() > 0,
        "{key} should be positive in {value}"
    );
}
