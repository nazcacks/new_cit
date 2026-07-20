use std::env;

use axum::serve;
use cit_system::{
    db, router,
    seed::{self, DemoSeedOptions},
    AppState, Config,
};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    multipart::{Form, Part},
    Client, StatusCode,
};
use serde_json::{json, Value};
use tokio::net::TcpListener;

#[tokio::test]
async fn asset_phase_c_carries_forward_previews_depreciation_and_reconciles_bs() {
    let (base_url, _) = spawn_seeded_app().await;
    let login = login_as(&base_url, "demo", "admin", "ChangeMe123!").await;
    let client = authenticated_client(login["token"].as_str().unwrap());

    let customer = post_json(
        &client,
        &format!("{base_url}/api/tenants/demo/customers"),
        json!({
            "customer_code": "ASSETC01",
            "customer_name": "Asset Phase C",
            "biz_reg_no": "2208123456",
            "corp_reg_no": null,
            "industry_code": "62010",
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"]
        }),
        StatusCode::CREATED,
    )
    .await;
    let customer_id = customer["customer_id"].as_i64().unwrap();
    let source_year = post_json(
        &client,
        &format!("{base_url}/api/tenants/demo/business-years"),
        json!({
            "customer_id": customer_id,
            "year_label": 2025,
            "start_date": "2025-01-01",
            "end_date": "2025-12-31"
        }),
        StatusCode::CREATED,
    )
    .await;
    let target_year = post_json(
        &client,
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
    let source_by_id = source_year["by_id"].as_i64().unwrap();
    let target_by_id = target_year["by_id"].as_i64().unwrap();

    let source_assets = "\
asset_code,asset_name,asset_category,acquisition_date,acquisition_cost,useful_life_years,depr_method,residual_value,accumulated_depr_prior,acct_depr_current
MACH1,CNC machine,MACHINERY,2025-01-01,1200000,5,SL,0,200000,100000
SW1,ERP license,INTANGIBLE,2025-01-01,300000,5,SL,0,0,0
";
    post_csv_file(
        &client,
        &format!(
            "{base_url}/api/tenants/demo/business-years/{source_by_id}/tax-data/assets/import"
        ),
        "source-assets.csv",
        source_assets,
        StatusCode::CREATED,
    )
    .await;

    let carried = post_json(
        &client,
        &format!("{base_url}/api/workspace/{target_by_id}/assets/carry-forward"),
        json!({"source_by_id": source_by_id}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(carried["source_by_id"].as_i64(), Some(source_by_id));
    assert_eq!(carried["copied_count"].as_u64(), Some(2));
    assert_eq!(carried["skipped_count"].as_u64(), Some(0));

    let target_assets = get_json(
        &client,
        &format!("{base_url}/api/workspace/{target_by_id}/assets"),
    )
    .await;
    let mach = find_asset(&target_assets, "MACH1");
    assert_eq!(mach["accumulated_depr_prior"].as_i64(), Some(300000));
    assert_eq!(mach["acct_depr_current"].as_i64(), Some(0));
    assert!(mach["prev_year_asset_id"].as_i64().unwrap_or_default() > 0);

    let preview = get_json(
        &client,
        &format!("{base_url}/api/workspace/{target_by_id}/assets/depr-preview"),
    )
    .await;
    let mach_preview = find_preview_row(&preview, "MACH1");
    assert_eq!(mach_preview["tax_depr_limit"].as_i64(), Some(240000));
    assert_eq!(mach_preview["depr_excess"].as_i64(), Some(0));
    assert_eq!(mach_preview["depr_shortfall"].as_i64(), Some(0));

    let target_fs = "\
statement_type,account_code,account_name,debit,credit,standard_account_code,standard_account_name
BS,MACH_GROSS,Machinery,1200000,0,STD_MACHINERY,Machinery
BS,SW_GROSS,Software,300000,0,STD_INTANGIBLE,Intangible assets
BS,ACC_DEP,Accumulated depreciation,0,300000,ACCUM_DEPR,Accumulated depreciation
BS,AP,Accounts payable,0,900000,STD_PAYABLE,Accounts payable
BS,CAP,Capital stock,0,300000,STD_CAPITAL,Capital stock
";
    post_csv_file(
        &client,
        &format!(
            "{base_url}/api/tenants/demo/business-years/{target_by_id}/tax-data/financial-statements/import"
        ),
        "target-fs.csv",
        target_fs,
        StatusCode::CREATED,
    )
    .await;

    post_json(
        &client,
        &format!("{base_url}/api/workspace/{target_by_id}/std-fs/mappings/bulk"),
        json!({
            "mappings": [
                {"account_code": "MACH_GROSS", "std_fs_item_code": "1524"},
                {"account_code": "SW_GROSS", "std_fs_item_code": "1530"},
                {"account_code": "AP", "std_fs_item_code": "2010"},
                {"account_code": "CAP", "std_fs_item_code": "3010"}
            ]
        }),
        StatusCode::OK,
    )
    .await;

    let reconcile = get_json(
        &client,
        &format!("{base_url}/api/workspace/{target_by_id}/assets/bs-reconcile"),
    )
    .await;
    assert_eq!(reconcile["valid"].as_bool(), Some(true), "{reconcile}");
    assert_reconcile_passed(&reconcile, "CHK_PPE_COST");
    assert_reconcile_passed(&reconcile, "CHK_ACCUM_DEPR");
    assert_reconcile_passed(&reconcile, "CHK_INTANGIBLE");

    let b4 = post_json(
        &client,
        &format!("{base_url}/api/tenants/demo/business-years/{target_by_id}/adjustments/assets/B4"),
        json!({}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(b4["details"]["total_tax_limit"].as_i64(), Some(300000));

    let after_b4_assets = get_json(
        &client,
        &format!("{base_url}/api/workspace/{target_by_id}/assets"),
    )
    .await;
    let mach_after_b4 = find_asset(&after_b4_assets, "MACH1");
    assert_eq!(mach_after_b4["tax_depr_limit"].as_i64(), Some(240000));
    assert_eq!(mach_after_b4["depr_excess"].as_i64(), Some(0));
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
    assert!(status.is_success(), "{text}");
    serde_json::from_str(&text).unwrap()
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

fn find_asset<'a>(assets: &'a Value, asset_code: &str) -> &'a Value {
    assets
        .as_array()
        .expect("assets")
        .iter()
        .find(|asset| asset["asset_code"] == asset_code)
        .unwrap_or_else(|| panic!("missing asset {asset_code}: {assets}"))
}

fn find_preview_row<'a>(preview: &'a Value, asset_code: &str) -> &'a Value {
    preview["rows"]
        .as_array()
        .expect("preview rows")
        .iter()
        .find(|asset| asset["asset_code"] == asset_code)
        .unwrap_or_else(|| panic!("missing preview row {asset_code}: {preview}"))
}

fn assert_reconcile_passed(reconcile: &Value, rule_code: &str) {
    let issue = reconcile["issues"]
        .as_array()
        .expect("reconcile issues")
        .iter()
        .find(|issue| issue["rule_code"] == rule_code)
        .unwrap_or_else(|| panic!("missing reconcile issue {rule_code}: {reconcile}"));
    assert_eq!(issue["passed"].as_bool(), Some(true), "{issue}");
}
