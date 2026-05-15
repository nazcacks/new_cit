use std::env;

use axum::serve;
use cit_system::{db, queue, router, AppState, Config};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use uuid::Uuid;

#[tokio::test]
async fn api_flow_persists_to_postgres_generates_efiling_and_handles_dlq() {
    let (base_url, state) = spawn_app().await;
    let client = Client::new();
    assert_web_ui_is_available(&client, &base_url).await;
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
            "is_sme": true
        }),
        StatusCode::CREATED,
    )
    .await;
    let customer_id = customer["customer_id"].as_i64().expect("customer_id");

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

async fn assert_web_ui_is_available(client: &Client, base_url: &str) {
    let index = client
        .get(format!("{base_url}/"))
        .send()
        .await
        .expect("index response");
    assert_eq!(index.status(), StatusCode::OK);
    let html = index.text().await.expect("index html");
    assert!(html.contains("법인세 세무조정계산서 시스템"));
    assert!(html.contains("loginForm"));
    assert!(html.contains("moduleMenu"));
    assert!(html.contains("lawVersioningWorkspace"));
    assert!(html.contains("law-screen-laws"));
    assert!(html.contains("law-screen-rates"));
    assert!(html.contains("law-screen-limits"));
    assert!(html.contains("law-screen-credits"));
    assert!(html.contains("law-screen-depreciation-lives"));
    assert!(html.contains("law-screen-sme-criteria"));
    assert!(html.contains("law-screen-loss-rules"));
    assert!(html.contains("law-screen-snapshots"));
    assert!(html.contains("law-screen-impact"));
    assert!(html.contains("law-screen-history"));
    assert!(!html.contains("id=\"lawScreen\""));
    assert!(!html.contains("lawVersioningWorkspace\" class=\"panel law-workspace hidden"));

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
    assert!(js_text.contains("renderModuleMenu"));
    assert!(js_text.contains("normalizeModuleTree"));
    assert!(js_text.contains("navigateLawRoute"));
    assert!(js_text.contains("setLawScreenHtml"));
    assert!(js_text.contains("law-screen-laws-body"));
    assert!(js_text.contains("renderTaxRatesScreen"));
    assert!(js_text.contains("renderImpactScreen"));
    assert!(!js_text.contains("el(\"lawScreen\")"));

    let health = get_json(client, &format!("{base_url}/health")).await;
    assert_eq!(health["status"], "ok");

    let auth = post_json(
        client,
        &format!("{base_url}/api/auth/login"),
        json!({
            "tenant_code": "demo",
            "login_id": "admin",
            "password": "admin123!"
        }),
        StatusCode::OK,
    )
    .await;
    assert_eq!(auth["user"]["tenant_code"], "demo");
    assert_eq!(auth["user"]["login_id"], "admin");
    assert_module_tree_matches_design(&auth["modules"]);

    let token = auth["token"].as_str().expect("token");
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

    let summary = get_json(client, &format!("{base_url}/api/law-versioning/summary")).await;
    assert!(summary["laws"].as_i64().unwrap_or_default() >= 4);
    assert!(summary["rates"].as_i64().unwrap_or_default() >= 12);

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
    assert!(histories.as_array().expect("histories").len() >= 2);

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

fn assert_module_tree_matches_design(tree: &Value) {
    assert_eq!(tree["code"], "cit-system");
    assert_eq!(tree["display_name"], "CIT System");

    let modules = tree["children"].as_array().expect("module children");
    assert_eq!(modules.len(), 9);
    assert_eq!(
        modules.iter().map(child_count).sum::<usize>(),
        40,
        "detailed module count including law-versioning menus"
    );

    let law = module_by_code(modules, "law-versioning");
    assert_eq!(
        law["display_name"],
        "0. 법령·세율 버전 관리 모듈 (Tax Law Versioning) ★"
    );
    assert_children(
        law,
        &[
            "0.1 법령 버전 마스터",
            "0.2 법인세율표",
            "0.3 한도·율표",
            "0.4 세액공제·감면 율표",
            "0.5 기준내용연수표",
            "0.6 중소기업 판정기준",
            "0.7 결손금 공제규정",
            "0.8 사업연도별 적용 스냅샷",
            "0.9 영향 시뮬레이션",
            "0.10 개정 공지/이력",
        ],
    );
    assert_eq!(law["children"][0]["path"], "/modules/law-versioning/laws");
    assert_eq!(
        law["children"][9]["path"],
        "/modules/law-versioning/history"
    );

    assert_eq!(
        module_by_code(modules, "auth")["display_name"],
        "1. 인증/계정 모듈 (Auth Module)"
    );

    let admin = module_by_code(modules, "admin");
    assert_eq!(admin["display_name"], "2. 시스템 관리 모듈 (Admin Module)");
    assert_children(
        admin,
        &[
            "2.1 사용자 관리",
            "2.2 권한/역할 관리",
            "2.3 메뉴 관리",
            "2.4 테넌트 관리",
            "2.5 감사 로그",
        ],
    );

    assert_children(
        module_by_code(modules, "customer"),
        &[
            "3.1 법인 기본정보",
            "3.2 사업연도 관리",
            "3.3 세무대리 계약",
        ],
    );
    assert_children(
        module_by_code(modules, "tax-data"),
        &[
            "4.1 재무제표 입력/임포트",
            "4.2 계정과목 매핑",
            "4.3 거래처 정보",
            "4.4 자산/감가상각 정보",
        ],
    );
    assert_children(
        module_by_code(modules, "adjustment"),
        &[
            "5.1 소득금액조정",
            "5.2 기부금/접대비",
            "5.3 감가상각",
            "5.4 퇴직급여충당금",
            "5.5 대손충당금",
            "5.6 이월결손금",
            "5.7 세액공제/감면",
            "5.8 가산세",
        ],
    );
    assert_children(
        module_by_code(modules, "forms"),
        &[
            "6.1 과세표준 및 세액조정계산서 (별지 제3호)",
            "6.2 100여 종 부속서식",
            "6.3 서식 간 데이터 연동",
            "6.4 미리보기",
        ],
    );
    assert_children(
        module_by_code(modules, "print"),
        &[
            "7.1 PDF 생성 (JasperReports)",
            "7.2 일괄 인쇄",
            "7.3 워터마크/봉인",
        ],
    );
    assert_children(
        module_by_code(modules, "efiling"),
        &[
            "8.1 홈택스 전자신고 레코드 파일 생성",
            "8.2 검증 및 오류 점검",
            "8.3 신고 이력 관리",
        ],
    );
}

fn module_by_code<'a>(modules: &'a [Value], code: &str) -> &'a Value {
    modules
        .iter()
        .find(|module| module["code"] == code)
        .unwrap_or_else(|| panic!("missing module {code}"))
}

fn child_count(module: &Value) -> usize {
    module["children"]
        .as_array()
        .map(|children| children.len())
        .unwrap_or_default()
}

fn assert_children(module: &Value, expected: &[&str]) {
    let actual = module["children"]
        .as_array()
        .expect("children")
        .iter()
        .map(|child| child["display_name"].as_str().expect("display_name"))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
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
