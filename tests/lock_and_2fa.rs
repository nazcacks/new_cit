use std::env;

use axum::serve;
use chrono::Utc;
use cit_system::{auth, db, queue, router, AppState, Config};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    multipart::{Form, Part},
    Client, StatusCode,
};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use uuid::Uuid;

#[tokio::test]
async fn filed_lock_login_ip_allowlist_and_lockout_are_enforced() {
    let (base_url, state) = spawn_app().await;
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
        StatusCode::UNAUTHORIZED,
        "{}",
        login_without_otp.1
    );
    assert!(login_without_otp.1["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("2fa otp is required"));
    let otp = current_otp(&state, secret).await;
    let login_with_otp = login_attempt_with_otp(
        &base_url,
        &tenant_code,
        "secure_user",
        "ChangeMe123!",
        "203.0.113.10",
        Some(&otp),
    )
    .await;
    assert_eq!(login_with_otp.0, StatusCode::OK, "{}", login_with_otp.1);
    let secure_token = login_with_otp.1["token"].as_str().unwrap();
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
    assert!(locked.1["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("locked"));

    let unlocked = post_json(
        &admin_client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users/lock_user/status"),
        json!({"status": "ACTIVE", "locked": false}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(unlocked["pwd_fail_count"], 0);
    sqlx::query(
        r#"
        UPDATE users
        SET pwd_changed_at = $1
        WHERE login_id = 'lock_user'
        "#,
    )
    .bind(Utc::now() - chrono::Duration::days(120))
    .execute(&state.pool)
    .await
    .unwrap();
    let expired = login_attempt(
        &base_url,
        &tenant_code,
        "lock_user",
        "ChangeMe123!",
        "203.0.113.10",
    )
    .await;
    assert_eq!(expired.0, StatusCode::UNAUTHORIZED);
    assert!(expired.1["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("password expired"));
    sqlx::query(
        r#"
        UPDATE users
        SET pwd_changed_at = NOW()
        WHERE login_id = 'lock_user'
        "#,
    )
    .execute(&state.pool)
    .await
    .unwrap();
    let relogin = login_attempt(
        &base_url,
        &tenant_code,
        "lock_user",
        "ChangeMe123!",
        "203.0.113.10",
    )
    .await;
    assert_eq!(relogin.0, StatusCode::OK, "{}", relogin.1);
    let lock_user_client =
        authed_client(relogin.1["token"].as_str().unwrap(), Some("203.0.113.10"));
    let forbidden_mapping = post_json(
        &lock_user_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/account-mappings"),
        json!({
            "statement_type": "BS",
            "source_account_code": "10100",
            "source_account_name": "Cash",
            "std_account_code": "STD_CASH"
        }),
        StatusCode::FORBIDDEN,
    )
    .await;
    assert_eq!(forbidden_mapping["error"]["code"], "FORBIDDEN");

    let mapping = post_json(
        &secure_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/account-mappings"),
        json!({
            "statement_type": "BS",
            "source_account_code": "10100",
            "source_account_name": "Cash",
            "std_account_code": "STD_CASH"
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(mapping["std_account_code"], "STD_CASH");
    assert_eq!(mapping["standard_account_code"], "STD_CASH");

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
    let mapping_blocked = post_json(
        &secure_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/account-mappings"),
        json!({
            "statement_type": "BS",
            "source_account_code": "10200",
            "source_account_name": "Accounts receivable",
            "std_account_code": "STD_AR"
        }),
        StatusCode::CONFLICT,
    )
    .await;
    assert_eq!(mapping_blocked["error"]["code"], "CONFLICT");

    let amended = post_json(
        &secure_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/unlock"),
        json!({"reason": "amendment", "actor": "secure_user", "version_mode": "CURRENT"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(amended["lock_mode"], "AMENDMENT_UNLOCK");
    let amended_by_id = amended["by_id"].as_i64().expect("amended by_id");
    assert_ne!(amended_by_id, by_id);
    post_json(
        &secure_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{amended_by_id}/adjustments"),
        json!({"accounting_income": 1000000}),
        StatusCode::OK,
    )
    .await;
}

#[tokio::test]
async fn business_year_carryforward_clones_snapshot() {
    let (base_url, _state) = spawn_app().await;
    let admin_token = login_demo_admin(&base_url).await;
    let client = authed_client(&admin_token, None);

    let tenant_code = format!("cf{}", &Uuid::new_v4().simple().to_string()[..10]);
    post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Carryforward Tenant",
            "biz_reg_no": "1234567890",
            "contract_start": "2026-01-01",
            "contract_end": null,
            "max_users": 10
        }),
        StatusCode::CREATED,
    )
    .await;

    let customer = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/customers"),
        json!({
            "customer_code": "CF001",
            "customer_name": "Carryforward Customer",
            "biz_reg_no": "2208112345",
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"]
        }),
        StatusCode::CREATED,
    )
    .await;
    let customer_id = customer["customer_id"].as_i64().unwrap();

    let source_by = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
        json!({
            "customer_id": customer_id,
            "year_label": 2025,
            "start_date": "2025-01-01",
            "end_date": "2025-12-31"
        }),
        StatusCode::CREATED,
    )
    .await;
    let source_by_id = source_by["by_id"].as_i64().unwrap();
    let source_snapshot = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{source_by_id}/snapshot"),
    )
    .await;

    let target_by = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
        json!({
            "customer_id": customer_id,
            "year_label": 2026,
            "start_date": "2026-01-01",
            "end_date": "2026-12-31",
            "carry_forward_from_by_id": source_by_id
        }),
        StatusCode::CREATED,
    )
    .await;
    let target_by_id = target_by["by_id"].as_i64().unwrap();
    let target_snapshot = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{target_by_id}/snapshot"),
    )
    .await;

    assert_eq!(
        target_snapshot["law_version_id"],
        source_snapshot["law_version_id"]
    );
    assert_eq!(
        target_snapshot["snapshot_data"]["business_year"]["carry_forward_from_by_id"],
        source_by_id
    );
    assert_eq!(
        target_snapshot["snapshot_data"]["carry_forward"]["source_by_id"],
        source_by_id
    );
}

#[tokio::test]
async fn efiling_submit_step_up_role_and_filed_lock_are_enforced() {
    let (base_url, state) = spawn_app().await;
    let admin_token = login_demo_admin(&base_url).await;
    let admin_client = authed_client(&admin_token, None);

    let tenant_code = format!("ef{}", &Uuid::new_v4().simple().to_string()[..10]);
    post_json(
        &admin_client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Efiling Security",
            "biz_reg_no": "1234567890",
            "contract_start": "2026-01-01",
            "contract_end": null,
            "max_users": 10
        }),
        StatusCode::CREATED,
    )
    .await;
    let customer = post_json(
        &admin_client,
        &format!("{base_url}/api/tenants/{tenant_code}/customers"),
        json!({
            "customer_code": "EF001",
            "customer_name": "Efiling Customer",
            "biz_reg_no": "2208112345",
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

    let fs_csv = "\
statement_type,account_code,account_name,debit,credit,standard_account_code,standard_account_name
BS,10100,Cash,1000,0,STD_CASH,Cash
BS,20100,Accounts payable,0,400,STD_PAYABLE,Accounts payable
BS,30100,Capital,0,600,STD_CAPITAL,Capital
IS,40100,Revenue,0,600,STD_PRODUCT_REVENUE,Revenue
IS,50100,Cost of goods sold,400,0,STD_COGS,Cost of goods sold
IS,51100,Salary expense,200,0,STD_SALARY,Salary expense
";
    post_csv_file(
        &admin_client,
        &format!(
            "{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/tax-data/financial-statements/import"
        ),
        "efiling-fs.csv",
        fs_csv,
        StatusCode::CREATED,
    )
    .await;
    post_json(
        &admin_client,
        &format!(
            "{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/std-fs/mappings/bulk"
        ),
        json!({
            "mappings": [
                {"account_code": "10100", "std_fs_item_code": "1010"},
                {"account_code": "20100", "std_fs_item_code": "2010"},
                {"account_code": "30100", "std_fs_item_code": "3010"},
                {"account_code": "40100", "std_fs_item_code": "4010"},
                {"account_code": "50100", "std_fs_item_code": "4510"},
                {"account_code": "51100", "std_fs_item_code": "5110"}
            ]
        }),
        StatusCode::OK,
    )
    .await;
    post_json(
        &admin_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/std-fs/confirm"),
        json!({}),
        StatusCode::OK,
    )
    .await;

    post_json(
        &admin_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/adjustments"),
        json!({"accounting_income": 500000000, "gross_revenue": 1000000000}),
        StatusCode::OK,
    )
    .await;

    for status in ["IN_REVIEW", "APPROVED"] {
        post_json(
            &admin_client,
            &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
            json!({"status": status, "actor": "admin", "approver": "admin"}),
            StatusCode::OK,
        )
        .await;
    }

    let secret = "12345678901234567890";
    post_json(
        &admin_client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users"),
        json!({
            "login_id": "efile_secure",
            "password": "ChangeMe123!",
            "user_name": "Efile Secure",
            "email": "efile-secure@example.test",
            "use_2fa": true,
            "totp_secret": secret,
            "roles": ["TAX_EXPERT"],
            "customer_access": [{
                "customer_id": customer_id,
                "access_level": "OWNER",
                "is_primary": true,
                "work_scopes": ["EFILE", "PRINT"]
            }]
        }),
        StatusCode::CREATED,
    )
    .await;
    post_json(
        &admin_client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users"),
        json!({
            "login_id": "efile_assistant",
            "password": "ChangeMe123!",
            "user_name": "Efile Assistant",
            "email": "efile-assistant@example.test",
            "use_2fa": false,
            "roles": ["ASSISTANT"],
            "customer_access": [{
                "customer_id": customer_id,
                "access_level": "ASSISTANT",
                "is_primary": true,
                "work_scopes": ["EFILE"]
            }]
        }),
        StatusCode::CREATED,
    )
    .await;

    let job = post_json(
        &admin_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/efilings"),
        json!({"max_attempts": 1}),
        StatusCode::ACCEPTED,
    )
    .await;
    run_until_job_status(&state, job["job_id"].as_str().unwrap(), "succeeded").await;
    let histories = get_json(
        &admin_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/efilings"),
    )
    .await;
    let efiling_id = histories[0]["efiling_id"].as_i64().unwrap();
    let submit_url = format!(
        "{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/efilings/{efiling_id}/submit"
    );

    let secure_otp = current_otp(&state, secret).await;
    let secure_login = login_attempt_with_otp(
        &base_url,
        &tenant_code,
        "efile_secure",
        "ChangeMe123!",
        "127.0.0.1",
        Some(&secure_otp),
    )
    .await;
    assert_eq!(secure_login.0, StatusCode::OK, "{}", secure_login.1);
    let secure_client = authed_client(secure_login.1["token"].as_str().unwrap(), None);
    let missing_otp = post_json(
        &secure_client,
        &submit_url,
        json!({"actor": "efile_secure"}),
        StatusCode::FORBIDDEN,
    )
    .await;
    assert!(missing_otp["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("2fa otp is required"));

    let assistant_token = login_attempt(
        &base_url,
        &tenant_code,
        "efile_assistant",
        "ChangeMe123!",
        "127.0.0.1",
    )
    .await;
    assert_eq!(assistant_token.0, StatusCode::OK, "{}", assistant_token.1);
    let assistant_client = authed_client(assistant_token.1["token"].as_str().unwrap(), None);
    post_json(
        &assistant_client,
        &submit_url,
        json!({"actor": "efile_assistant"}),
        StatusCode::FORBIDDEN,
    )
    .await;

    let submitted = post_json(
        &secure_client,
        &submit_url,
        json!({"actor": "efile_secure", "otp": current_otp(&state, secret).await}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(submitted["status"], "ACCEPTED");
    assert!(submitted["receipt_no"]
        .as_str()
        .unwrap_or_default()
        .starts_with("R-"));
    let resubmit_blocked = post_json(
        &secure_client,
        &submit_url,
        json!({"actor": "efile_secure", "otp": current_otp(&state, secret).await}),
        StatusCode::CONFLICT,
    )
    .await;
    assert_eq!(resubmit_blocked["error"]["code"], "CONFLICT");

    let workflow = get_json(
        &admin_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/workflow"),
    )
    .await;
    assert_eq!(workflow["business_year"]["status"], "FILED");
    assert!(!workflow["business_year"]["locked_at"].is_null());
    let blocked = post_json(
        &admin_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/adjustments"),
        json!({"accounting_income": 1000000}),
        StatusCode::CONFLICT,
    )
    .await;
    assert_eq!(blocked["error"]["code"], "CONFLICT");
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
    login_attempt_with_otp(base_url, tenant_code, login_id, password, ip, None).await
}

async fn login_attempt_with_otp(
    base_url: &str,
    tenant_code: &str,
    login_id: &str,
    password: &str,
    ip: &str,
    otp: Option<&str>,
) -> (StatusCode, Value) {
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", HeaderValue::from_str(ip).unwrap());
    let client = Client::builder().default_headers(headers).build().unwrap();
    let response = client
        .post(format!("{base_url}/api/auth/login"))
        .json(&json!({
            "tenant_code": tenant_code,
            "login_id": login_id,
            "password": password,
            "otp": otp
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

async fn current_otp(state: &AppState, secret: &str) -> String {
    let counter = Utc::now().timestamp() / 30;
    auth::hotp(&state.pool, secret.as_bytes(), counter)
        .await
        .unwrap()
}

async fn run_until_job_status(state: &AppState, job_id: &str, expected: &str) -> Value {
    let id = job_id.parse::<Uuid>().unwrap();
    for _ in 0..50 {
        queue::run_once(state.clone()).await.unwrap();
        let job = queue::get_job(&state.pool, id).await.unwrap();
        if job.status == expected {
            return serde_json::to_value(job).unwrap();
        }
    }
    let job = queue::get_job(&state.pool, id).await.unwrap();
    panic!(
        "job {job_id} did not reach {expected}; current status={} last_error={:?}",
        job.status, job.last_error
    );
}

async fn post_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.post(url).json(&body).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn post_csv_file(
    client: &Client,
    url: &str,
    file_name: &str,
    csv: &str,
    expected: StatusCode,
) -> Value {
    let form = Form::new().part(
        "file",
        Part::text(csv.to_string())
            .file_name(file_name.to_string())
            .mime_str("text/csv")
            .unwrap(),
    );
    let response = client.post(url).multipart(form).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn get_json(client: &Client, url: &str) -> Value {
    let response = client.get(url).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{text}");
    serde_json::from_str(&text).unwrap()
}
