use std::env;

use axum::serve;
use cit_system::{db, permissions, router, AppState, Config};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client, StatusCode,
};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use uuid::Uuid;

#[tokio::test]
async fn deny_precedence_scope_masking_delegation_and_audit_verification_work() {
    let (base_url, state) = spawn_app().await;
    let admin_token = login(&base_url, "demo", "admin").await;
    let admin_client = authed_client(&admin_token);

    let tenant_code = format!("perm{}", &Uuid::new_v4().simple().to_string()[..9]);
    post_json(
        &admin_client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Permission Test",
            "biz_reg_no": "1234567890",
            "contract_start": "2026-01-01",
            "contract_end": null,
            "max_users": 10
        }),
        StatusCode::CREATED,
    )
    .await;
    let first = create_customer(&admin_client, &base_url, &tenant_code, "P001").await;
    let second = create_customer(&admin_client, &base_url, &tenant_code, "P002").await;
    let first_id = first["customer_id"].as_i64().unwrap();
    let second_id = second["customer_id"].as_i64().unwrap();

    post_json(
        &admin_client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users"),
        json!({
            "login_id": "scoped_user",
            "password": "ChangeMe123!",
            "user_name": "Scoped User",
            "email": "scoped@example.test",
            "use_2fa": false,
            "roles": ["ASSISTANT"],
            "customer_access": [{
                "customer_id": first_id,
                "access_level": "ASSISTANT",
                "is_primary": true,
                "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "PRINT", "POST"]
            }]
        }),
        StatusCode::CREATED,
    )
    .await;

    sqlx::query(
        r#"
        INSERT INTO roles (role_code, role_name, description, system_role)
        VALUES ('MASK_ALLOW_TEST', 'Mask Allow Test', 'test role', FALSE)
        ON CONFLICT (role_code) DO NOTHING
        "#,
    )
    .execute(&state.pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO role_permissions (role_code, module_code, function_code, effect)
        VALUES ('MASK_ALLOW_TEST', 'customers', 'MASK_OFF', 'ALLOW')
        ON CONFLICT (role_code, module_code, function_code)
        DO UPDATE SET effect = 'ALLOW'
        "#,
    )
    .execute(&state.pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO user_roles (user_id, role_code, granted_by)
        SELECT u.user_id, 'MASK_ALLOW_TEST', 'test'
        FROM users u
        JOIN tenants t ON t.tenant_id = u.tenant_id
        WHERE t.tenant_code = $1 AND u.login_id = 'scoped_user'
        ON CONFLICT (user_id, role_code) DO NOTHING
        "#,
    )
    .bind(&tenant_code)
    .execute(&state.pool)
    .await
    .unwrap();

    let user_id = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT u.user_id
        FROM users u
        JOIN tenants t ON t.tenant_id = u.tenant_id
        WHERE t.tenant_code = $1 AND u.login_id = 'scoped_user'
        "#,
    )
    .bind(&tenant_code)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    let decision = permissions::evaluate_permission(&state.pool, user_id, "customers", "MASK_OFF")
        .await
        .unwrap();
    assert!(decision.denied);
    assert!(!decision.allowed, "DENY must override ALLOW");

    let scoped_token = login(&base_url, &tenant_code, "scoped_user").await;
    let scoped_client = authed_client(&scoped_token);
    let customers = get_json(
        &scoped_client,
        &format!("{base_url}/api/tenants/{tenant_code}/customers"),
    )
    .await;
    let customers = customers.as_array().unwrap();
    assert_eq!(customers.len(), 1);
    assert_eq!(customers[0]["customer_id"], first_id);
    assert!(customers[0]["biz_reg_no"]
        .as_str()
        .unwrap()
        .starts_with("***"));

    let created_by_scoped = post_json(
        &scoped_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
        json!({
            "customer_id": first_id,
            "year_label": 2031,
            "start_date": "2031-01-01",
            "end_date": "2031-12-31",
            "carry_forward_from_by_id": null
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(created_by_scoped["customer_id"], first_id);

    post_json(
        &scoped_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
        json!({
            "customer_id": second_id,
            "year_label": 2031,
            "start_date": "2031-01-01",
            "end_date": "2031-12-31",
            "carry_forward_from_by_id": null
        }),
        StatusCode::FORBIDDEN,
    )
    .await;

    post_json(
        &admin_client,
        &format!("{base_url}/api/tenants/{tenant_code}/access-delegations"),
        json!({
            "grantor_login_id": "scoped_user",
            "delegatee_login_id": "scoped_user",
            "customer_id": second_id,
            "work_scope": "INFO",
            "valid_from": "2026-01-01",
            "valid_to": null,
            "reason": "temporary review"
        }),
        StatusCode::CREATED,
    )
    .await;
    let delegated = get_json(
        &scoped_client,
        &format!("{base_url}/api/tenants/{tenant_code}/customers"),
    )
    .await;
    assert_eq!(delegated.as_array().unwrap().len(), 2);

    post_json(
        &scoped_client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
        json!({
            "customer_id": second_id,
            "year_label": 2032,
            "start_date": "2032-01-01",
            "end_date": "2032-12-31",
            "carry_forward_from_by_id": null
        }),
        StatusCode::FORBIDDEN,
    )
    .await;

    let function_codes = get_json(
        &admin_client,
        &format!("{base_url}/api/admin/function-codes"),
    )
    .await;
    assert!(function_codes.as_array().unwrap().len() >= 12);
    let audit = get_json(
        &admin_client,
        &format!("{base_url}/api/tenants/{tenant_code}/audit-logs/verify"),
    )
    .await;
    assert_eq!(audit["valid"], true);
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

async fn create_customer(client: &Client, base_url: &str, tenant_code: &str, code: &str) -> Value {
    post_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/customers"),
        json!({
            "customer_code": code,
            "customer_name": format!("Customer {code}"),
            "biz_reg_no": format!("22081{}123", &code[1..]),
            "corp_reg_no": null,
            "industry_code": "62010",
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"]
        }),
        StatusCode::CREATED,
    )
    .await
}

async fn login(base_url: &str, tenant_code: &str, login_id: &str) -> String {
    let client = Client::new();
    let auth = post_json(
        &client,
        &format!("{base_url}/api/auth/login"),
        json!({"tenant_code": tenant_code, "login_id": login_id, "password": "ChangeMe123!"}),
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
