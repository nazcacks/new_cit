use std::env;

use axum::serve;
use cit_system::{db, queue, router, seed, AppState, Config};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client, StatusCode,
};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use uuid::Uuid;

#[tokio::test]
async fn phase_h_erp_mock_adapters_retry_and_audit_work() {
    let (base_url, state) = spawn_seeded_app().await;
    let login = login_as(&base_url, "demo", "admin", "ChangeMe123!").await;
    let client = authenticated_client(login["token"].as_str().unwrap());

    for (vendor, prefix) in [("DOUZONE", "DZ-"), ("SAP", "SAP-"), ("ORACLE_EBS", "EBS-")] {
        let by_id = create_phase_h_year(&client, &base_url, vendor).await;
        let root = format!("{base_url}/api/tenants/demo/business-years/{by_id}");
        let enqueue = post_json(
            &client,
            &format!("{root}/erp/imports"),
            json!({
                "vendor": vendor,
                "source_system": format!("{vendor}-MOCK"),
                "mock_profile": "balanced",
                "max_attempts": 1
            }),
            StatusCode::ACCEPTED,
        )
        .await;

        assert_eq!(enqueue["run"]["vendor"].as_str(), Some(vendor));
        assert_eq!(enqueue["run"]["status"].as_str(), Some("QUEUED"));
        assert_eq!(enqueue["job"]["job_type"].as_str(), Some("erp_import"));

        run_until_job_status(
            &state,
            enqueue["job"]["job_id"].as_str().unwrap(),
            "succeeded",
        )
        .await;
        let run = get_json(
            &client,
            &format!(
                "{root}/erp/imports/{}",
                enqueue["run"]["run_id"].as_i64().unwrap()
            ),
        )
        .await;
        assert_eq!(run["status"].as_str(), Some("IMPORTED"));
        assert_eq!(run["row_count"].as_i64(), Some(6));
        assert_eq!(run["valid_count"].as_i64(), Some(6));
        assert!(run["import_batch_id"].as_i64().is_some(), "{run}");

        let lines = get_json(&client, &format!("{root}/tax-data/financial-statements")).await;
        assert_vendor_lines(&lines, prefix);

        let audit = get_json(&client, &format!("{base_url}/api/tenants/demo/audit-logs")).await;
        let run_id = run["run_id"].as_i64().unwrap();
        assert!(has_audit(&audit, run_id, "ERP_IMPORT_ENQUEUED"), "{audit}");
        assert!(has_audit(&audit, run_id, "ERP_IMPORT_SUCCEEDED"), "{audit}");
    }

    let retry_by_id = create_phase_h_year(&client, &base_url, "SAP_RETRY").await;
    let retry_root = format!("{base_url}/api/tenants/demo/business-years/{retry_by_id}");
    let retry = post_json(
        &client,
        &format!("{retry_root}/erp/imports"),
        json!({
            "vendor": "SAP",
            "mock_profile": "fail_once",
            "max_attempts": 2
        }),
        StatusCode::ACCEPTED,
    )
    .await;
    let retry_job_id = retry["job"]["job_id"].as_str().unwrap();
    let first_attempt = queue::run_once(state.clone())
        .await
        .expect("first retry worker pass")
        .expect("retry job claimed");
    assert_eq!(first_attempt.status, "pending");
    assert_eq!(first_attempt.attempts, 1);
    assert!(
        first_attempt
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("transient"),
        "{first_attempt:?}"
    );

    let retry_run_id = retry["run"]["run_id"].as_i64().unwrap();
    let after_first = get_json(&client, &format!("{retry_root}/erp/imports/{retry_run_id}")).await;
    assert_eq!(after_first["status"].as_str(), Some("RETRYING"));
    assert_eq!(after_first["attempt_count"].as_i64(), Some(1));

    let retry_uuid = retry_job_id.parse::<Uuid>().unwrap();
    sqlx::query("UPDATE jobs SET next_run_at = NOW() WHERE job_id = $1")
        .bind(retry_uuid)
        .execute(&state.pool)
        .await
        .expect("force retry due");
    run_until_job_status(&state, retry_job_id, "succeeded").await;
    let after_retry = get_json(&client, &format!("{retry_root}/erp/imports/{retry_run_id}")).await;
    assert_eq!(after_retry["status"].as_str(), Some("IMPORTED"));
    assert_eq!(after_retry["attempt_count"].as_i64(), Some(2));

    let failed_by_id = create_phase_h_year(&client, &base_url, "DOUZONE_FAIL").await;
    let failed_root = format!("{base_url}/api/tenants/demo/business-years/{failed_by_id}");
    let failed = post_json(
        &client,
        &format!("{failed_root}/erp/imports"),
        json!({
            "vendor": "더존",
            "mock_profile": "fail",
            "max_attempts": 1
        }),
        StatusCode::ACCEPTED,
    )
    .await;
    run_until_job_status(
        &state,
        failed["job"]["job_id"].as_str().unwrap(),
        "dead_letter",
    )
    .await;
    let failed_run_id = failed["run"]["run_id"].as_i64().unwrap();
    let failed_run = get_json(
        &client,
        &format!("{failed_root}/erp/imports/{failed_run_id}"),
    )
    .await;
    assert_eq!(failed_run["status"].as_str(), Some("FAILED"));
    assert!(
        failed_run["last_error"]
            .as_str()
            .unwrap_or_default()
            .contains("forced failure"),
        "{failed_run}"
    );

    let audit = get_json(&client, &format!("{base_url}/api/tenants/demo/audit-logs")).await;
    assert!(
        has_audit(&audit, retry_run_id, "ERP_IMPORT_RETRYING"),
        "{audit}"
    );
    assert!(
        has_audit(&audit, retry_run_id, "ERP_IMPORT_SUCCEEDED"),
        "{audit}"
    );
    assert!(
        has_audit(&audit, failed_run_id, "ERP_IMPORT_FAILED"),
        "{audit}"
    );
    let verification = get_json(
        &client,
        &format!("{base_url}/api/tenants/demo/audit-logs/verify"),
    )
    .await;
    assert_eq!(
        verification["valid"].as_bool(),
        Some(true),
        "{verification}"
    );
}

async fn spawn_seeded_app() -> (String, AppState) {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let pool = db::connect(&database_url).await.expect("db connection");
    db::migrate(&pool).await.expect("migrations");
    seed::run_demo_seed(
        &pool,
        seed::DemoSeedOptions {
            reset: true,
            ..seed::DemoSeedOptions::default()
        },
    )
    .await
    .expect("demo seed");
    let state = AppState::new(pool, Config::test(database_url));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(state.clone());
    tokio::spawn(async move {
        serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), state)
}

async fn create_phase_h_year(client: &Client, base_url: &str, label: &str) -> i64 {
    let suffix = Uuid::new_v4().simple().to_string();
    let customer = post_json(
        client,
        &format!("{base_url}/api/tenants/demo/customers"),
        json!({
            "customer_code": format!("PHASEH{}{}", label.replace('_', ""), &suffix[..6]),
            "customer_name": format!("Phase H {label}"),
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
    let year = post_json(
        client,
        &format!("{base_url}/api/tenants/demo/business-years"),
        json!({
            "customer_id": customer_id,
            "year_label": 2026,
            "start_date": "2026-01-01",
            "end_date": "2026-12-31"
        }),
        StatusCode::CREATED,
    )
    .await;
    year["by_id"].as_i64().unwrap()
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

fn assert_vendor_lines(lines: &Value, prefix: &str) {
    let rows = lines.as_array().expect("financial statement rows");
    let matching = rows
        .iter()
        .filter(|line| {
            line["account_code"]
                .as_str()
                .is_some_and(|code| code.starts_with(prefix))
        })
        .count();
    assert_eq!(matching, 6, "{lines}");
}

fn has_audit(logs: &Value, run_id: i64, action: &str) -> bool {
    let record_id = run_id.to_string();
    logs.as_array().unwrap().iter().any(|entry| {
        entry["table_name"] == "erp_import_runs"
            && entry["record_id"].as_str() == Some(record_id.as_str())
            && entry["action"].as_str() == Some(action)
    })
}

async fn login_as(base_url: &str, tenant_code: &str, login_id: &str, password: &str) -> Value {
    let client = Client::new();
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
    let text = response.text().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn post_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.post(url).json(&body).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}
