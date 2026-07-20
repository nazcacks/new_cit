use std::{
    env,
    io::{Cursor, Write},
};

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
use uuid::Uuid;
use zip::{write::SimpleFileOptions, ZipWriter};

#[tokio::test]
async fn std_fs_admin_versions_items_integrity_diff_and_permissions_work() {
    let (base_url, _) = spawn_seeded_app().await;
    let super_login = login_as(&base_url, "demo", "admin", "ChangeMe123!").await;
    let super_token = super_login["token"].as_str().unwrap();
    let super_client = authenticated_client(super_token);
    let suffix = Uuid::new_v4().simple().to_string()[..8].to_ascii_uppercase();
    let version_code = format!("STD-B2-{suffix}");
    let industry_type = format!("B2{suffix}");

    let version = post_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions"),
        json!({
            "version_code": version_code,
            "industry_type": industry_type,
            "corp_type": "DOMESTIC",
            "effective_from": "2035-01-01",
            "effective_to": null,
            "nts_doc_ref": "test source",
            "status": "DRAFT",
            "xml_schema_ver": "B2-TEST"
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(version["status"], "DRAFT");
    let version_id = version["id"].as_str().unwrap();

    let fetched = get_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{version_id}"),
    )
    .await;
    assert_eq!(fetched["version_code"], version_code);

    let listed = get_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions?status=DRAFT&industry_type={industry_type}"),
    )
    .await;
    assert!(listed
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == version_id));

    let updated = patch_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{version_id}"),
        json!({"nts_doc_ref": "updated test source"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(updated["nts_doc_ref"], "updated test source");

    create_std_fs_item(
        &super_client,
        &base_url,
        version_id,
        json!({
            "stmt_type": "STD_BS",
            "item_code": "1000",
            "item_name": "Assets",
            "level": 1,
            "account_class": "ASSET",
            "normal_balance": "DEBIT",
            "is_subtotal": true,
            "is_required": true,
            "agg_formula": "1010+1020",
            "xml_field_id": "BS_ASSET_TOTAL",
            "sort_order": 100
        }),
    )
    .await;
    create_std_fs_item(
        &super_client,
        &base_url,
        version_id,
        json!({
            "stmt_type": "STD_BS",
            "item_code": "1010",
            "item_name": "Cash",
            "parent_code": "1000",
            "level": 2,
            "account_class": "ASSET",
            "normal_balance": "DEBIT",
            "sort_order": 110
        }),
    )
    .await;
    create_std_fs_item(
        &super_client,
        &base_url,
        version_id,
        json!({
            "stmt_type": "STD_BS",
            "item_code": "1020",
            "item_name": "Receivables",
            "parent_code": "1000",
            "level": 2,
            "account_class": "ASSET",
            "normal_balance": "DEBIT",
            "sort_order": 120
        }),
    )
    .await;
    create_std_fs_item(
        &super_client,
        &base_url,
        version_id,
        json!({
            "stmt_type": "STD_IS",
            "item_code": "4000",
            "item_name": "Revenue",
            "level": 1,
            "account_class": "REVENUE",
            "normal_balance": "CREDIT",
            "is_subtotal": true,
            "is_required": true,
            "agg_formula": "4010",
            "xml_field_id": "IS_REVENUE_TOTAL",
            "sort_order": 400
        }),
    )
    .await;
    create_std_fs_item(
        &super_client,
        &base_url,
        version_id,
        json!({
            "stmt_type": "STD_IS",
            "item_code": "4010",
            "item_name": "Sales",
            "parent_code": "4000",
            "level": 2,
            "account_class": "REVENUE",
            "normal_balance": "CREDIT",
            "sort_order": 410
        }),
    )
    .await;

    let items = get_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{version_id}/items?include_inactive=true"),
    )
    .await;
    assert_eq!(items.as_array().unwrap().len(), 5);
    let cash_id = item_id_by_code(&items, "1010");
    let cash = get_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/items/{cash_id}"),
    )
    .await;
    assert_eq!(cash["item_name"], "Cash");

    let integrity = get_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{version_id}/integrity"),
    )
    .await;
    assert_eq!(integrity["valid"], true);
    assert_eq!(integrity["error_count"], 0);

    post_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{version_id}/status"),
        json!({"status": "REVIEWED"}),
        StatusCode::OK,
    )
    .await;

    let tenant_admin_login_id = format!("stdta{}", &suffix[..6].to_ascii_lowercase());
    post_json(
        &super_client,
        &format!("{base_url}/api/admin/tenants/demo/users"),
        json!({
            "login_id": tenant_admin_login_id,
            "password": "ChangeMe123!",
            "user_name": "Std FS Tenant Admin",
            "email": null,
            "phone": null,
            "use_2fa": false,
            "status": "ACTIVE",
            "roles": ["TENANT_ADMIN"],
            "customer_access": []
        }),
        StatusCode::CREATED,
    )
    .await;
    let tenant_admin = login_as(&base_url, "demo", &tenant_admin_login_id, "ChangeMe123!").await;
    let tenant_admin_client = authenticated_client(tenant_admin["token"].as_str().unwrap());
    let denied = tenant_admin_client
        .post(format!(
            "{base_url}/api/admin/std-fs/versions/{version_id}/status"
        ))
        .json(&json!({"status": "ACTIVE"}))
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let active = post_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{version_id}/status"),
        json!({"status": "ACTIVE"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(active["status"], "ACTIVE");
    assert!(active["activated_at"].as_str().is_some());

    let clone_code = format!("STD-B2-CLONE-{suffix}");
    let clone_industry = format!("C2{suffix}");
    let cloned = post_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{version_id}/clone"),
        json!({
            "version_code": clone_code,
            "industry_type": clone_industry,
            "corp_type": "DOMESTIC",
            "effective_from": "2035-02-01",
            "effective_to": null,
            "nts_doc_ref": "clone source",
            "xml_schema_ver": "B2-TEST-CLONE"
        }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(cloned["status"], "DRAFT");
    let clone_id = cloned["id"].as_str().unwrap();

    let cloned_items = get_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{clone_id}/items?include_inactive=true"),
    )
    .await;
    assert_eq!(cloned_items.as_array().unwrap().len(), 5);
    let cloned_cash_id = item_id_by_code(&cloned_items, "1010");
    let renamed = patch_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/items/{cloned_cash_id}"),
        json!({"item_name": "Cash Renamed"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(renamed["item_name"], "Cash Renamed");

    let diff = get_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{version_id}/diff/{clone_id}"),
    )
    .await;
    assert_eq!(diff["summary"]["changed_count"], 1);
    let changed = diff["changed"].as_array().unwrap();
    assert!(changed.iter().any(|item| item["item_code"] == "1010"
        && item["changed_fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "item_name")));

    delete_expect(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/items/{cloned_cash_id}"),
        StatusCode::NO_CONTENT,
    )
    .await;
    let cloned_items_after_delete = get_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{clone_id}/items?include_inactive=true"),
    )
    .await;
    assert_eq!(cloned_items_after_delete.as_array().unwrap().len(), 4);
    delete_expect(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{clone_id}"),
        StatusCode::NO_CONTENT,
    )
    .await;

    let bad_version_code = format!("STD-B2-BAD-{suffix}");
    let bad_industry = format!("D2{suffix}");
    let bad_version = post_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions"),
        json!({
            "version_code": bad_version_code,
            "industry_type": bad_industry,
            "corp_type": "DOMESTIC",
            "effective_from": "2035-03-01",
            "effective_to": null
        }),
        StatusCode::CREATED,
    )
    .await;
    let bad_version_id = bad_version["id"].as_str().unwrap();
    create_std_fs_item(
        &super_client,
        &base_url,
        bad_version_id,
        json!({
            "stmt_type": "STD_BS",
            "item_code": "1999",
            "item_name": "Broken Child",
            "parent_code": "NOPE",
            "level": 2
        }),
    )
    .await;
    let bad_integrity = get_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{bad_version_id}/integrity"),
    )
    .await;
    assert_eq!(bad_integrity["valid"], false);
    assert!(bad_integrity["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["code"] == "PARENT_MISSING"));
    delete_expect(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{bad_version_id}"),
        StatusCode::NO_CONTENT,
    )
    .await;

    let template = super_client
        .get(format!("{base_url}/api/admin/std-fs/items/template"))
        .send()
        .await
        .unwrap();
    assert_eq!(template.status(), StatusCode::OK);
    let template = template.text().await.unwrap();
    assert!(template.starts_with("stmt_type,item_code,item_name,parent_code"));

    let import_version_code = format!("STD-B2-IMPORT-{suffix}");
    let import_industry = format!("I2{suffix}");
    let import_version = post_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions"),
        json!({
            "version_code": import_version_code,
            "industry_type": import_industry,
            "corp_type": "DOMESTIC",
            "effective_from": "2035-04-01",
            "effective_to": null
        }),
        StatusCode::CREATED,
    )
    .await;
    let import_version_id = import_version["id"].as_str().unwrap();
    let code_table_csv = "\
stmt_type,item_code,item_name,parent_code,level,account_class,normal_balance,is_subtotal,is_required,agg_formula,xml_field_id,sort_order,is_active
STD_BS,1000,Assets,,1,ASSET,DEBIT,true,true,1010+1020,BS_ASSET_TOTAL,100,true
STD_BS,1010,Cash,1000,2,ASSET,DEBIT,false,false,,BS_CASH,110,true
STD_BS,1020,Receivables,1000,2,ASSET,DEBIT,false,false,,BS_AR,120,true
STD_IS,4000,Revenue,,1,REVENUE,CREDIT,true,true,4010,IS_REVENUE_TOTAL,400,true
STD_IS,4010,Sales,4000,2,REVENUE,CREDIT,false,false,,IS_SALES,410,true
";
    let imported = post_file(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{import_version_id}/items/import"),
        "std-fs-code-table.csv",
        code_table_csv.as_bytes(),
        "text/csv",
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(imported["status"], "IMPORTED");
    assert_eq!(imported["total_rows"], 5);
    assert_eq!(imported["inserted_count"], 5);
    assert_eq!(imported["error_count"], 0);
    assert_eq!(imported["integrity"]["valid"], true);

    let update_csv = "\
stmt_type,item_code,item_name,parent_code,level,account_class,normal_balance,is_subtotal,is_required,agg_formula,xml_field_id,sort_order,is_active
STD_BS,1010,Cash Renamed,1000,2,ASSET,DEBIT,false,false,,BS_CASH_RENAMED,110,true
";
    let updated_import = post_file(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{import_version_id}/import"),
        "std-fs-code-table-update.csv",
        update_csv.as_bytes(),
        "text/csv",
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(updated_import["status"], "IMPORTED");
    assert_eq!(updated_import["updated_count"], 1);
    let imported_items = get_json(
        &super_client,
        &format!(
            "{base_url}/api/admin/std-fs/versions/{import_version_id}/items?include_inactive=true"
        ),
    )
    .await;
    assert_eq!(imported_items.as_array().unwrap().len(), 5);
    assert!(imported_items
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["item_code"] == "1010" && item["item_name"] == "Cash Renamed"));

    let bad_header_csv = "\
stmt_type,item_code,parent_code,level,account_class,normal_balance,is_subtotal,is_required,agg_formula,xml_field_id,sort_order,is_active
STD_BS,1999,1000,2,ASSET,DEBIT,false,false,,BS_BAD,199,true
";
    let bad_header = post_file(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{import_version_id}/items/import"),
        "std-fs-bad-header.csv",
        bad_header_csv.as_bytes(),
        "text/csv",
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(bad_header["status"], "VALIDATION_FAILED");
    assert!(bad_header["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["code"] == "HEADER_MISSING" && issue["field_name"] == "item_name"));

    let invalid_import_version_code = format!("STD-B2-IMPORT-BAD-{suffix}");
    let invalid_import_industry = format!("J2{suffix}");
    let invalid_import_version = post_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions"),
        json!({
            "version_code": invalid_import_version_code,
            "industry_type": invalid_import_industry,
            "corp_type": "DOMESTIC",
            "effective_from": "2035-05-01",
            "effective_to": null
        }),
        StatusCode::CREATED,
    )
    .await;
    let invalid_import_version_id = invalid_import_version["id"].as_str().unwrap();
    let bad_structure_csv = "\
stmt_type,item_code,item_name,parent_code,level,account_class,normal_balance,is_subtotal,is_required,agg_formula,xml_field_id,sort_order,is_active
STD_BS,1000,Assets,,1,ASSET,DEBIT,false,true,,BS_ASSET_TOTAL,100,true
STD_BS,1010,Cash,1000,2,ASSET,DEBIT,false,false,,BS_CASH,110,true
STD_BS,1010,Cash Duplicate,1000,2,ASSET,DEBIT,false,false,,BS_CASH_DUP,111,true
STD_IS,4010,Sales,NOPE,2,REVENUE,CREDIT,false,false,,IS_SALES,410,true
";
    let bad_structure = post_file(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{invalid_import_version_id}/items/import"),
        "std-fs-bad-structure.csv",
        bad_structure_csv.as_bytes(),
        "text/csv",
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(bad_structure["status"], "VALIDATION_FAILED");
    let bad_codes = bad_structure["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|issue| issue["code"].as_str())
        .collect::<Vec<_>>();
    assert!(bad_codes.contains(&"ITEM_CODE_DUPLICATE"));
    assert!(bad_codes.contains(&"PARENT_NOT_SUBTOTAL"));
    assert!(bad_codes.contains(&"PARENT_MISSING"));
    let no_items = get_json(
        &super_client,
        &format!(
            "{base_url}/api/admin/std-fs/versions/{invalid_import_version_id}/items?include_inactive=true"
        ),
    )
    .await;
    assert_eq!(no_items.as_array().unwrap().len(), 0);

    let xlsx_version_code = format!("STD-B2-XLSX-{suffix}");
    let xlsx_industry = format!("X2{suffix}");
    let xlsx_version = post_json(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions"),
        json!({
            "version_code": xlsx_version_code,
            "industry_type": xlsx_industry,
            "corp_type": "DOMESTIC",
            "effective_from": "2035-06-01",
            "effective_to": null
        }),
        StatusCode::CREATED,
    )
    .await;
    let xlsx_version_id = xlsx_version["id"].as_str().unwrap();
    let xlsx = make_xlsx(
        &[
            "stmt_type",
            "item_code",
            "item_name",
            "parent_code",
            "level",
            "account_class",
            "normal_balance",
            "is_subtotal",
            "is_required",
            "agg_formula",
            "xml_field_id",
            "sort_order",
            "is_active",
        ],
        &[
            vec![
                "STD_BS",
                "1000",
                "Assets",
                "",
                "1",
                "ASSET",
                "DEBIT",
                "true",
                "true",
                "1010",
                "BS_ASSET_TOTAL",
                "100",
                "true",
            ],
            vec![
                "STD_BS", "1010", "Cash", "1000", "2", "ASSET", "DEBIT", "false", "false", "",
                "BS_CASH", "110", "true",
            ],
        ],
    );
    let xlsx_imported = post_file(
        &super_client,
        &format!("{base_url}/api/admin/std-fs/versions/{xlsx_version_id}/items/import"),
        "std-fs-code-table.xlsx",
        &xlsx,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(xlsx_imported["status"], "IMPORTED");
    assert_eq!(xlsx_imported["inserted_count"], 2);
    assert_eq!(xlsx_imported["integrity"]["valid"], true);
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

async fn create_std_fs_item(
    client: &Client,
    base_url: &str,
    version_id: &str,
    body: Value,
) -> Value {
    post_json(
        client,
        &format!("{base_url}/api/admin/std-fs/versions/{version_id}/items"),
        body,
        StatusCode::CREATED,
    )
    .await
}

async fn post_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.post(url).json(&body).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn post_file(
    client: &Client,
    url: &str,
    file_name: &str,
    bytes: &[u8],
    mime: &str,
    expected: StatusCode,
) -> Value {
    let form = Form::new().part(
        "file",
        Part::bytes(bytes.to_vec())
            .file_name(file_name.to_string())
            .mime_str(mime)
            .unwrap(),
    );
    let response = client.post(url).multipart(form).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn patch_json(client: &Client, url: &str, body: Value, expected: StatusCode) -> Value {
    let response = client.patch(url).json(&body).send().await.unwrap();
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

async fn delete_expect(client: &Client, url: &str, expected: StatusCode) {
    let response = client.delete(url).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(status, expected, "{text}");
}

fn item_id_by_code(items: &Value, item_code: &str) -> String {
    items
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["item_code"] == item_code)
        .and_then(|item| item["id"].as_str())
        .unwrap()
        .to_string()
}

fn make_xlsx(headers: &[&str], rows: &[Vec<&str>]) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();
    write_zip_file(
        &mut zip,
        "[Content_Types].xml",
        options,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#,
    );
    write_zip_file(
        &mut zip,
        "_rels/.rels",
        options,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#,
    );
    write_zip_file(
        &mut zip,
        "xl/workbook.xml",
        options,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#,
    );
    write_zip_file(
        &mut zip,
        "xl/_rels/workbook.xml.rels",
        options,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#,
    );
    write_zip_file(
        &mut zip,
        "xl/worksheets/sheet1.xml",
        options,
        &sheet_xml(headers, rows),
    );
    zip.finish().unwrap().into_inner()
}

fn write_zip_file(
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    name: &str,
    options: SimpleFileOptions,
    contents: &str,
) {
    zip.start_file(name, options).unwrap();
    zip.write_all(contents.as_bytes()).unwrap();
}

fn sheet_xml(headers: &[&str], rows: &[Vec<&str>]) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    xml.push_str(&xlsx_row(1, headers));
    for (index, row) in rows.iter().enumerate() {
        xml.push_str(&xlsx_row(index + 2, row));
    }
    xml.push_str("</sheetData></worksheet>");
    xml
}

fn xlsx_row(row_no: usize, values: &[&str]) -> String {
    let mut row = format!(r#"<row r="{row_no}">"#);
    for (index, value) in values.iter().enumerate() {
        let cell_ref = format!("{}{}", xlsx_col(index), row_no);
        row.push_str(&format!(
            r#"<c r="{cell_ref}" t="inlineStr"><is><t>{}</t></is></c>"#,
            xml_escape(value)
        ));
    }
    row.push_str("</row>");
    row
}

fn xlsx_col(index: usize) -> String {
    let first = (b'A' + index as u8) as char;
    first.to_string()
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
