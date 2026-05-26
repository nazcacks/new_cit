use std::env;

use axum::serve;
use chrono::{Datelike, Duration, Utc};
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
use tokio::sync::Mutex;
use uuid::Uuid;

static TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn dashboard_work_status_counts_business_year_flow_states() {
    let _guard = TEST_LOCK.lock().await;
    let (base_url, state) = spawn_seeded_app().await;
    let token = login(&base_url).await;
    let client = authenticated_client(&token);

    let tenant_code = format!(
        "dash{}",
        &Uuid::new_v4().simple().to_string()[..8].to_ascii_lowercase()
    );
    post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Dashboard Work Status",
            "biz_reg_no": "9988112345",
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
            "customer_code": "DASH001",
            "customer_name": "Dashboard Customer",
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
    let year = Utc::now().year();

    create_year(&client, &base_url, &tenant_code, customer_id, year).await;
    let validation_waiting =
        create_year(&client, &base_url, &tenant_code, customer_id, year + 1).await;
    mark_in_review_without_approval(&state, &tenant_code, validation_waiting).await;

    let approval_waiting =
        create_year(&client, &base_url, &tenant_code, customer_id, year + 2).await;
    update_status(
        &client,
        &base_url,
        &tenant_code,
        approval_waiting,
        json!({"status": "IN_REVIEW", "actor": "writer01", "approver": "reviewer01"}),
    )
    .await;

    let approved = create_year(&client, &base_url, &tenant_code, customer_id, year + 3).await;
    update_status(
        &client,
        &base_url,
        &tenant_code,
        approved,
        json!({"status": "IN_REVIEW", "actor": "writer01", "approver": "reviewer01"}),
    )
    .await;
    update_status(
        &client,
        &base_url,
        &tenant_code,
        approved,
        json!({"status": "APPROVED", "actor": "reviewer01", "approver": "reviewer01"}),
    )
    .await;

    let filed = create_year(&client, &base_url, &tenant_code, customer_id, year + 4).await;
    update_status(
        &client,
        &base_url,
        &tenant_code,
        filed,
        json!({"status": "IN_REVIEW", "actor": "writer01", "approver": "reviewer01"}),
    )
    .await;
    update_status(
        &client,
        &base_url,
        &tenant_code,
        filed,
        json!({"status": "APPROVED", "actor": "reviewer01", "approver": "reviewer01"}),
    )
    .await;
    update_status(
        &client,
        &base_url,
        &tenant_code,
        filed,
        json!({"status": "FILED", "actor": "reviewer01"}),
    )
    .await;

    let returned = create_year(&client, &base_url, &tenant_code, customer_id, year + 5).await;
    update_status(
        &client,
        &base_url,
        &tenant_code,
        returned,
        json!({"status": "IN_REVIEW", "actor": "writer01", "approver": "reviewer01"}),
    )
    .await;
    update_status(
        &client,
        &base_url,
        &tenant_code,
        returned,
        json!({"status": "DRAFT", "actor": "reviewer01", "comment": "returned for rework"}),
    )
    .await;

    let dashboard = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard"),
    )
    .await;
    let work_status = dashboard["workStatus"].as_array().unwrap();
    assert_eq!(work_status.len(), 5);
    assert_eq!(year_count(work_status, "DRAFT"), 2);
    assert_eq!(year_count(work_status, "IN_REVIEW_VALIDATION"), 1);
    assert_eq!(year_count(work_status, "IN_REVIEW_APPROVAL"), 1);
    assert_eq!(year_count(work_status, "APPROVED"), 1);
    assert_eq!(year_count(work_status, "FILED"), 1);
    assert_eq!(dashboard["rejectedCount"], 1);
    assert_eq!(customer_count(work_status, "DRAFT"), 1);
    assert!(urgent_count(work_status, "FILED") == 0);
}

#[tokio::test]
async fn dashboard_filing_deadlines_are_d30_sorted_and_exclude_filed() {
    let _guard = TEST_LOCK.lock().await;
    let (base_url, _state) = spawn_seeded_app().await;
    let token = login(&base_url).await;
    let client = authenticated_client(&token);

    let tenant_code = format!(
        "due{}",
        &Uuid::new_v4().simple().to_string()[..9].to_ascii_lowercase()
    );
    post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Dashboard Deadlines",
            "biz_reg_no": "9988112345",
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
            "customer_code": "DUE001",
            "customer_name": "Deadline Customer",
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
    let year = Utc::now().year() + 20;

    let notice =
        create_year_due_in_days(&client, &base_url, &tenant_code, customer_id, year, 20).await;
    let critical =
        create_year_due_in_days(&client, &base_url, &tenant_code, customer_id, year + 1, 5).await;
    let warning =
        create_year_due_in_days(&client, &base_url, &tenant_code, customer_id, year + 2, 10).await;
    create_year_due_in_days(&client, &base_url, &tenant_code, customer_id, year + 3, 31).await;
    let filed =
        create_year_due_in_days(&client, &base_url, &tenant_code, customer_id, year + 4, 3).await;
    update_status(
        &client,
        &base_url,
        &tenant_code,
        filed,
        json!({"status": "IN_REVIEW", "actor": "writer01", "approver": "reviewer01"}),
    )
    .await;
    update_status(
        &client,
        &base_url,
        &tenant_code,
        filed,
        json!({"status": "APPROVED", "actor": "reviewer01", "approver": "reviewer01"}),
    )
    .await;
    update_status(
        &client,
        &base_url,
        &tenant_code,
        filed,
        json!({"status": "FILED", "actor": "reviewer01"}),
    )
    .await;

    let deadlines = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard/filing-deadlines?withinDays=30"),
    )
    .await;
    assert_eq!(deadlines["totalCount"], 3);
    let rows = deadlines["deadlines"].as_array().unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| row["businessYearId"].as_i64().unwrap())
            .collect::<Vec<_>>(),
        vec![critical, warning, notice]
    );
    assert_eq!(rows[0]["daysRemaining"], 5);
    assert_eq!(rows[0]["urgencyLevel"], "CRITICAL");
    assert_eq!(rows[1]["daysRemaining"], 10);
    assert_eq!(rows[1]["urgencyLevel"], "WARNING");
    assert_eq!(rows[2]["daysRemaining"], 20);
    assert_eq!(rows[2]["urgencyLevel"], "NOTICE");
    assert!(rows.iter().all(|row| row["businessYearId"] != filed));

    let summary = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard"),
    )
    .await;
    assert_eq!(summary["filingDeadlines"]["totalCount"], 3);
}

#[tokio::test]
async fn dashboard_notifications_include_due_buckets_and_mark_read() {
    let _guard = TEST_LOCK.lock().await;
    let (base_url, _state) = spawn_seeded_app().await;
    let token = login(&base_url).await;
    let client = authenticated_client(&token);

    let tenant_code = format!(
        "ntf{}",
        &Uuid::new_v4().simple().to_string()[..9].to_ascii_lowercase()
    );
    post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Dashboard Notifications",
            "biz_reg_no": "9988112345",
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
            "customer_code": "NTF001",
            "customer_name": "Notification Customer",
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
    let year = Utc::now().year() + 40;

    create_year_due_in_days(&client, &base_url, &tenant_code, customer_id, year, 30).await;
    create_year_due_in_days(&client, &base_url, &tenant_code, customer_id, year + 1, 7).await;
    let due_today =
        create_year_due_in_days(&client, &base_url, &tenant_code, customer_id, year + 2, 0).await;

    let summary = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard/notifications?limit=20"),
    )
    .await;
    assert!(summary["unreadCount"].as_i64().unwrap() >= 3, "{summary}");
    assert_eq!(summary["limit"], 20);
    assert_eq!(summary["unreadOnly"], false);
    let notifications = summary["notifications"].as_array().unwrap();
    let buckets = notifications
        .iter()
        .filter_map(|item| item["dueBucket"].as_str())
        .collect::<Vec<_>>();
    assert!(buckets.contains(&"D-30"), "{summary}");
    assert!(buckets.contains(&"D-7"), "{summary}");
    assert!(buckets.contains(&"D-Day"), "{summary}");

    let dday = notifications
        .iter()
        .find(|item| item["dueBucket"] == "D-Day" && item["byId"].as_i64() == Some(due_today))
        .unwrap_or_else(|| panic!("D-Day notification missing in {summary}"));
    assert_eq!(dday["notificationType"], "DEADLINE_DDAY");
    assert_eq!(dday["severity"], "ERROR");
    assert!(dday["routeKey"].as_str().unwrap().starts_with("ws/"));
    let notification_id = dday["notificationId"].as_i64().unwrap();
    let unread_before = summary["unreadCount"].as_i64().unwrap();

    let read = patch_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/notifications/{notification_id}"),
        json!({"status": "READ"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(read["status"], "READ");
    assert!(read["read_at"].is_string());

    let after = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard/notifications?limit=20"),
    )
    .await;
    assert_eq!(after["unreadCount"].as_i64().unwrap(), unread_before - 1);

    let unread_only = get_json(
        &client,
        &format!(
            "{base_url}/api/tenants/{tenant_code}/dashboard/notifications?limit=20&unreadOnly=true"
        ),
    )
    .await;
    assert!(unread_only["notifications"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["status"] == "UNREAD"));
}

#[tokio::test]
async fn dashboard_inline_approval_actions_update_queue_status_and_notifications() {
    let _guard = TEST_LOCK.lock().await;
    let (base_url, _state) = spawn_seeded_app().await;
    let token = login(&base_url).await;
    let client = authenticated_client(&token);

    let tenant_code = format!(
        "act{}",
        &Uuid::new_v4().simple().to_string()[..9].to_ascii_lowercase()
    );
    post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Dashboard Approval Actions",
            "biz_reg_no": "9988112345",
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
            "customer_code": "ACT001",
            "customer_name": "Approval Action Customer",
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
    let year = Utc::now().year() + 60;
    let approve_by =
        create_year_due_in_days(&client, &base_url, &tenant_code, customer_id, year, 90).await;
    let reject_by =
        create_year_due_in_days(&client, &base_url, &tenant_code, customer_id, year + 1, 91).await;

    for by_id in [approve_by, reject_by] {
        update_status(
            &client,
            &base_url,
            &tenant_code,
            by_id,
            json!({
                "status": "IN_REVIEW",
                "actor": "writer01",
                "approver": "reviewer01",
                "comment": "dashboard action test submit"
            }),
        )
        .await;
    }

    let queue_before = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/workflow/queue?assignee=reviewer01"),
    )
    .await;
    assert_eq!(queue_before.as_array().unwrap().len(), 2);
    let dashboard_before = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard"),
    )
    .await;
    let work_status_before = dashboard_before["workStatus"].as_array().unwrap();
    assert_eq!(year_count(work_status_before, "IN_REVIEW_APPROVAL"), 2);
    assert_eq!(year_count(work_status_before, "APPROVED"), 0);
    let unread_before = dashboard_before["unread_notifications"].as_i64().unwrap();

    update_status(
        &client,
        &base_url,
        &tenant_code,
        approve_by,
        json!({
            "status": "APPROVED",
            "actor": "reviewer01",
            "approver": "reviewer01",
            "comment": "dashboard inline approval"
        }),
    )
    .await;
    update_status(
        &client,
        &base_url,
        &tenant_code,
        reject_by,
        json!({
            "status": "DRAFT",
            "actor": "reviewer01",
            "approver": "reviewer01",
            "comment": "dashboard inline rejection"
        }),
    )
    .await;

    let approved_workflow = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{approve_by}/workflow"),
    )
    .await;
    assert_eq!(approved_workflow["business_year"]["status"], "APPROVED");
    assert!(approved_workflow["approval_lines"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line["status"] == "APPROVED"));

    let rejected_workflow = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{reject_by}/workflow"),
    )
    .await;
    assert_eq!(rejected_workflow["business_year"]["status"], "DRAFT");
    assert!(rejected_workflow["approval_lines"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line["status"] == "RETURNED"));

    let queue_after = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/workflow/queue?assignee=reviewer01"),
    )
    .await;
    assert!(queue_after.as_array().unwrap().is_empty());

    let dashboard_after = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard"),
    )
    .await;
    let work_status_after = dashboard_after["workStatus"].as_array().unwrap();
    assert_eq!(year_count(work_status_after, "IN_REVIEW_APPROVAL"), 0);
    assert_eq!(year_count(work_status_after, "APPROVED"), 1);
    assert_eq!(year_count(work_status_after, "DRAFT"), 1);
    assert_eq!(dashboard_after["rejectedCount"], 1);
    assert_eq!(
        dashboard_after["unread_notifications"].as_i64().unwrap(),
        unread_before + 2
    );

    let notification_summary = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard/notifications?limit=10"),
    )
    .await;
    let titles = notification_summary["notifications"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["title"].as_str())
        .collect::<Vec<_>>();
    assert!(
        titles.contains(&"Approval completed"),
        "{notification_summary}"
    );
    assert!(
        titles.contains(&"Approval returned"),
        "{notification_summary}"
    );
}

#[tokio::test]
async fn dashboard_recent_activities_returns_latest_15_read_only_with_labels_and_routes() {
    let _guard = TEST_LOCK.lock().await;
    let (base_url, _state) = spawn_seeded_app().await;
    let token = login(&base_url).await;
    let client = authenticated_client(&token);

    let tenant_code = format!(
        "actv{}",
        &Uuid::new_v4().simple().to_string()[..8].to_ascii_lowercase()
    );
    post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Dashboard Recent Activity",
            "biz_reg_no": "9988112345",
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
            "customer_code": "ACT001",
            "customer_name": "Activity Customer",
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
    let base_year = Utc::now().year() + 20;
    let mut years = Vec::new();
    for offset in 0..16 {
        years.push(
            create_year_due_in_days(
                &client,
                &base_url,
                &tenant_code,
                customer_id,
                base_year + offset,
                90 + i64::from(offset),
            )
            .await,
        );
    }
    update_status(
        &client,
        &base_url,
        &tenant_code,
        years[0],
        json!({
            "status": "IN_REVIEW",
            "actor": "writer01",
            "approver": "reviewer01",
            "comment": "recent activity submit"
        }),
    )
    .await;

    let audit_before = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/audit-logs"),
    )
    .await;
    let audit_count_before = audit_before.as_array().unwrap().len();
    let recent = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard/recent-activities?limit=15"),
    )
    .await;
    let audit_after = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/audit-logs"),
    )
    .await;

    assert_eq!(audit_after.as_array().unwrap().len(), audit_count_before);
    assert_eq!(recent["limit"], 15);
    assert_eq!(recent["totalCount"], audit_count_before as i64);
    let activities = recent["activities"].as_array().unwrap();
    assert_eq!(activities.len(), 15);
    assert!(activities
        .windows(2)
        .all(|pair| pair[0]["auditId"].as_i64().unwrap() > pair[1]["auditId"].as_i64().unwrap()));

    let first = &activities[0];
    assert_eq!(first["activityType"], "REVIEW_REQUESTED");
    assert_eq!(first["typeLabel"], "결재 요청");
    assert_eq!(first["description"], "결재 요청 (IN_REVIEW 전환)");
    assert_eq!(first["routeKey"], "ws/appr:inbox");
    assert_eq!(first["byId"], years[0]);
    assert_eq!(first["customerId"], customer_id);
    assert_eq!(first["customerName"], "Activity Customer");
    assert_eq!(first["fiscalYear"], base_year);
    assert_eq!(first["actorLoginId"], "writer01");
    assert_eq!(first["actorName"], "writer01");
    assert!(first["occurredAt"].as_str().unwrap().contains('T'));
    assert!(activities.iter().any(|item| {
        item["activityType"] == "BUSINESS_YEAR_CREATED"
            && item["typeLabel"] == "사업연도 생성"
            && item["routeKey"] == "ws/start:snapshot"
    }));
}

#[tokio::test]
async fn dashboard_tax_burden_kpi_returns_recent_5_year_weighted_trend() {
    let _guard = TEST_LOCK.lock().await;
    let (base_url, state) = spawn_seeded_app().await;
    let token = login(&base_url).await;
    let client = authenticated_client(&token);

    let tenant_code = format!(
        "kpi{}",
        &Uuid::new_v4().simple().to_string()[..9].to_ascii_lowercase()
    );
    post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Dashboard KPI Tax Burden",
            "biz_reg_no": "9988112345",
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
            "customer_code": "KPI001",
            "customer_name": "KPI Customer",
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
    let samples = [
        (2021, 1_000_000_i64, 100_000_i64),
        (2022, 2_000_000, 200_000),
        (2023, 3_000_000, 450_000),
        (2024, 4_000_000, 800_000),
        (2025, 5_000_000, 500_000),
        (2026, 6_000_000, 1_200_000),
    ];
    for (index, (year_label, taxable_income, total_tax_due)) in samples.iter().enumerate() {
        let by_id = create_year_due_in_days(
            &client,
            &base_url,
            &tenant_code,
            customer_id,
            *year_label,
            180 + index as i64,
        )
        .await;
        mark_business_year_status(&state, &tenant_code, by_id, "APPROVED").await;
        insert_tax_burden_adjustments(&state, &tenant_code, by_id, *taxable_income, *total_tax_due)
            .await;
    }

    let audit_before = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/audit-logs"),
    )
    .await;
    let kpi = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard/kpi/tax-burden?years=5"),
    )
    .await;
    let audit_after = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/audit-logs"),
    )
    .await;

    assert_eq!(audit_after, audit_before);
    assert_eq!(kpi["years"], 5);
    assert_eq!(kpi["customerId"], Value::Null);
    assert_eq!(kpi["totalTaxableIncome"], 20_000_000);
    assert_eq!(kpi["totalTaxDue"], 3_150_000);
    assert_eq!(kpi["averageEffectiveTaxRateBps"], 1575);
    let trend = kpi["trend"].as_array().unwrap();
    assert_eq!(trend.len(), 5);
    assert_eq!(
        trend
            .iter()
            .map(|item| item["fiscalYear"].as_i64().unwrap())
            .collect::<Vec<_>>(),
        vec![2022, 2023, 2024, 2025, 2026]
    );
    assert_eq!(trend[0]["effectiveTaxRateBps"], 1000);
    assert_eq!(trend[2]["effectiveTaxRateBps"], 2000);
    assert_eq!(trend[4]["taxableIncome"], 6_000_000);
    assert_eq!(trend[4]["totalTaxDue"], 1_200_000);
    assert_eq!(trend[4]["customerCount"], 1);

    let filtered = get_json(
        &client,
        &format!(
            "{base_url}/api/tenants/{tenant_code}/dashboard/kpi/tax-burden?years=5&customerId={customer_id}"
        ),
    )
    .await;
    assert_eq!(filtered["customerId"], customer_id);
    assert_eq!(filtered["trend"].as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn dashboard_industry_distribution_and_loss_expiry_kpis_use_customer_and_loss_data() {
    let _guard = TEST_LOCK.lock().await;
    let (base_url, state) = spawn_seeded_app().await;
    let token = login(&base_url).await;
    let client = authenticated_client(&token);

    let tenant_code = format!(
        "kpi2{}",
        &Uuid::new_v4().simple().to_string()[..8].to_ascii_lowercase()
    );
    post_json(
        &client,
        &format!("{base_url}/api/tenants"),
        json!({
            "tenant_code": tenant_code,
            "tenant_name": "Dashboard KPI Industry Loss",
            "biz_reg_no": "9988112345",
            "contract_start": "2026-01-01",
            "contract_end": null,
            "max_users": 10
        }),
        StatusCode::CREATED,
    )
    .await;
    let c1 = create_customer_with_industry(&client, &base_url, &tenant_code, "K2A", "62010").await;
    let c2 = create_customer_with_industry(&client, &base_url, &tenant_code, "K2B", "62010").await;
    let c3 = create_customer_with_industry(&client, &base_url, &tenant_code, "K2C", "47110").await;
    let c4 = create_customer_with_industry(&client, &base_url, &tenant_code, "K2D", "").await;
    let current_year = Utc::now().year();
    insert_loss_carryforward(
        &state,
        &tenant_code,
        c1,
        current_year - 3,
        100_000,
        current_year,
    )
    .await;
    insert_loss_carryforward(
        &state,
        &tenant_code,
        c1,
        current_year - 2,
        300_000,
        current_year + 1,
    )
    .await;
    insert_loss_carryforward(
        &state,
        &tenant_code,
        c2,
        current_year - 1,
        200_000,
        current_year + 1,
    )
    .await;
    insert_loss_carryforward(
        &state,
        &tenant_code,
        c3,
        current_year,
        400_000,
        current_year + 3,
    )
    .await;

    let audit_before = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/audit-logs"),
    )
    .await;
    let industry = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard/kpi/industry-distribution"),
    )
    .await;
    let loss = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/dashboard/kpi/loss-expiry?years=3"),
    )
    .await;
    let audit_after = get_json(
        &client,
        &format!("{base_url}/api/tenants/{tenant_code}/audit-logs"),
    )
    .await;

    assert_eq!(audit_after, audit_before);
    assert_eq!(industry["totalCustomers"], 4);
    let industries = industry["industries"].as_array().unwrap();
    assert_eq!(industries.len(), 3);
    let software = industries
        .iter()
        .find(|item| item["industryCode"] == "62010")
        .unwrap();
    assert_eq!(software["industryName"], "Software development");
    assert_eq!(software["customerCount"], 2);
    assert_eq!(software["percentageBps"], 5000);
    assert_eq!(software["percentagePct"], 50.0);
    assert!(industries
        .iter()
        .any(|item| item["industryCode"] == "UNSPECIFIED" && item["customerCount"] == 1));

    assert_eq!(loss["years"], 3);
    assert_eq!(loss["totalAmount"], 600_000);
    assert_eq!(loss["totalCustomerCount"], 2);
    assert_eq!(loss["totalLossCount"], 3);
    let buckets = loss["buckets"].as_array().unwrap();
    assert_eq!(buckets.len(), 2);
    assert_eq!(buckets[0]["expiresYear"], current_year);
    assert_eq!(buckets[0]["totalAmount"], 100_000);
    assert_eq!(buckets[0]["customerCount"], 1);
    assert_eq!(buckets[0]["lossCount"], 1);
    assert_eq!(buckets[1]["expiresYear"], current_year + 1);
    assert_eq!(buckets[1]["totalAmount"], 500_000);
    assert_eq!(buckets[1]["customerCount"], 2);
    assert_eq!(buckets[1]["lossCount"], 2);

    let _ = c4;
}

#[tokio::test]
async fn dashboard_scope_filters_assigned_customers_and_blocks_other_tenants() {
    let _guard = TEST_LOCK.lock().await;
    let (base_url, _state) = spawn_seeded_app().await;
    let admin_token = login(&base_url).await;
    let admin_client = authenticated_client(&admin_token);
    let suffix = &Uuid::new_v4().simple().to_string()[..8].to_ascii_lowercase();
    let tenant_a = format!("sca{suffix}");
    let tenant_b = format!("scb{suffix}");

    for tenant_code in [&tenant_a, &tenant_b] {
        post_json(
            &admin_client,
            &format!("{base_url}/api/tenants"),
            json!({
                "tenant_code": tenant_code,
                "tenant_name": format!("Scoped Dashboard {tenant_code}"),
                "biz_reg_no": "9988112345",
                "contract_start": "2026-01-01",
                "contract_end": null,
                "max_users": 10
            }),
            StatusCode::CREATED,
        )
        .await;
    }

    let assigned_customer =
        create_customer_with_industry(&admin_client, &base_url, &tenant_a, "SCA1", "62010").await;
    let hidden_customer =
        create_customer_with_industry(&admin_client, &base_url, &tenant_a, "SCA2", "47110").await;
    let other_tenant_customer =
        create_customer_with_industry(&admin_client, &base_url, &tenant_b, "SCB1", "62010").await;
    let base_year = Utc::now().year() + 80;
    create_year_due_in_days(
        &admin_client,
        &base_url,
        &tenant_a,
        assigned_customer,
        base_year,
        10,
    )
    .await;
    create_year_due_in_days(
        &admin_client,
        &base_url,
        &tenant_a,
        hidden_customer,
        base_year + 1,
        11,
    )
    .await;
    create_year_due_in_days(
        &admin_client,
        &base_url,
        &tenant_b,
        other_tenant_customer,
        base_year,
        10,
    )
    .await;

    let login_id = format!("scoped{suffix}");
    post_json(
        &admin_client,
        &format!("{base_url}/api/admin/tenants/{tenant_a}/users"),
        json!({
            "login_id": login_id,
            "password": "ChangeMe123!",
            "user_name": "Scoped Dashboard Expert",
            "use_2fa": false,
            "roles": ["TAX_EXPERT"],
            "customer_access": [{
                "customer_id": assigned_customer,
                "access_level": "OWNER",
                "is_primary": true,
                "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT"]
            }]
        }),
        StatusCode::CREATED,
    )
    .await;

    let scoped_token = login_as(&base_url, &tenant_a, &login_id, "ChangeMe123!").await;
    let scoped_client = authenticated_client(&scoped_token);
    let dashboard = get_json(
        &scoped_client,
        &format!("{base_url}/api/tenants/{tenant_a}/dashboard"),
    )
    .await;
    assert_eq!(dashboard["tenant_code"].as_str(), Some(tenant_a.as_str()));
    assert_eq!(dashboard["customer_count"], 1);
    assert_eq!(dashboard["business_year_count"], 1);
    assert_eq!(
        year_count(dashboard["workStatus"].as_array().unwrap(), "DRAFT"),
        1
    );

    let deadlines = get_json(
        &scoped_client,
        &format!("{base_url}/api/tenants/{tenant_a}/dashboard/filing-deadlines?withinDays=30"),
    )
    .await;
    assert_eq!(deadlines["totalCount"], 1);
    assert_eq!(
        deadlines["deadlines"][0]["customerId"].as_i64(),
        Some(assigned_customer)
    );

    let forbidden = scoped_client
        .get(format!("{base_url}/api/tenants/{tenant_b}/dashboard"))
        .send()
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

async fn create_year(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    customer_id: i64,
    year_label: i32,
) -> i64 {
    let today = Utc::now().date_naive();
    let end_date = today + Duration::days(5);
    let value = post_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
        json!({
            "customer_id": customer_id,
            "year_label": year_label,
            "start_date": today,
            "end_date": end_date
        }),
        StatusCode::CREATED,
    )
    .await;
    value["by_id"].as_i64().unwrap()
}

async fn create_customer_with_industry(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    code: &str,
    industry_code: &str,
) -> i64 {
    let industry = if industry_code.is_empty() {
        Value::Null
    } else {
        json!(industry_code)
    };
    let value = post_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/customers"),
        json!({
            "customer_code": code,
            "customer_name": format!("KPI {code} Customer"),
            "biz_reg_no": "2208112345",
            "corp_reg_no": null,
            "industry_code": industry,
            "is_sme": true,
            "work_scopes": ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"]
        }),
        StatusCode::CREATED,
    )
    .await;
    value["customer_id"].as_i64().unwrap()
}

async fn create_year_due_in_days(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    customer_id: i64,
    year_label: i32,
    due_in_days: i64,
) -> i64 {
    let today = Utc::now().date_naive();
    let end_date = today + Duration::days(due_in_days);
    let value = post_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years"),
        json!({
            "customer_id": customer_id,
            "year_label": year_label,
            "start_date": today,
            "end_date": end_date
        }),
        StatusCode::CREATED,
    )
    .await;
    value["by_id"].as_i64().unwrap()
}

async fn update_status(
    client: &Client,
    base_url: &str,
    tenant_code: &str,
    by_id: i64,
    body: Value,
) {
    post_json(
        client,
        &format!("{base_url}/api/tenants/{tenant_code}/business-years/{by_id}/status"),
        body,
        StatusCode::OK,
    )
    .await;
}

async fn mark_in_review_without_approval(state: &AppState, tenant_code: &str, by_id: i64) {
    let schema = format!("tenant_{tenant_code}");
    sqlx::query(&format!(
        "UPDATE {schema}.business_years SET status = 'IN_REVIEW', updated_at = NOW() WHERE by_id = $1"
    ))
    .bind(by_id)
    .execute(&state.pool)
    .await
    .expect("mark validation waiting year");
}

async fn mark_business_year_status(state: &AppState, tenant_code: &str, by_id: i64, status: &str) {
    let schema = format!("tenant_{tenant_code}");
    sqlx::query(&format!(
        "UPDATE {schema}.business_years SET status = $2, updated_at = NOW() WHERE by_id = $1"
    ))
    .bind(by_id)
    .bind(status)
    .execute(&state.pool)
    .await
    .expect("mark business year status");
}

async fn insert_tax_burden_adjustments(
    state: &AppState,
    tenant_code: &str,
    by_id: i64,
    taxable_income: i64,
    total_tax_due: i64,
) {
    let schema = format!("tenant_{tenant_code}");
    sqlx::query(&format!(
        r#"
        INSERT INTO {schema}.tax_adjustments (
            by_id, adj_category, adj_code, amount, direction, description, metadata, status
        )
        VALUES
            ($1, 'B12_TAX_AMOUNT', 'TAXABLE_INCOME', $2, 'INFO', 'KPI taxable income', '{{}}'::jsonb, 'POSTED'),
            ($1, 'B12_TAX_AMOUNT', 'TOTAL_TAX_DUE', $3, 'INFO', 'KPI total tax due', '{{}}'::jsonb, 'POSTED')
        "#
    ))
    .bind(by_id)
    .bind(taxable_income)
    .bind(total_tax_due)
    .execute(&state.pool)
    .await
    .expect("insert tax burden adjustments");
}

async fn insert_loss_carryforward(
    state: &AppState,
    tenant_code: &str,
    customer_id: i64,
    origin_year: i32,
    remaining_amount: i64,
    expires_year: i32,
) {
    let schema = format!("tenant_{tenant_code}");
    sqlx::query(&format!(
        r#"
        INSERT INTO {schema}.carryforward_loss (
            customer_id, origin_year, original_amount, used_amount,
            expired_amount, remaining_amount, expires_year
        )
        VALUES ($1, $2, $3, 0, 0, $3, $4)
        "#
    ))
    .bind(customer_id)
    .bind(origin_year)
    .bind(remaining_amount)
    .bind(expires_year)
    .execute(&state.pool)
    .await
    .expect("insert carryforward loss");
}

fn year_count(work_status: &[Value], status: &str) -> i64 {
    find_status(work_status, status)["yearCount"]
        .as_i64()
        .unwrap()
}

fn customer_count(work_status: &[Value], status: &str) -> i64 {
    find_status(work_status, status)["customerCount"]
        .as_i64()
        .unwrap()
}

fn urgent_count(work_status: &[Value], status: &str) -> i64 {
    find_status(work_status, status)["urgentCount"]
        .as_i64()
        .unwrap()
}

fn find_status<'a>(work_status: &'a [Value], status: &str) -> &'a Value {
    work_status
        .iter()
        .find(|item| item["status"] == status)
        .unwrap_or_else(|| panic!("{status} not found in {work_status:?}"))
}

async fn spawn_seeded_app() -> (String, AppState) {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let pool = db::connect(&database_url).await.expect("db connection");
    db::migrate(&pool).await.expect("migrations");
    seed::run_demo_seed(
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
    let app = router(state.clone());
    tokio::spawn(async move {
        serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), state)
}

async fn login(base_url: &str) -> String {
    login_as(base_url, "demo", "admin", "ChangeMe123!").await
}

async fn login_as(base_url: &str, tenant_code: &str, login_id: &str, password: &str) -> String {
    let client = Client::new();
    let auth = post_json(
        &client,
        &format!("{base_url}/api/auth/login"),
        json!({"tenant_code": tenant_code, "login_id": login_id, "password": password}),
        StatusCode::OK,
    )
    .await;
    auth["token"].as_str().unwrap().to_string()
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
