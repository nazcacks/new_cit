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
async fn filed_lock_login_ip_allowlist_and_lockout_are_enforced() {
    let (base_url, _state) = spawn_app().await;
    let admin_token = login_demo_admin(&base_url).await;
    let admin_client = authed_client(&admin_token, None);

    let tenant_code = format!("sec{}", &Uuid::new_v4().simple().to_string()[..10]);
    post_json(
        &admin_client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Security Test",
            "biz_reg_no": "1234567890",
            "contract_start": "2026-01-01",
            "contract_end": null,
            "allowed_ips": "203.0.113.10/32",
            "max_users": 10
        }),
        StatusCode::CREATED,
    )
    .await;

    let customer = post_json(
        &admin_client,
        &format!("{base_url}/api/tenants/{tenant_code}/customers"),
        json!({
            "customer_code": "SEC001",
            "customer_name": "Security Customer",
            "biz_reg_no": "2208112345",
            "corp_reg_no": null,
            "industry_code": "62010",
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"]
        }),
        StatusCode::CREATED,
    )
    .await;
    let customer_id = customer["customer_id"].as_i64().unwrap();
    let by = post_json(
        &admin_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
        json!({
            "customer_id": customer_id,
            "year_label": 2026,
            "start_date": "2026-01-01",
            "end_date": "2026-12-31"
        }),
        StatusCode::CREATED,
    )
    .await;
    let by_id = by["by_id"].as_i64().unwrap();

    let secret = "12345678901234567890";
    post_json(
        &admin_client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users"),
        json!({
            "login_id": "secure_user",
            "password": "ChangeMe123!",
            "user_name": "Secure User",
            "email": "secure@example.test",
            "phone": null,
            "use_2fa": true,
            "totp_secret": secret,
            "roles": ["TENANT_ADMIN"],
            "customer_access": [{
                "customer_id": customer_id,
                "access_level": "OWNER",
                "is_primary": true,
                "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"]
            }]
        }),
        StatusCode::CREATED,
    )
    .await;

    let bad_ip = login_attempt(
        &base_url,
        &tenant_code,
        "secure_user",
        "ChangeMe123!",
        "198.51.100.99",
    )
    .await;
    assert_eq!(bad_ip.0, StatusCode::UNAUTHORIZED);

    let login_without_otp = login_attempt(
        &base_url,
        &tenant_code,
        "secure_user",
        "ChangeMe123!",
        "203.0.113.10",
    )
    .await;
    assert_eq!(
        login_without_otp.0,
        StatusCode::OK,
        "{}",
        login_without_otp.1
    );
    let secure_token = login_without_otp.1["token"].as_str().unwrap();
    let secure_client = authed_client(secure_token, Some("203.0.113.10"));

    post_json(
        &admin_client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users"),
        json!({
            "login_id": "lock_user",
            "password": "ChangeMe123!",
            "user_name": "Lock User",
            "email": "lock@example.test",
            "use_2fa": false,
            "roles": ["ASSISTANT"]
        }),
        StatusCode::CREATED,
    )
    .await;
    for _ in 0..5 {
        let failed = login_attempt(
            &base_url,
            &tenant_code,
            "lock_user",
            "WrongPassword!",
            "203.0.113.10",
        )
        .await;
        assert_eq!(failed.0, StatusCode::UNAUTHORIZED);
    }
    let locked = login_attempt(
        &base_url,
        &tenant_code,
        "lock_user",
        "ChangeMe123!",
        "203.0.113.10",
    )
    .await;
    assert_eq!(locked.0, StatusCode::UNAUTHORIZED);
    let unlocked = post_json(
        &admin_client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users/lock_user/status"),
        json!({"status": "ACTIVE", "locked": false}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(unlocked["pwd_fail_count"], 0);
    let relogin = login_attempt(
        &base_url,
        &tenant_code,
        "lock_user",
        "ChangeMe123!",
        "203.0.113.10",
    )
    .await;
    assert_eq!(relogin.0, StatusCode::OK, "{}", relogin.1);

    for status in ["IN_REVIEW", "APPROVED", "FILED"] {
        post_json(
            &secure_client,
            &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
            json!({"status": status, "actor": "secure_user", "approver": "secure_user"}),
            StatusCode::OK,
        )
        .await;
    }
    let blocked = post_json(
        &secure_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/adjustments"),
        json!({"accounting_income": 1000000}),
        StatusCode::CONFLICT,
    )
    .await;
    assert_eq!(blocked["error"]["code"], "CONFLICT");

    let amended = post_json(
        &secure_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/unlock"),
        json!({"reason": "amendment", "actor": "secure_user", "version_mode": "CURRENT"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(amended["lock_mode"], "AMENDMENT_UNLOCK");
    post_json(
        &secure_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/adjustments"),
        json!({"accounting_income": 1000000}),
        StatusCode::OK,
    )
    .await;
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

async fn login_demo_admin(base_url: &str) -> String {
    let auth = login_attempt(base_url, "demo", "admin", "ChangeMe123!", "127.0.0.1").await;
    assert_eq!(auth.0, StatusCode::OK, "{}", auth.1);
    auth.1["token"].as_str().unwrap().to_string()
}

async fn login_attempt(
    base_url: &str,
    tenant_code: &str,
    login_id: &str,
    password: &str,
    ip: &str,
) -> (StatusCode, Value) {
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", HeaderValue::from_str(ip).unwrap());
    let client = Client::builder().default_headers(headers).build().unwrap();
    let response = client
        .post(format!("{base_url}/api/auth/login"))
        .json(&json!({
            "tenant_code": tenant_code,
            "login_id": login_id,
            "password": password
        }))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    let body = serde_json::from_str(&text).unwrap_or_else(|_| json!({"raw": text}));
    (status, body)
}

fn authed_client(token: &str, ip: Option<&str>) -> Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    if let Some(ip) = ip {
        headers.insert("x-forwarded-for", HeaderValue::from_str(ip).unwrap());
    }
    Client::builder().default_headers(headers).build().unwrap()
}

async fn post_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.post(url).json(&body).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}
