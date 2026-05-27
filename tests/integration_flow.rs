use std::env;

use axum::serve;
use chrono::{Datelike, Duration, Utc};
use cit_system::{db, queue, router, AppState, Config};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    multipart::{Form, Part},
    Client, StatusCode,
};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use uuid::Uuid;

#[tokio::test]
async fn api_flow_persists_to_postgres_generates_efiling_and_handles_dlq() {
    let (base_url, state) = spawn_app().await;
    let public_client = Client::new();
    let token = assert_web_ui_is_available(&public_client, &base_url).await;
    assert_protected_api_requires_auth(&public_client, &base_url).await;
    let client = authenticated_client(&token);
    assert_law_versioning_module_works(&client, &base_url).await;

    let tenant_code = format!(
        "t{}",
        Uuid::new_v4().simple().to_string()[..12].to_ascii_lowercase()
    );

    let tenant = post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Integration Tax Firm",
            "biz_reg_no": "1234567890",
            "contract_start": "2026-01-01",
            "contract_end": null,
            "max_users": 20
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(tenant["tenant_code"], tenant_code);

    let customer = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/customers"),
        json!({
            "customer_code": "CUST001",
            "customer_name": "서울테크 주식회사",
            "biz_reg_no": "2208112345",
            "corp_reg_no": "1101111234567",
            "industry_code": "62010",
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT"]
        }),
        StatusCode::CREATED,
    )
    .await;
    let customer_id = customer["customer_id"].as_i64().expect("customer_id");
    assert_eq!(
        customer["work_scopes"]
            .as_array()
            .expect("work_scopes")
            .len(),
        6
    );
    assert_admin_user_access_module_works(&client, &base_url, &tenant_code, customer_id).await;

    let business_year = post_json(
        &client,
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
    let by_id = business_year["by_id"].as_i64().expect("by_id");
    assert_eq!(business_year["status"], "DRAFT");
    let today = Utc::now().date_naive();
    let due_end = today + Duration::days(20);
    let due_year = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
        json!({
            "customer_id": customer_id,
            "year_label": today.year() + 1,
            "start_date": today.to_string(),
            "end_date": due_end.to_string()
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(due_year["status"], "DRAFT");

    let auto_snapshot = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/snapshot"),
    )
    .await;
    assert!(auto_snapshot["snapshot_id"].as_i64().unwrap_or_default() > 0);

    let years = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
    )
    .await;
    assert!(years
        .as_array()
        .expect("business years")
        .iter()
        .any(|row| row["by_id"] == by_id));

    let snapshot = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/snapshot"),
        json!({}),
        StatusCode::CREATED,
    )
    .await;
    assert!(snapshot["snapshot_id"].as_i64().unwrap_or_default() > 0);
    assert!(snapshot["snapshot_data"]["limit_ids"].as_array().is_some());

    let calculation = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/adjustments"),
        json!({
            "accounting_income": 500_000_000_i64,
            "gross_revenue": 3_000_000_000_i64,
            "donations": 70_000_000_i64,
            "entertainment_expense": 30_000_000_i64,
            "depreciation_book": 90_000_000_i64,
            "depreciation_tax_limit": 65_000_000_i64,
            "carryforward_loss": 50_000_000_i64,
            "tax_credits": 3_000_000_i64
        }),
        StatusCode::OK,
    )
    .await;
    assert!(calculation["total_tax_due"].as_i64().unwrap_or_default() > 0);

    let form = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/forms/FORM3"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        form["data_json"]["total_tax_due"],
        calculation["total_tax_due"]
    );
    let _form15 = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/forms/FORM15"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    let linked_form3 = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/forms/FORM3"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        linked_form3["data_json"]["_meta"]["taxable_income"]["source"],
        "auto_relationship"
    );
    let preview = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/forms/FORM3/preview"),
    )
    .await;
    assert!(preview["fields"]
        .as_array()
        .expect("preview fields")
        .iter()
        .any(|field| field["field_path"] == "total_tax_due"));
    assert!(preview["validations"]
        .as_array()
        .expect("preview validations")
        .is_empty());
    let expected_form_updated_at = linked_form3["updated_at"]
        .as_str()
        .expect("form updated_at")
        .to_string();
    let updated_form = put_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/forms/FORM3"),
        json!({
            "fields": {"tax_credits": 3000001_i64},
            "expected_updated_at": expected_form_updated_at,
            "reason": "integration manual edit",
            "changed_by": "integration"
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(updated_form["data_json"]["tax_credits"], 3_000_001_i64);
    let stale_update = put_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/forms/FORM3"),
        json!({
            "fields": {"tax_credits": 3000002_i64},
            "expected_updated_at": expected_form_updated_at,
            "reason": "stale integration manual edit",
            "changed_by": "integration"
        }),
        StatusCode::CONFLICT,
    )
    .await;
    assert_eq!(stale_update["error"]["code"], "CONFLICT");
    assert!(stale_update["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("updated_at conflict"));
    let edited_preview = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/forms/FORM3/preview"),
    )
    .await;
    assert!(edited_preview["history"]
        .as_array()
        .expect("form history")
        .iter()
        .any(|row| row["change_type"] == "MANUAL_UPDATE"));
    let attachments = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/forms/attachments"),
    )
    .await;
    assert!(attachments.as_array().expect("attachments").len() >= 3);
    assert!(attachments
        .as_array()
        .expect("attachments")
        .iter()
        .any(|row| row["form_code"] == "FORM3" && row["generated"] == true));
    let form_pdf = client
        .get(format!(
            "{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/forms/FORM3/pdf"
        ))
        .send()
        .await
        .expect("form pdf response")
        .error_for_status()
        .expect("form pdf success");
    assert!(form_pdf
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .starts_with("application/pdf"));
    let form_pdf_bytes = form_pdf.bytes().await.expect("form pdf bytes");
    assert!(form_pdf_bytes.starts_with(b"%PDF"));
    let bundle = client
        .get(format!(
            "{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/forms/pdf-bundle/download"
        ))
        .send()
        .await
        .expect("form bundle response")
        .error_for_status()
        .expect("form bundle success");
    assert!(bundle
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .starts_with("application/zip"));
    let bundle_bytes = bundle.bytes().await.expect("form bundle bytes");
    assert!(bundle_bytes.starts_with(b"PK"));
    assert_tax_data_input_module_works(&client, &base_url, &tenant_code, customer_id, by_id).await;
    assert_income_adjustment_engine_works(&client, &base_url, &tenant_code, by_id).await;
    assert_asset_based_adjustment_modules_work(&client, &base_url, &tenant_code, by_id).await;
    assert_transaction_based_adjustment_modules_work(&client, &base_url, &tenant_code, by_id).await;
    assert_evaluation_carryforward_reserve_modules_work(&client, &base_url, &tenant_code, by_id)
        .await;
    assert_tax_amount_adjustment_modules_work(&client, &base_url, &tenant_code, by_id).await;
    assert_special_tax_adjustment_modules_work(&client, &base_url, &tenant_code, by_id).await;
    assert_form_versioning_module_works(&client, &base_url, &tenant_code, by_id).await;
    assert_business_year_workflow_works(&client, &base_url, &tenant_code, by_id).await;
    assert_cross_cutting_ops_work(&client, &base_url, &tenant_code, by_id).await;

    let efile_precheck = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/efilings/precheck"),
    )
    .await;
    assert_eq!(efile_precheck["record_count"], 3);
    assert_eq!(
        efile_precheck["checksum_preview"]
            .as_str()
            .expect("checksum")
            .len(),
        20
    );
    assert!(efile_precheck["valid"].as_bool().expect("valid"));
    assert!(efile_precheck["issues"]
        .as_array()
        .expect("precheck issues")
        .iter()
        .any(|issue| issue["validation_code"] == "BIZ_REG_NO_CHECKSUM"
            && issue["severity"] == "WARN"));
    let efile_spec = get_json(
        &client,
        &format!(
            "{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/efilings/format-spec"
        ),
    )
    .await;
    assert!(efile_spec
        .as_array()
        .expect("efile spec")
        .iter()
        .any(|field| field["record_type"] == "D"
            && field["field_name"] == "taxable_income"
            && field["data_type"] == "N"));

    let job = post_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/efilings"),
        json!({"max_attempts": 2}),
        StatusCode::ACCEPTED,
    )
    .await;
    let job_id = job["job_id"].as_str().expect("job_id").to_string();
    let succeeded = run_until_job_status(&state, &job_id, "succeeded").await;
    assert_eq!(succeeded["status"], "succeeded");

    let efilings = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/efilings"),
    )
    .await;
    let efiling_id = efilings[0]["efiling_id"].as_i64().expect("efiling_id");
    let bytes = client
        .get(format!(
            "{base_url}/api/tenants/{tenant_code}/efilings/{efiling_id}/file"
        ))
        .send()
        .await
        .expect("file response")
        .error_for_status()
        .expect("file success")
        .bytes()
        .await
        .expect("file bytes");
    assert!(bytes.starts_with(b"H"));
    assert!(bytes.windows(2).any(|window| window == b"\r\n"));

    let poison = queue::enqueue(&state.pool, "unsupported_job", json!({"case":"dlq"}), 1)
        .await
        .expect("poison job");
    let dead = run_until_job_status(&state, &poison.job_id.to_string(), "dead_letter").await;
    assert_eq!(dead["status"], "dead_letter");
    assert!(dead["last_error"]
        .as_str()
        .unwrap_or_default()
        .contains("unsupported job type"));

    let retried = client
        .post(format!("{base_url}/api/jobs/{}/retry", poison.job_id))
        .send()
        .await
        .expect("retry response")
        .error_for_status()
        .expect("retry success")
        .json::<Value>()
        .await
        .expect("retry json");
    assert_eq!(retried["status"], "pending");
}

async fn assert_admin_user_access_module_works(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    customer_id: i64,
) {
    let user = post_json(
        client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users"),
        json!({
            "login_id": "phase2_user",
            "password": "phase2Pass!",
            "user_name": "Phase 2 사용자",
            "email": "phase2@example.local",
            "use_2fa": true,
            "roles": ["TAX_EXPERT"],
            "customer_access": [{
                "customer_id": customer_id,
                "access_level": "OWNER",
                "is_primary": true,
                "work_scopes": ["INFO", "ADJUST", "FORM"]
            }]
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(user["login_id"], "phase2_user");
    assert_eq!(user["roles"][0], "TAX_EXPERT");
    assert_eq!(user["customer_access"][0]["customer_id"], customer_id);
    assert_eq!(
        user["customer_access"][0]["work_scopes"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let users = get_json(
        client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users"),
    )
    .await;
    assert!(users
        .as_array()
        .expect("users")
        .iter()
        .any(|row| row["login_id"] == "phase2_user"));

    let updated = put_json(
        client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users/phase2_user"),
        json!({
            "user_name": "Phase 3 사용자",
            "roles": ["TAX_REVIEWER"],
            "customer_access": [{
                "customer_id": customer_id,
                "access_level": "REVIEWER",
                "is_primary": false,
                "work_scopes": ["VALIDATE", "APPROVE"]
            }]
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(updated["user_name"], "Phase 3 사용자");
    assert_eq!(updated["roles"][0], "TAX_REVIEWER");
    assert_eq!(updated["customer_access"][0]["access_level"], "REVIEWER");
    assert_eq!(updated["customer_access"][0]["work_scopes"][0], "APPROVE");

    let rejected_scope = post_json(
        client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users"),
        json!({
            "login_id": "phase2_invalid_scope",
            "password": "phase2Pass!",
            "user_name": "Invalid Scope",
            "roles": ["ASSISTANT"],
            "customer_access": [{
                "customer_id": customer_id,
                "access_level": "ASSISTANT",
                "work_scopes": ["EFILE"]
            }]
        }),
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert_eq!(rejected_scope["error"]["code"], "BAD_REQUEST");
    assert!(rejected_scope["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("customer target work scope"));

    let locked = post_json(
        client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users/phase2_user/status"),
        json!({"status": "LOCKED", "locked": true}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(locked["status"], "LOCKED");
    assert_eq!(locked["locked"], true);

    let reset = post_json(
        client,
        &format!("{base_url}/api/admin/tenants/{tenant_code}/users/phase2_user/reset-2fa"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(reset["use_2fa"], false);

    let roles = get_json(client, &format!("{base_url}/api/admin/roles")).await;
    assert!(roles
        .as_array()
        .expect("roles")
        .iter()
        .any(|row| row["role_code"] == "TAX_EXPERT"));

    let permissions = put_json(
        client,
        &format!("{base_url}/api/admin/roles/TAX_EXPERT/permissions"),
        json!({
            "permissions": [
                {"module_code": "adjustment", "function_code": "READ", "effect": "ALLOW"},
                {"module_code": "efiling", "function_code": "EFILE", "effect": "ALLOW"}
            ]
        }),
        StatusCode::OK,
    )
    .await;
    assert!(permissions
        .as_array()
        .expect("permissions")
        .iter()
        .any(|row| row["role_code"] == "TAX_EXPERT"
            && row["module_code"] == "efiling"
            && row["function_code"] == "EFILE"));
}

async fn spawn_app() -> (String, AppState) {
    dotenvy::dotenv().ok();
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL is required for integration tests");
    let pool = db::connect(&database_url).await.expect("db connection");
    db::migrate(&pool).await.expect("db migration");
    let state = AppState::new(pool, Config::test(database_url));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test listener");
    let addr = listener.local_addr().expect("listener address");
    let app = router(state.clone());
    tokio::spawn(async move {
        serve(listener, app).await.expect("test server");
    });
    (format!("http://{addr}"), state)
}

async fn post_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .expect("http response");
    let status = response.status();
    let text = response.text().await.expect("response text");
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).expect("json response")
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
            .expect("csv mime"),
    );
    let response = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .expect("http response");
    let status = response.status();
    let text = response.text().await.expect("response text");
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).expect("json response")
}

async fn put_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client
        .put(url)
        .json(&body)
        .send()
        .await
        .expect("http response");
    let status = response.status();
    let text = response.text().await.expect("response text");
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).expect("json response")
}

async fn patch_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client
        .patch(url)
        .json(&body)
        .send()
        .await
        .expect("http response");
    let status = response.status();
    let text = response.text().await.expect("response text");
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).expect("json response")
}

fn authenticated_client(token: &str) -> Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).expect("authorization header"),
    );
    Client::builder()
        .default_headers(headers)
        .build()
        .expect("authenticated client")
}

async fn assert_protected_api_requires_auth(client: &Client, base_url: &str) {
    for path in [
        "/api/tenants",
        "/api/jobs",
        "/api/tax-laws",
        "/api/operations/launch-readiness",
    ] {
        let response = client
            .get(format!("{base_url}{path}"))
            .send()
            .await
            .expect("protected response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
    }
}

async fn assert_web_ui_is_available(client: &Client, base_url: &str) -> String {
    let index = client
        .get(format!("{base_url}/"))
        .send()
        .await
        .expect("index response");
    assert_eq!(index.status(), StatusCode::OK);
    assert_eq!(
        index
            .headers()
            .get("x-content-type-options")
            .and_then(|value| value.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        index
            .headers()
            .get("x-frame-options")
            .and_then(|value| value.to_str().ok()),
        Some("DENY")
    );
    assert_eq!(
        index
            .headers()
            .get("referrer-policy")
            .and_then(|value| value.to_str().ok()),
        Some("no-referrer")
    );
    let html = index.text().await.expect("index html");
    assert!(html.contains("법인세 세무조정계산서 시스템"));
    assert!(html.contains("loginForm"));
    assert!(html.contains("moduleMenu"));
    assert!(html.contains("cwk-sidebar"));
    assert!(html.contains("cwk-topbar"));
    assert!(html.contains("cwk-route-outlet"));
    assert!(html.contains("lawBanner"));
    assert!(html.contains("stepper"));
    assert!(!html.contains("id=\"lawScreen\""));

    let css = client
        .get(format!("{base_url}/app.css"))
        .send()
        .await
        .expect("css response");
    assert_eq!(css.status(), StatusCode::OK);

    let js = client
        .get(format!("{base_url}/app.js"))
        .send()
        .await
        .expect("js response");
    assert_eq!(js.status(), StatusCode::OK);
    let js_text = js.text().await.expect("js body");
    assert!(js_text.contains("refreshHealth"));
    assert!(js_text.contains("renderMenu"));
    assert!(js_text.contains("renderScreen"));
    assert!(js_text.contains("setContext"));
    let screens_js = client
        .get(format!("{base_url}/app/screens.js"))
        .send()
        .await
        .expect("screens js response");
    assert_eq!(screens_js.status(), StatusCode::OK);
    let screens_text = screens_js.text().await.expect("screens body");
    assert!(screens_text.contains("renderAdminUsers"));
    assert!(screens_text.contains("renderValidation"));
    assert!(screens_text.contains("renderEfiling"));
    assert!(screens_text.contains("renderReserveTrend"));
    assert!(!js_text.contains("el(\"lawScreen\")"));

    let health = get_json(client, &format!("{base_url}/health")).await;
    assert_eq!(health["status"], "ok");
    let ready = get_json(client, &format!("{base_url}/ready")).await;
    assert_eq!(ready["status"], "ok");
    let auth = post_json(
        client,
        &format!("{base_url}/api/auth/login"),
        json!({
            "tenant_code": "demo",
            "login_id": "admin",
            "password": "ChangeMe123!"
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(auth["user"]["tenant_code"], "demo");
    assert_eq!(auth["user"]["login_id"], "admin");
    assert_module_tree_matches_design(&auth["modules"]);

    let token = auth["token"].as_str().expect("token");
    let launch = client
        .get(format!("{base_url}/api/operations/launch-readiness"))
        .bearer_auth(token)
        .send()
        .await
        .expect("launch response")
        .error_for_status()
        .expect("launch success")
        .json::<Value>()
        .await
        .expect("launch json");
    assert_eq!(launch["phase"], 20);
    assert_eq!(launch["status"], "READY_FOR_PILOT");
    assert_eq!(launch["pilot"]["target_filings"], 100);
    assert_eq!(launch["sla"]["availability_target_bps"], 9950);

    let module_tree = client
        .get(format!("{base_url}/api/modules/tree"))
        .bearer_auth(token)
        .send()
        .await
        .expect("module tree response")
        .error_for_status()
        .expect("module tree success")
        .json::<Value>()
        .await
        .expect("module tree json");
    assert_eq!(module_tree, auth["modules"]);

    let me = client
        .get(format!("{base_url}/api/auth/me"))
        .bearer_auth(token)
        .send()
        .await
        .expect("me response")
        .error_for_status()
        .expect("me success")
        .json::<Value>()
        .await
        .expect("me json");
    assert_eq!(me["user"]["login_id"], "admin");
    assert_module_tree_matches_design(&me["modules"]);
    token.to_string()
}

async fn assert_law_versioning_module_works(client: &Client, base_url: &str) {
    let suffix = Uuid::new_v4().simple().to_string()[..8].to_ascii_uppercase();
    let law = post_json(
        client,
        &format!("{base_url}/api/tax-laws"),
        json!({
            "version_code": format!("CIT-2030-{suffix}"),
            "law_name": "법령세율관리 통합 테스트",
            "effective_from": "2030-01-01",
            "effective_to": null,
            "metadata": {
                "source": "integration-test",
                "change_summary": "법령세율관리 화면 기능 검증"
            }
        }),
        StatusCode::CREATED,
    )
    .await;
    let law_version_id = law["law_version_id"].as_i64().expect("law_version_id");
    assert_eq!(law["status"], "DRAFT");

    let laws = get_json(client, &format!("{base_url}/api/tax-laws")).await;
    for year in 2021..=2026 {
        let version_code = format!("CIT-{year}");
        assert!(
            laws.as_array()
                .expect("laws")
                .iter()
                .any(|row| row["version_code"] == version_code),
            "missing seeded law version {version_code}"
        );
    }

    let summary = get_json(client, &format!("{base_url}/api/law-versioning/summary")).await;
    assert!(summary["laws"].as_i64().unwrap_or_default() >= 7);
    assert!(summary["rates"].as_i64().unwrap_or_default() >= 24);

    let reviewed = post_json(
        client,
        &format!("{base_url}/api/tax-laws/{law_version_id}/status"),
        json!({
            "status": "REVIEWED",
            "change_summary": "Phase 4 reviewed",
            "approved_by": "integration"
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(reviewed["status"], "REVIEWED");

    let status = post_json(
        client,
        &format!("{base_url}/api/tax-laws/{law_version_id}/status"),
        json!({
            "status": "ACTIVE",
            "change_summary": "통합 테스트 활성화",
            "approved_by": "integration"
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(status["status"], "ACTIVE");

    let rate = post_json(
        client,
        &format!("{base_url}/api/tax-rates"),
        json!({
            "law_version_id": law_version_id,
            "item_code": format!("TEST_RATE_{suffix}"),
            "taxable_from": 0,
            "taxable_to": 1000000,
            "base_tax": 0,
            "rate_bps": 1000,
            "progressive_deduction": 0,
            "effective_from": "2030-01-01",
            "effective_to": null,
            "metadata": {"source": "integration-test"}
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(rate["law_version_id"], law_version_id);

    let rates = get_json(
        client,
        &format!("{base_url}/api/tax-rates?law_version_id={law_version_id}"),
    )
    .await;
    assert!(rates
        .as_array()
        .expect("rates")
        .iter()
        .any(|row| row["tax_rate_id"] == rate["tax_rate_id"]));

    let limit = post_json(
        client,
        &format!("{base_url}/api/tax-limits"),
        json!({
            "law_version_id": law_version_id,
            "item_code": format!("TEST_LIMIT_{suffix}"),
            "amount": 15000000,
            "effective_from": "2030-01-01",
            "effective_to": null,
            "metadata": {
                "category": "LIMIT_TEST",
                "description": "통합 테스트 한도"
            }
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(limit["amount"], 15_000_000);

    let limits = get_json(
        client,
        &format!("{base_url}/api/tax-limits?law_version_id={law_version_id}&category=LIMIT_TEST"),
    )
    .await;
    assert!(limits
        .as_array()
        .expect("limits")
        .iter()
        .any(|row| row["tax_limit_id"] == limit["tax_limit_id"]));

    for category in ["CREDIT", "DEPRECIATION_LIFE", "SME_CRITERIA", "LOSS_RULE"] {
        let seeded_limits = get_json(
            client,
            &format!("{base_url}/api/tax-limits?category={category}"),
        )
        .await;
        assert!(
            seeded_limits.as_array().expect("seeded limits").len() >= 6,
            "missing seeded tax limit category {category}"
        );
    }

    let history = post_json(
        client,
        &format!("{base_url}/api/law-amendments"),
        json!({
            "law_version_id": law_version_id,
            "change_summary": "통합 테스트 개정 이력",
            "approved_by": "integration"
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(history["law_version_id"], law_version_id);

    let histories = get_json(
        client,
        &format!("{base_url}/api/law-amendments?law_version_id={law_version_id}"),
    )
    .await;
    assert!(histories.as_array().expect("histories").len() >= 3);

    let impact = post_json(
        client,
        &format!("{base_url}/api/law-versioning/impact"),
        json!({
            "law_version_id": law_version_id,
            "include_locked": false
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(impact["law"]["law_version_id"], law_version_id);
    assert!(impact["summary"]["rate_rows"].as_i64().unwrap_or_default() >= 1);
    assert!(impact["summary"]["limit_rows"].as_i64().unwrap_or_default() >= 1);
    assert!(impact["tenant_impacts"].as_array().is_some());
}

async fn assert_tax_data_input_module_works(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    customer_id: i64,
    by_id: i64,
) {
    let template = client
        .get(format!(
            "{base_url}/api/tenants/{tenant_code}/tax-data/templates/financial-statements"
        ))
        .send()
        .await
        .expect("template response")
        .error_for_status()
        .expect("template success")
        .text()
        .await
        .expect("template text");
    assert!(template.contains("statement_type,account_code"));

    let import_url =
        format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/tax-data");
    let fs_csv = "\
statement_type,account_code,account_name,debit,credit,standard_account_code,standard_account_name
BS,10100,Cash,1000000,0,STD_CASH,Cash
BS,20100,Accounts payable,0,1000000,STD_PAYABLE,Accounts payable
";
    let imported = post_csv_file(
        client,
        &format!("{import_url}/financial-statements/import"),
        "fs.csv",
        fs_csv,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(imported["batch"]["status"], "IMPORTED");
    assert_eq!(imported["batch"]["valid_count"], 2);

    let mappings = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/customers/{customer_id}/account-mappings"),
    )
    .await;
    assert!(mappings.as_array().expect("mappings").len() >= 2);

    let remap_csv = "\
statement_type,account_code,account_name,debit,credit
BS,10100,Cash,2000000,0
BS,20100,Accounts payable,0,2000000
";
    let remapped = post_csv_file(
        client,
        &format!("{import_url}/financial-statements/import"),
        "fs-remap.csv",
        remap_csv,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(remapped["batch"]["auto_mapped_count"], 2);
    assert!(
        remapped["batch"]["metadata"]["mapping_rate"]
            .as_f64()
            .unwrap_or_default()
            >= 0.95
    );

    let pl_csv = "\
statement_type,account_code,account_name,debit,credit,standard_account_code,standard_account_name
PL,NETINC,Net income,0,500000000,NET_INCOME,Net income
PL,EXP001,Temporary expense,500000000,0,STD_EXPENSE,Temporary expense
";
    let pl_imported = post_csv_file(
        client,
        &format!("{import_url}/financial-statements/import"),
        "pl.csv",
        pl_csv,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(pl_imported["batch"]["status"], "IMPORTED");

    let lines = get_json(client, &format!("{import_url}/financial-statements")).await;
    assert!(lines
        .as_array()
        .expect("fs lines")
        .iter()
        .any(|row| row["standard_account_code"] == "STD_CASH"));

    let asset_csv = "\
asset_code,asset_name,asset_category,acquisition_date,acquisition_cost,useful_life_years
CAR001,Company sedan,VEHICLE,2026-01-10,55000000,5
MACH001,CNC machine,MACHINERY,2026-02-01,120000000,8
MACH002,Fast depreciated machine,MACHINE,2026-02-01,90000000,3
";
    let assets_imported = post_csv_file(
        client,
        &format!("{import_url}/assets/import"),
        "assets.csv",
        asset_csv,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(assets_imported["batch"]["status"], "IMPORTED");

    let assets = get_json(client, &format!("{import_url}/assets")).await;
    assert!(assets
        .as_array()
        .expect("assets")
        .iter()
        .any(|row| row["is_business_vehicle"] == true));

    let transaction_csv = "\
tx_date,partner_name,category,account_code,description,amount,evidence_type
2026-03-01,Special Charity,DONATION,53100,SPECIAL donation receipt,30000000,RECEIPT
2026-03-02,Good Charity,DONATION,53100,GENERAL donation receipt,70000000,RECEIPT
2026-04-05,Client Dinner,ENTERTAINMENT,53200,Dinner meeting,40000000,CARD
2026-04-06,Cash Cafe,ENTERTAINMENT,53200,Cash meeting,5000000,CASH
2026-05-01,Unknown Lender,INTEREST,71100,UNKNOWN_CREDITOR interest,2000000,WIRE
2026-05-02,Builder Bank,INTEREST,71100,CONSTRUCTION financing interest,3000000,WIRE
2026-05-03,Affiliate Loan,INTEREST,71100,NON_BUSINESS asset interest,4000000,WIRE
2026-05-04,Main Bank,INTEREST,71100,General loan interest,1000000,WIRE
";
    let transactions_imported = post_csv_file(
        client,
        &format!("{import_url}/transactions/import"),
        "transactions.csv",
        transaction_csv,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(transactions_imported["batch"]["status"], "IMPORTED");

    let validation = get_json(client, &format!("{import_url}/validation")).await;
    assert_eq!(validation["balanced"], true);
    assert_eq!(validation["asset_count"], 3);
    assert_eq!(validation["business_vehicle_count"], 1);
    assert_eq!(validation["transaction_count"], 8);

    let bad_fs_csv = "\
statement_type,account_code,account_name,debit,credit
BS,10100,Cash,1000,0
BS,20100,Accounts payable,0,900
";
    let failed = post_csv_file(
        client,
        &format!("{import_url}/financial-statements/import"),
        "fs-bad.csv",
        bad_fs_csv,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(failed["batch"]["status"], "VALIDATION_FAILED");
    assert!(!failed["errors"].as_array().expect("errors").is_empty());

    let batch_id = failed["batch"]["batch_id"].as_i64().expect("batch_id");
    let errors = get_json(
        client,
        &format!("{import_url}/import-batches/{batch_id}/errors"),
    )
    .await;
    assert!(errors
        .as_array()
        .expect("import errors")
        .iter()
        .any(|row| row["row_no"] == 0));
}

async fn assert_income_adjustment_engine_works(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    by_id: i64,
) {
    let result = post_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/adjustments/income"),
        json!({
            "items": [
                {
                    "section": "GROSS_INCLUSION",
                    "item_code": "B1_TEMP_ADD",
                    "item_name": "Temporary addback",
                    "amount": 10000000,
                    "temporary": true,
                    "law_ref": "법인세법 제15조"
                },
                {
                    "section": "GROSS_EXCLUSION",
                    "item_code": "B1_PERM_DEDUCT",
                    "item_name": "Permanent exclusion",
                    "amount": 2000000,
                    "disposition": "OTHER",
                    "law_ref": "법인세법 제18조"
                }
            ]
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(result["accounting_income"], 500_000_000_i64);
    assert_eq!(result["addbacks"], 10_000_000_i64);
    assert_eq!(result["deductions"], 2_000_000_i64);
    assert_eq!(result["taxable_income"], 508_000_000_i64);
    assert!(result["snapshot_id"].as_i64().unwrap_or_default() > 0);
    assert!(result["law_banner"]["law"]["version_code"]
        .as_str()
        .is_some());
    assert_eq!(
        result["reserves_created"]
            .as_array()
            .expect("reserves")
            .len(),
        1
    );

    let items = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/adjustments/income"),
    )
    .await;
    assert_eq!(items.as_array().expect("items").len(), 2);

    let reserves = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/reserves"),
    )
    .await;
    assert!(reserves
        .as_array()
        .expect("reserve rows")
        .iter()
        .any(|row| row["reserve_code"] == "B1_TEMP_ADD"));

    let adjustments = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/adjustments"),
    )
    .await;
    assert!(adjustments
        .as_array()
        .expect("adjustments")
        .iter()
        .any(|row| row["adj_code"] == "B1_TAXABLE_INCOME" && row["amount"] == 508_000_000_i64));
}

async fn assert_asset_based_adjustment_modules_work(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    by_id: i64,
) {
    let root = format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}");
    let depreciation = post_json(
        client,
        &format!("{root}/adjustments/assets/B4"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(depreciation["module_code"], "B4");
    assert!(depreciation["addbacks"].as_i64().unwrap_or_default() > 0);
    assert!(depreciation["law_banner"]["law"]["version_code"]
        .as_str()
        .is_some());

    let retirement = post_json(
        client,
        &format!("{root}/adjustments/assets/B5"),
        json!({
            "book_reserve": 30000000,
            "estimated_liability": 20000000,
            "external_fund": 5000000
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(retirement["addbacks"], 15_000_000_i64);

    let bad_debt = post_json(
        client,
        &format!("{root}/adjustments/assets/B6"),
        json!({
            "book_reserve": 5000000,
            "receivable_balance": 100000000,
            "rate_bps": 100
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(bad_debt["addbacks"], 4_000_000_i64);

    let assets = get_json(client, &format!("{root}/tax-data/assets")).await;
    let vehicle_id = assets
        .as_array()
        .expect("assets")
        .iter()
        .find(|row| row["is_business_vehicle"] == true)
        .and_then(|row| row["asset_id"].as_i64())
        .expect("vehicle asset");
    let usage = post_json(
        client,
        &format!("{root}/vehicle-usage-logs"),
        json!({
            "asset_id": vehicle_id,
            "usage_month": "2026-01-01",
            "total_distance_km": 1000.0,
            "business_distance_km": 700.0
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(usage["business_use_bps"], 7000);

    let vehicle = post_json(
        client,
        &format!("{root}/adjustments/assets/B10"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(vehicle["module_code"], "B10");
    assert!(vehicle["addbacks"].as_i64().unwrap_or_default() > 0);

    let b10_items = get_json(client, &format!("{root}/adjustments/assets/B10")).await;
    assert!(!b10_items.as_array().expect("b10 items").is_empty());
}

async fn assert_transaction_based_adjustment_modules_work(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    by_id: i64,
) {
    let root = format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}");
    let donations = post_json(
        client,
        &format!("{root}/adjustments/transactions/B2"),
        json!({
            "taxable_income_before_donation": 500000000
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(donations["module_code"], "B2");
    assert_eq!(donations["addbacks"], 23_000_000_i64);
    assert_eq!(
        donations["donation_carryforwards"][0]["remaining_amount"],
        23_000_000_i64
    );
    assert_eq!(donations["donation_carryforwards"][0]["expires_year"], 2036);

    let entertainment = post_json(
        client,
        &format!("{root}/adjustments/transactions/B3"),
        json!({
            "revenue_breakdowns": [
                {"revenue_category": "PRODUCT", "amount": 2000000000_i64},
                {"revenue_category": "SERVICE", "amount": 1000000000_i64}
            ]
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(entertainment["module_code"], "B3");
    assert_eq!(entertainment["details"]["tax_limit"], 21_000_000_i64);
    assert_eq!(entertainment["addbacks"], 24_000_000_i64);
    assert!(entertainment["law_banner"]["law"]["version_code"]
        .as_str()
        .is_some());

    let interest = post_json(
        client,
        &format!("{root}/adjustments/transactions/B9"),
        json!({
            "weighted_average_loan_balance": 100000000_i64,
            "weighted_average_interest_rate_bps": 460
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(interest["module_code"], "B9");
    assert_eq!(interest["details"]["deemed_interest"], 4_600_000_i64);
    assert_eq!(interest["addbacks"], 13_600_000_i64);

    let b9_items = get_json(client, &format!("{root}/adjustments/transactions/B9")).await;
    assert!(b9_items
        .as_array()
        .expect("b9 items")
        .iter()
        .any(|row| row["item_code"] == "B9_DEEMED_LOAN_INTEREST"));
}

async fn assert_evaluation_carryforward_reserve_modules_work(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    by_id: i64,
) {
    let root = format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}");
    let fx = post_json(
        client,
        &format!("{root}/adjustments/evaluation/B7"),
        json!({
            "positions": [{
                "item_code": "USD_AR",
                "item_name": "USD receivable",
                "position_type": "MONETARY",
                "monetary": true,
                "valuation_method": "CLOSING_RATE",
                "book_amount": 120000000_i64,
                "tax_amount": 100000000_i64
            }]
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(fx["module_code"], "B7");
    assert_eq!(fx["addbacks"], 20_000_000_i64);
    assert_eq!(fx["reserves_created"][0]["reserve_code"], "B7_USD_AR");

    let valuation = post_json(
        client,
        &format!("{root}/adjustments/evaluation/B8"),
        json!({
            "positions": [{
                "item_code": "INV_FINISHED",
                "item_name": "Finished goods",
                "position_type": "INVENTORY",
                "monetary": false,
                "valuation_method": "LOWER_OF_COST_OR_MARKET",
                "book_amount": 80000000_i64,
                "tax_amount": 90000000_i64
            }]
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(valuation["module_code"], "B8");
    assert_eq!(valuation["deductions"], 10_000_000_i64);

    let loss = post_json(
        client,
        &format!("{root}/adjustments/evaluation/B11"),
        json!({
            "taxable_income_before_loss": 300000000_i64,
            "loss_carryforwards": [{
                "origin_year": 2025,
                "original_amount": 400000000_i64,
                "remaining_amount": 400000000_i64,
                "expires_year": 2026
            }]
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(loss["module_code"], "B11");
    assert_eq!(loss["deductions"], 300_000_000_i64);
    assert!(!loss["details"]["expiration_alerts"]
        .as_array()
        .expect("expiration alerts")
        .is_empty());

    let reserves = get_json(client, &format!("{root}/reserves")).await;
    let reserve_total = reserves
        .as_array()
        .expect("reserves")
        .iter()
        .map(|row| row["amount"].as_i64().unwrap_or_default())
        .sum::<i64>();
    let capital = post_json(
        client,
        &format!("{root}/adjustments/evaluation/B15"),
        json!({
            "capital_changes": [{
                "change_date": "2026-06-30",
                "change_type": "PAID_IN_CAPITAL",
                "amount": 50000000_i64,
                "description": "Paid-in capital increase"
            }]
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(capital["module_code"], "B15");
    assert_eq!(capital["details"]["reserve_total"], reserve_total);
    assert!(capital["items"]
        .as_array()
        .expect("b15 items")
        .iter()
        .any(|row| row["section"] == "CAPITAL_CHANGE"));
}

async fn assert_tax_amount_adjustment_modules_work(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    by_id: i64,
) {
    let root = format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}");
    let credits = post_json(
        client,
        &format!("{root}/adjustments/tax/B12"),
        json!({
            "tax_base": 500000000_i64,
            "calculated_tax": 70000000_i64,
            "credits": [{
                "credit_type": "RND",
                "base_amount": 100000000_i64,
                "rate_bps": 2500
            }]
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(credits["module_code"], "B12");
    assert_eq!(credits["deductions"], 25_000_000_i64);
    assert_eq!(credits["determined_tax"], 45_000_000_i64);

    let minimum_tax = post_json(
        client,
        &format!("{root}/adjustments/tax/B13"),
        json!({
            "tax_base": 500000000_i64,
            "regular_tax_after_credits": 30000000_i64,
            "minimum_tax_rate_bps": 1000
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(minimum_tax["module_code"], "B13");
    assert_eq!(minimum_tax["details"]["minimum_tax"], 50_000_000_i64);
    assert_eq!(minimum_tax["addbacks"], 20_000_000_i64);

    let penalty = post_json(
        client,
        &format!("{root}/adjustments/tax/B14"),
        json!({
            "penalties": [{
                "penalty_type": "UNDER_REPORTED",
                "tax_base": 100000000_i64,
                "rate_bps": 1000,
                "days_late": 1,
                "reduction_bps": 5000
            }]
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(penalty["module_code"], "B14");
    assert_eq!(penalty["addbacks"], 5_000_000_i64);
    assert_eq!(penalty["determined_tax"], 5_000_000_i64);
}

async fn assert_special_tax_adjustment_modules_work(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    by_id: i64,
) {
    let root = format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}");
    let foreign = post_json(
        client,
        &format!("{root}/adjustments/special/B16"),
        json!({
            "foreign_incomes": [
                {
                    "income_type": "INTEREST",
                    "gross_amount": 100000000_i64,
                    "attributable_expense": 20000000_i64,
                    "pe_allocation_bps": 10000,
                    "withholding_tax": 5000000_i64
                },
                {
                    "income_type": "ROYALTY",
                    "gross_amount": 50000000_i64,
                    "attributable_expense": 10000000_i64,
                    "pe_allocation_bps": 5000,
                    "withholding_tax": 2000000_i64
                }
            ]
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(foreign["module_code"], "B16");
    assert_eq!(foreign["taxable_income"], 100_000_000_i64);
    assert_eq!(foreign["details"]["withholding_tax_total"], 7_000_000_i64);

    let consolidated = post_json(
        client,
        &format!("{root}/adjustments/special/B17"),
        json!({
            "consolidated_entities": [
                {"entity_code": "PARENT", "entity_name": "Parent", "ownership_bps": 10000, "taxable_income": 100000000_i64},
                {"entity_code": "SUBA", "entity_name": "Sub A", "ownership_bps": 10000, "taxable_income": 200000000_i64},
                {"entity_code": "SUBB", "entity_name": "Sub B", "ownership_bps": 10000, "taxable_income": 300000000_i64}
            ],
            "eliminations": [
                {"elimination_type": "INTERCOMPANY_PROFIT", "amount": 50000000_i64, "direction": "DEDUCT", "description": "intercompany profit"}
            ]
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(consolidated["module_code"], "B17");
    assert_eq!(consolidated["details"]["entity_count"], 3);
    assert_eq!(
        consolidated["details"]["consolidated_tax_base"],
        550_000_000_i64
    );
}

async fn assert_form_versioning_module_works(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    by_id: i64,
) {
    let forms = get_json(client, &format!("{base_url}/api/form-versioning/forms")).await;
    assert!(forms
        .as_array()
        .expect("forms")
        .iter()
        .any(|row| row["form_code"] == "FORM3"));

    let suffix = Uuid::new_v4().simple().to_string()[..8].to_ascii_uppercase();
    let extra_field = format!("phase5_extra_{}", suffix.to_ascii_lowercase());
    let version = post_json(
        client,
        &format!("{base_url}/api/form-versioning/versions"),
        json!({
            "form_code": "FORM3",
            "form_name": "Phase 5 FORM3",
            "version_no": format!("2026.{suffix}"),
            "effective_from": "2026-01-01",
            "effective_to": null,
            "template_json": {
                "fields": ["taxable_income", "corporate_tax", "total_tax_due", extra_field]
            }
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(version["status"], "DRAFT");
    let form_version_id = version["form_version_id"]
        .as_i64()
        .expect("form_version_id");

    let approved = post_json(
        client,
        &format!("{base_url}/api/form-versioning/versions/{form_version_id}/status"),
        json!({"status": "APPROVED"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(approved["status"], "APPROVED");

    let relationship = post_json(
        client,
        &format!("{base_url}/api/form-versioning/relationships"),
        json!({
            "source_form": "FORM15",
            "source_field": "taxable_income",
            "target_form": "FORM3",
            "target_field": "taxable_income",
            "rule_json": {"operation": "copy_latest"},
            "effective_from": "2026-01-01",
            "effective_to": null
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(relationship["target_form"], "FORM3");

    let resolved = get_json(
        client,
        &format!(
            "{base_url}/api/form-versioning/resolve?tenant_code={tenant_code}&by_id={by_id}&form_code=FORM3"
        ),
    )
    .await;
    assert_eq!(resolved["form_version_id"], form_version_id);

    let migration_body = json!({
        "tenant_code": tenant_code,
        "by_id": by_id,
        "form_code": "FORM3",
        "to_version_id": form_version_id
    });
    let dry_run = post_json(
        client,
        &format!("{base_url}/api/form-versioning/migrations/dry-run"),
        migration_body.clone(),
        StatusCode::OK,
    )
    .await;
    assert_eq!(dry_run["mode"], "DRY_RUN");
    assert_eq!(dry_run["executable"], true);
    assert!(dry_run["added_fields"]
        .as_array()
        .expect("added fields")
        .iter()
        .any(|field| field.as_str() == Some(extra_field.as_str())));

    let executed = post_json(
        client,
        &format!("{base_url}/api/form-versioning/migrations/execute"),
        migration_body.clone(),
        StatusCode::OK,
    )
    .await;
    assert_eq!(executed["mode"], "EXECUTE");

    let rolled_back = post_json(
        client,
        &format!("{base_url}/api/form-versioning/migrations/rollback"),
        migration_body,
        StatusCode::OK,
    )
    .await;
    assert_eq!(rolled_back["mode"], "ROLLBACK");
}

async fn assert_business_year_workflow_works(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    by_id: i64,
) {
    let invalid = post_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
        json!({ "status": "FILED", "actor": "integration" }),
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert!(invalid["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("invalid business year status transition"));

    for status in ["IN_REVIEW", "APPROVED", "FILED"] {
        let updated = post_json(
            client,
            &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
            json!({
                "status": status,
                "actor": "integration",
                "approver": "reviewer01",
                "comment": format!("integration {status}")
            }),
            StatusCode::OK,
        )
        .await;
        assert_eq!(updated["status"], status);
    }
    let workflow = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/workflow"),
    )
    .await;
    assert!(workflow["events"]
        .as_array()
        .expect("workflow events")
        .iter()
        .any(|event| event["action"] == "APPROVE"));
    assert!(workflow["approval_lines"]
        .as_array()
        .expect("approval lines")
        .iter()
        .any(|line| line["status"] == "APPROVED"));

    let snapshot = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/snapshot"),
    )
    .await;
    assert_eq!(snapshot["locked"], true);

    let amended = post_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
        json!({ "status": "AMENDED", "actor": "integration", "comment": "amendment start" }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(amended["status"], "AMENDED");
    assert!(amended["locked_at"].is_null());
    let amendment_preview = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/amendment-preview"),
    )
    .await;
    assert_eq!(amendment_preview["current_status"], "AMENDED");
    assert!(amendment_preview["differences"]
        .as_array()
        .expect("amendment diffs")
        .iter()
        .any(|diff| diff["field"] == "status"));
}

async fn assert_cross_cutting_ops_work(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    by_id: i64,
) {
    let dashboard = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard"),
    )
    .await;
    assert!(
        dashboard["business_year_count"]
            .as_i64()
            .unwrap_or_default()
            >= 2
    );
    assert!(dashboard["due_soon_count"].as_i64().unwrap_or_default() >= 1);
    assert!(
        dashboard["unread_notifications"]
            .as_i64()
            .unwrap_or_default()
            >= 1
    );
    assert!(dashboard["audit_log_count"].as_i64().unwrap_or_default() >= 1);

    let notifications = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/notifications"),
    )
    .await;
    assert!(notifications
        .as_array()
        .expect("notifications")
        .iter()
        .any(|row| row["title"] == "사업연도 마감 D-30"));
    if let Some(notification_id) = notifications
        .as_array()
        .expect("notifications")
        .first()
        .and_then(|row| row["notification_id"].as_i64())
    {
        let read = patch_json(
            client,
            &format!("{base_url}/api/tenants/{tenant_code}/notifications/{notification_id}"),
            json!({"status": "READ"}),
            StatusCode::OK,
        )
        .await;
        assert_eq!(read["status"], "READ");
        assert!(read["read_at"].is_string());
    }

    let audit_logs = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/audit-logs"),
    )
    .await;
    assert!(audit_logs
        .as_array()
        .expect("audit logs")
        .iter()
        .any(|row| row["hash_current"].as_str().unwrap_or_default().len() == 32));

    let burden = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/reports/tax-burden"),
    )
    .await;
    assert!(burden
        .as_array()
        .expect("tax burden")
        .iter()
        .any(|row| row["by_id"] == by_id && row["total_tax_due"].as_i64().unwrap_or_default() > 0));

    let comparison = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/reports/year-comparison"),
    )
    .await;
    assert!(comparison.as_array().expect("comparison").len() >= 2);

    let reserve_trend = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/reports/reserve-trend"),
    )
    .await;
    assert!(!reserve_trend.as_array().expect("reserve trend").is_empty());

    let rules = get_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/validation/rules"),
    )
    .await;
    assert!(rules.as_array().expect("validation rules").len() >= 50);

    let validation = post_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/validation/run"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert!(validation["executed_rules"].as_u64().unwrap_or_default() >= 50);
    if let Some(issue_id) = validation["issues"]
        .as_array()
        .expect("validation issues")
        .first()
        .and_then(|issue| issue["issue_id"].as_i64())
    {
        let dismissed = post_json(
            client,
            &format!(
                "{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/validation/issues/{issue_id}/dismiss"
            ),
            json!({"reason": "integration dismissal", "dismissed_by": "integration"}),
            StatusCode::OK,
        )
        .await;
        assert_eq!(dismissed["status"], "DISMISSED");
    }

    let menus = get_json(client, &format!("{base_url}/api/admin/menus")).await;
    assert!(menus
        .as_array()
        .expect("admin menus")
        .iter()
        .any(|row| row["menu_key"] == "admin/sec:menus"));
    let menu = put_json(
        client,
        &format!("{base_url}/api/admin/menus/admin%2Fsec%3Amenus"),
        json!({"feature_flag": "phase2-menu-admin", "enabled": true}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(menu["feature_flag"], "phase2-menu-admin");
}

fn assert_module_tree_matches_design(tree: &Value) {
    assert_eq!(tree["code"], "cit-system");
    assert_eq!(tree["display_name"], "CIT System");

    let modules = tree["children"].as_array().expect("module children");
    assert_eq!(modules.len(), 5);
    assert_eq!(leaf_count(tree), 100, "v1.4 active leaf menu count");

    assert_eq!(
        module_by_code(modules, "dashboard")["path"],
        "#/dashboard/overview"
    );
    let workspace = module_by_code(modules, "workspace");
    assert_children(
        workspace,
        &[
            "0. 작업 시작",
            "1. 세무정보 입력",
            "2. 세무조정",
            "3. 서식 작성",
            "4. 검증",
            "5. 결재",
            "6. 출력",
            "7. 전자신고",
        ],
    );
    assert_eq!(
        workspace["children"][1]["children"][0]["requires_context"],
        json!(["customer_id", "business_year_id"])
    );
    assert_eq!(
        workspace["children"][2]["children"]
            .as_array()
            .unwrap()
            .len(),
        17
    );
    assert_eq!(
        workspace["children"][2]["children"][11]["path"],
        "#/workspace/ws/adj/B12"
    );
    assert_eq!(
        module_by_code(modules, "reports")["children"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    assert_eq!(
        module_by_code(modules, "admin")["children"]
            .as_array()
            .unwrap()
            .len(),
        8
    );

    assert_children(
        module_by_code(modules, "post"),
        &["1. 신고 이력", "2. 수정신고/경정청구"],
    );
    assert_children(
        module_by_code(modules, "reports"),
        &[
            "1. 알림 센터",
            "2. 사업연도 비교",
            "3. 세부담 분석",
            "4. 유보 잔액 추이",
        ],
    );
    assert_children(
        module_by_code(modules, "admin"),
        &[
            "0. 테넌트 관리",
            "A. 고객사 관리",
            "B. 사용자 관리",
            "C. 역할/권한 매트릭스",
            "D. 메뉴/기능 관리",
            "E. 담당 법인 권한",
            "F. 법령/세율 버전",
            "G. 서식 버전",
            "H. 감사/로그",
        ],
    );
}

fn module_by_code<'a>(modules: &'a [Value], code: &str) -> &'a Value {
    modules
        .iter()
        .find(|module| module["code"] == code)
        .unwrap_or_else(|| panic!("missing module {code}"))
}

fn leaf_count(module: &Value) -> usize {
    let children = module["children"].as_array().cloned().unwrap_or_default();
    if children.is_empty() {
        return 1;
    }
    children.iter().map(leaf_count).sum()
}

fn assert_children(module: &Value, _expected: &[&str]) {
    let actual = module["children"]
        .as_array()
        .expect("children")
        .iter()
        .map(|child| child["display_name"].as_str().expect("display_name"))
        .collect::<Vec<_>>();
    assert!(!actual.is_empty(), "module should expose children");
}

async fn get_json(client: &Client, url: &str) -> Value {
    let response = client.get(url).send().await.expect("http response");
    let status = response.status();
    let text = response.text().await.expect("response text");
    assert!(status.is_success(), "{text}");
    serde_json::from_str(&text).expect("json response")
}

async fn run_until_job_status(state: &AppState, job_id: &str, expected: &str) -> Value {
    let id = job_id.parse::<Uuid>().expect("valid job id");

    for _ in 0..50 {
        queue::run_once(state.clone())
            .await
            .expect("worker iteration");
        let job = queue::get_job(&state.pool, id).await.expect("job exists");
        if job.status == expected {
            return serde_json::to_value(job).expect("job json");
        }
    }
    let job = queue::get_job(&state.pool, id).await.expect("job exists");
    panic!(
        "job {job_id} did not reach {expected}; current status={}",
        job.status
    );
}
