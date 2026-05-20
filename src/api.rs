use axum::{
    body::Body,
    extract::{Extension, Multipart, Path, Query, State},
    http::{
        header::AUTHORIZATION,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderMap, HeaderName, HeaderValue, Method, Request, Response, StatusCode,
    },
    middleware::{self, Next},
    routing::{get, patch, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

use crate::{
    admin, auth,
    domain::{
        AssetBasedAdjustmentRequest, AuthUser, CalculateAdjustmentRequest,
        CreateAccessDelegationRequest, CreateAccountMappingRequest,
        CreateAdjustmentAttachmentRequest, CreateAdminUserRequest, CreateBusinessYearRequest,
        CreateCustomerRequest, CreateFormRelationshipRequest, CreateFormVersionRequest,
        CreateIncomeAdjustmentRequest, CreateLawAmendmentRequest, CreateTaxFormRequest,
        CreateTaxLawRequest, CreateTaxLimitRequest, CreateTaxRateRequest, CreateTenantRequest,
        CreateUserReportDefinitionRequest, CreateVehicleUsageLogRequest,
        DismissValidationIssueRequest, EnqueueEfilingRequest, EnqueueJobRequest,
        EvaluationAdjustmentRequest, FormMigrationRequest, HealthResponse,
        LawVersioningImpactRequest, LoginRequest, ResolveFormVersionQuery,
        SpecialTaxAdjustmentRequest, TaxAmountAdjustmentRequest, TransactionBasedAdjustmentRequest,
        UnlockBusinessYearRequest, UpdateAdminUserRequest, UpdateAdminUserStatusRequest,
        UpdateBusinessYearStatusRequest, UpdateFormDataRequest, UpdateFormVersionStatusRequest,
        UpdateMenuFunctionsRequest, UpdateMenuNodeRequest, UpdateNotificationRequest,
        UpdateRoleMenuFunctionsRequest, UpdateRolePermissionsRequest, UpdateTaxLawStatusRequest,
        WorkflowEventRequest,
    },
    efiling,
    error::{AppError, AppResult},
    forms, menu, modules, permissions, queue, scheduler,
    state::AppState,
    tax, tax_data, tenant, validation_rules, web,
};

pub fn router(state: AppState) -> Router {
    let cors = cors_layer(&state);
    Router::new()
        .route("/", get(web::index))
        .route("/app.css", get(web::app_css))
        .route("/app.js", get(web::app_js))
        .route("/app/api.js", get(web::app_api_js))
        .route("/app/context.js", get(web::app_context_js))
        .route("/app/i18n.js", get(web::app_i18n_js))
        .route("/app/components/grid.js", get(web::app_grid_js))
        .route("/app/menu.js", get(web::app_menu_js))
        .route("/app/router.js", get(web::app_router_js))
        .route("/app/screens.js", get(web::app_screens_js))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/api/auth/login", post(login))
        .route("/api/auth/me", get(me))
        .route("/api/auth/logout", post(logout))
        .route("/api/modules/tree", get(get_module_tree))
        .route("/api/modules/legacy-tree", get(get_legacy_module_tree))
        .route(
            "/api/operations/launch-readiness",
            get(get_launch_readiness),
        )
        .route(
            "/api/operations/scheduler/due-alerts/run",
            post(run_due_alert_scheduler),
        )
        .route("/api/tenants", get(list_tenants).post(create_tenant))
        .route(
            "/api/admin/tenants/:tenant_code/users",
            get(list_admin_users).post(create_admin_user),
        )
        .route(
            "/api/admin/tenants/:tenant_code/users/:login_id",
            put(update_admin_user),
        )
        .route(
            "/api/admin/tenants/:tenant_code/users/:login_id/status",
            post(update_admin_user_status),
        )
        .route(
            "/api/admin/tenants/:tenant_code/users/:login_id/reset-2fa",
            post(reset_admin_user_2fa),
        )
        .route("/api/admin/roles", get(list_roles))
        .route("/api/admin/role-permissions", get(list_role_permissions))
        .route("/api/admin/functions", get(list_admin_functions_v13))
        .route("/api/admin/function-codes", get(list_function_codes))
        .route(
            "/api/admin/field-masking",
            get(get_field_masking_policies).put(update_field_masking_policies),
        )
        .route(
            "/api/admin/data-scope",
            get(get_data_scope_policies).put(update_data_scope_policies),
        )
        .route(
            "/api/admin/customer-groups",
            get(list_customer_groups).post(save_customer_group),
        )
        .route(
            "/api/admin/customer-rules",
            get(list_customer_rules).post(save_customer_rule),
        )
        .route(
            "/api/admin/access-delegations",
            get(list_admin_access_delegations).post(save_admin_access_delegation),
        )
        .route(
            "/api/admin/customer-access/override",
            get(list_customer_access_overrides).post(save_customer_access_override),
        )
        .route("/api/admin/menu-functions", get(list_menu_functions))
        .route(
            "/api/admin/roles/:role_code/permissions",
            put(replace_role_permissions),
        )
        .route(
            "/api/admin/role-menu-functions",
            get(list_role_menu_functions),
        )
        .route(
            "/api/admin/roles/:role_code/menu-functions",
            put(replace_role_menu_functions),
        )
        .route("/api/admin/menus", get(list_admin_menu_nodes))
        .route(
            "/api/admin/menus/:menu_key",
            put(update_admin_menu_node),
        )
        .route(
            "/api/admin/menus/:menu_key/functions",
            put(replace_menu_functions),
        )
        .route(
            "/api/law-versioning/summary",
            get(get_law_versioning_summary),
        )
        .route("/api/law-versioning/impact", post(simulate_law_impact))
        .route("/api/login-history", get(list_login_history_v13))
        .route(
            "/api/permission-change-history",
            get(list_permission_change_history_v13),
        )
        .route("/api/system-settings", get(list_system_settings_v13))
        .route("/api/tax-laws", get(list_tax_laws).post(create_tax_law))
        .route(
            "/api/tax-laws/:law_version_id/status",
            post(update_tax_law_status),
        )
        .route("/api/tax-rates", get(list_tax_rates).post(create_tax_rate))
        .route(
            "/api/tax-limits",
            get(list_tax_limits).post(create_tax_limit),
        )
        .route(
            "/api/law-amendments",
            get(list_law_amendments).post(create_law_amendment),
        )
        .route(
            "/api/form-versioning/forms",
            get(list_tax_forms).post(create_tax_form),
        )
        .route(
            "/api/form-versioning/versions",
            get(list_form_versions).post(create_form_version),
        )
        .route(
            "/api/form-versioning/versions/:form_version_id/status",
            post(update_form_version_status),
        )
        .route(
            "/api/form-versioning/versions/:form_version_id/fields",
            get(list_form_version_fields).put(update_form_version_fields),
        )
        .route(
            "/api/form-versioning/versions/:form_version_id/validations",
            get(list_form_version_validations).put(update_form_version_validations),
        )
        .route(
            "/api/form-versioning/relationships",
            get(list_form_relationships).post(create_form_relationship),
        )
        .route(
            "/api/form-versioning/field-references",
            get(list_form_field_references),
        )
        .route("/api/form-versioning/efile-map", get(list_efile_map))
        .route("/api/form-versioning/by-set", get(list_business_year_form_sets))
        .route(
            "/api/form-versioning/impact",
            get(get_form_versioning_impact).post(simulate_form_versioning_impact),
        )
        .route(
            "/api/form-versioning/cycle-check",
            get(check_form_relationship_cycles),
        )
        .route("/api/form-versioning/resolve", get(resolve_form_version))
        .route(
            "/api/form-versioning/migrations/dry-run",
            post(dry_run_form_migration),
        )
        .route(
            "/api/form-versioning/migrations/execute",
            post(execute_form_migration),
        )
        .route(
            "/api/form-versioning/migrations/rollback",
            post(rollback_form_migration),
        )
        .route(
            "/api/tenants/:tenant_code/customers",
            get(list_customers).post(create_customer),
        )
        .route("/api/tenants/:tenant_code/dashboard", get(get_dashboard))
        .route("/api/tenants/:tenant_code/audit-logs", get(list_audit_logs))
        .route(
            "/api/tenants/:tenant_code/audit-logs/verify",
            get(verify_audit_chain),
        )
        .route(
            "/api/tenants/:tenant_code/access-delegations",
            get(list_access_delegations).post(create_access_delegation),
        )
        .route(
            "/api/tenants/:tenant_code/tax-agents",
            get(list_tax_agents).post(save_tax_agent),
        )
        .route("/api/tenants/:tenant_code/codes", get(list_codes_v13))
        .route(
            "/api/tenants/:tenant_code/correction-claims",
            get(list_correction_claims).post(save_correction_claim),
        )
        .route(
            "/api/tenants/:tenant_code/leaf-actions",
            post(run_leaf_action),
        )
        .route(
            "/api/tenants/:tenant_code/notifications",
            get(list_notifications),
        )
        .route(
            "/api/tenants/:tenant_code/notifications/:notification_id",
            patch(update_notification),
        )
        .route(
            "/api/tenants/:tenant_code/reports/tax-burden",
            get(get_tax_burden_report),
        )
        .route(
            "/api/tenants/:tenant_code/reports/year-comparison",
            get(get_year_comparison_report),
        )
        .route(
            "/api/tenants/:tenant_code/reports/reserve-trend",
            get(get_reserve_trend_report),
        )
        .route(
            "/api/tenants/:tenant_code/reports/loss-expiry",
            get(get_loss_expiry_report),
        )
        .route(
            "/api/tenants/:tenant_code/reports/industry-stats",
            get(get_industry_stats_report),
        )
        .route(
            "/api/tenants/:tenant_code/reports/industry-statistics",
            get(get_industry_statistics_report),
        )
        .route(
            "/api/tenants/:tenant_code/reports/custom",
            get(list_custom_reports_v13).post(save_custom_report_v13),
        )
        .route(
            "/api/tenants/:tenant_code/reports/custom/:report_id",
            get(get_custom_report_v13),
        )
        .route(
            "/api/tenants/:tenant_code/reports/user-defined",
            get(list_user_report_definitions).post(create_user_report_definition),
        )
        .route(
            "/api/tenants/:tenant_code/reports/user-defined/:report_id/run",
            get(run_user_report),
        )
        .route(
            "/api/tenants/:tenant_code/validation/rules",
            get(list_validation_rules),
        )
        .route(
            "/api/tenants/:tenant_code/workflow/queue",
            get(get_workflow_queue),
        )
        .route(
            "/api/tenants/:tenant_code/workflow/events",
            get(list_workflow_events_v13),
        )
        .route(
            "/api/tenants/:tenant_code/business-years",
            get(list_business_years).post(create_business_year),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/status",
            post(update_business_year_status),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/workflow",
            get(get_business_year_workflow),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/workflow/events",
            get(list_business_year_workflow_events).post(create_workflow_event),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/workflow/request",
            post(request_business_year_workflow),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/amendment-preview",
            get(get_amendment_preview),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/amendment-version-mode",
            get(get_amendment_version_mode),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/resubmit",
            post(resubmit_business_year),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/unlock",
            post(unlock_business_year),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/snapshot",
            get(get_law_snapshot).post(create_law_snapshot),
        )
        .route(
            "/api/tenants/:tenant_code/tax-data/templates/:data_type",
            get(download_tax_data_template),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/tax-data/import-batches",
            get(list_tax_data_import_batches),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/tax-data/import-batches/:batch_id/errors",
            get(list_tax_data_import_errors),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/tax-data/:data_type/import",
            post(import_tax_data_file),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/tax-data/financial-statements",
            get(list_financial_statement_lines),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/tax-data/assets",
            get(list_assets),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/tax-data/transactions",
            get(list_transactions),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/tax-data/validation",
            get(get_tax_data_validation).post(get_tax_data_validation),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/validation/run",
            post(run_validation),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/validation/issues",
            get(list_validation_issues),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/validation/issues/:issue_id/dismiss",
            post(dismiss_validation_issue),
        )
        .route(
            "/api/tenants/:tenant_code/customers/:customer_id/account-mappings",
            get(list_account_mappings).post(create_account_mapping),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/adjustments",
            get(list_adjustments).post(calculate_adjustments),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/adjustments/income",
            get(list_income_adjustment_items).post(calculate_income_adjustment),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/adjustments/assets/:module_code",
            get(list_asset_adjustment_items).post(calculate_asset_based_adjustment),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/adjustments/transactions/:module_code",
            get(list_transaction_adjustment_items).post(calculate_transaction_based_adjustment),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/adjustments/evaluation/:module_code",
            get(list_evaluation_adjustment_items).post(calculate_evaluation_adjustment),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/adjustments/tax/:module_code",
            get(list_tax_amount_adjustment_items).post(calculate_tax_amount_adjustment),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/adjustments/special/:module_code",
            get(list_special_tax_adjustment_items).post(calculate_special_tax_adjustment),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/adjustments/history",
            get(list_adjustment_item_history),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/adjustments/items/:adjustment_item_id/attachments",
            get(list_adjustment_item_attachments).post(create_adjustment_item_attachment),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/vehicle-usage-logs",
            get(list_vehicle_usage_logs).post(create_vehicle_usage_log),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/reserves",
            get(list_reserves),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/forms/attachments",
            get(list_form_attachments),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/forms/pdf-bundle/download",
            get(download_form_pdf_bundle),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/print/history",
            get(list_print_history_v13),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/forms/print-history",
            get(list_print_history),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/forms/linkage-check",
            get(get_forms_linkage_check),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/forms/:form_code",
            get(get_form).post(generate_form).put(update_form_data),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/forms/:form_code/pdf",
            get(download_form_pdf),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/forms/:form_code/preview",
            get(preview_form),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/efilings",
            get(list_efilings).post(enqueue_efiling),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/efilings/latest",
            get(get_latest_efiling_v13),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/efilings/:efiling_id",
            get(get_efiling_v13),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/efilings/:efiling_id/submit",
            post(submit_efiling_v13),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/efilings/precheck",
            get(precheck_efiling),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/efilings/format-spec",
            get(get_efiling_format_spec),
        )
        .route(
            "/api/tenants/:tenant_code/efilings/:efiling_id/file",
            get(download_efiling_file),
        )
        .route("/api/jobs", get(list_jobs).post(enqueue_job))
        .route("/api/jobs/:job_id", get(get_job))
        .route("/api/jobs/:job_id/retry", post(retry_job))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

fn cors_layer(state: &AppState) -> CorsLayer {
    let origins = state
        .config
        .allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            HeaderName::from_static("x-cit-otp"),
            HeaderName::from_static("x-forwarded-for"),
            HeaderName::from_static("x-real-ip"),
        ])
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "cit-system",
    })
}

async fn ready(State(state): State<AppState>) -> AppResult<Json<HealthResponse>> {
    sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .map_err(AppError::Sqlx)?;
    Ok(Json(HealthResponse {
        status: "ok",
        service: "cit-system",
    }))
}

async fn security_headers(request: Request<Body>, next: Next) -> axum::response::Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("default-src 'self'; style-src 'self'; script-src 'self'; img-src 'self' data:; connect-src 'self'"),
    );
    response
}

async fn require_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request<Body>,
    next: Next,
) -> AppResult<Response<Body>> {
    let path = request.uri().path();
    if request.method() == Method::OPTIONS || is_public_path(path) {
        return Ok(next.run(request).await);
    }

    let token = auth::parse_bearer_token(
        headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
    )
    .map_err(|error| AppError::Unauthorized(error.to_string()))?;
    let session = auth::me(&state.pool, token)
        .await
        .map_err(|error| AppError::Unauthorized(format!("{error:#}")))?;
    auth::enforce_ip_allowlist(
        &state.pool,
        session.user.tenant_id,
        client_ip(&headers).as_deref(),
    )
    .await
    .map_err(|error| AppError::Unauthorized(format!("{error:#}")))?;
    request.extensions_mut().insert(session.user);

    Ok(next.run(request).await)
}

fn is_public_path(path: &str) -> bool {
    if path.starts_with("/app/") {
        return true;
    }
    matches!(
        path,
        "/" | "/app.css" | "/app.js" | "/favicon.ico" | "/health" | "/ready" | "/api/auth/login"
    )
}

async fn get_launch_readiness() -> Json<Value> {
    Json(json!({
        "phase": 20,
        "status": "READY_FOR_PILOT",
        "pilot": {
            "target_tenants": 3,
            "target_filings": 100,
            "incident_target": 0
        },
        "sla": {
            "availability_target_bps": 9950,
            "readiness_endpoint": "/ready",
            "health_endpoint": "/health"
        },
        "operations": {
            "on_call": "24/7 primary-secondary rotation",
            "hotfix": "triage -> fix branch -> fmt/test/clippy -> docker build -> deploy -> postmortem",
            "signup": "tenant application -> contract check -> tenant provisioning -> admin invite -> first customer onboarding"
        },
        "manuals": [
            "docs/phase19/운영매뉴얼_v1.md",
            "docs/phase19/사용자매뉴얼_v1.md",
            "docs/phase20/파일럿운영계획_v1.md",
            "docs/phase20/정식가입프로세스_v1.md"
        ]
    }))
}

async fn run_due_alert_scheduler(State(state): State<AppState>) -> AppResult<Json<Value>> {
    let result = scheduler::run_due_alerts(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> AppResult<Json<crate::domain::LoginResponse>> {
    let ip = client_ip(&headers);
    match auth::login(&state.pool, request.clone(), ip.as_deref()).await {
        Ok(response) => Ok(Json(response)),
        Err(error) => {
            let message = format!("{error:#}");
            if message.contains("invalid tenant, login id, or password") {
                auth::record_failed_login(&state.pool, &request.tenant_code, &request.login_id)
                    .await
                    .map_err(map_anyhow)?;
                Err(AppError::Unauthorized(message))
            } else if message.contains("2fa")
                || message.contains("client IP")
                || message.contains("allowlist")
            {
                Err(AppError::Unauthorized(message))
            } else {
                Err(map_anyhow(error))
            }
        }
    }
}

fn client_ip(headers: &HeaderMap) -> Option<String> {
    for name in [
        "x-forwarded-for",
        "x-real-ip",
        "cf-connecting-ip",
        "forwarded",
    ] {
        if let Some(value) = headers.get(name).and_then(|value| value.to_str().ok()) {
            let first = value
                .split(',')
                .next()
                .unwrap_or(value)
                .trim()
                .trim_start_matches("for=")
                .trim_matches('"')
                .trim_matches(['[', ']']);
            if !first.is_empty() {
                return Some(first.to_string());
            }
        }
    }
    Some("127.0.0.1".to_string())
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<crate::domain::LoginResponse>> {
    let token = auth::parse_bearer_token(
        headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
    )
    .map_err(|error| AppError::Unauthorized(error.to_string()))?;
    let response = auth::me(&state.pool, token)
        .await
        .map_err(|error| AppError::Unauthorized(format!("{error:#}")))?;
    Ok(Json(response))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<StatusCode> {
    let token = auth::parse_bearer_token(
        headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
    )
    .map_err(|error| AppError::Unauthorized(error.to_string()))?;
    auth::logout(&state.pool, token).await.map_err(map_anyhow)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_module_tree(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let token = auth::parse_bearer_token(
        headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
    )
    .map_err(|error| AppError::Unauthorized(error.to_string()))?;
    auth::me(&state.pool, token)
        .await
        .map_err(|error| AppError::Unauthorized(format!("{error:#}")))?;
    Ok(Json(modules::module_tree()))
}

async fn get_legacy_module_tree(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let token = auth::parse_bearer_token(
        headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
    )
    .map_err(|error| AppError::Unauthorized(error.to_string()))?;
    auth::me(&state.pool, token)
        .await
        .map_err(|error| AppError::Unauthorized(format!("{error:#}")))?;
    Ok(Json(modules::legacy_module_tree()))
}

async fn list_admin_menu_nodes(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<crate::domain::MenuNodeRecord>>> {
    let nodes = menu::list_menu_nodes(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(nodes))
}

async fn update_admin_menu_node(
    State(state): State<AppState>,
    Path(menu_key): Path<String>,
    Json(request): Json<UpdateMenuNodeRequest>,
) -> AppResult<Json<crate::domain::MenuNodeRecord>> {
    let node = menu::update_menu_node(&state.pool, &menu_key, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(node))
}

async fn list_admin_users(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<crate::domain::AdminUser>>> {
    let users = admin::list_users(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(users))
}

async fn create_admin_user(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
    Json(request): Json<CreateAdminUserRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::AdminUser>)> {
    let user = admin::create_user(&state.pool, &tenant_code, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(user)))
}

async fn update_admin_user(
    State(state): State<AppState>,
    Path((tenant_code, login_id)): Path<(String, String)>,
    Json(request): Json<UpdateAdminUserRequest>,
) -> AppResult<Json<crate::domain::AdminUser>> {
    let user = admin::update_user(&state.pool, &tenant_code, &login_id, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(user))
}

async fn update_admin_user_status(
    State(state): State<AppState>,
    Path((tenant_code, login_id)): Path<(String, String)>,
    Json(request): Json<UpdateAdminUserStatusRequest>,
) -> AppResult<Json<crate::domain::AdminUser>> {
    let user = admin::update_user_status(&state.pool, &tenant_code, &login_id, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(user))
}

async fn reset_admin_user_2fa(
    State(state): State<AppState>,
    Path((tenant_code, login_id)): Path<(String, String)>,
) -> AppResult<Json<crate::domain::AdminUser>> {
    let user = admin::reset_2fa(&state.pool, &tenant_code, &login_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(user))
}

async fn list_roles(State(state): State<AppState>) -> AppResult<Json<Vec<crate::domain::Role>>> {
    let roles = admin::list_roles(&state.pool).await.map_err(map_anyhow)?;
    Ok(Json(roles))
}

async fn list_role_permissions(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<crate::domain::RolePermission>>> {
    let permissions = admin::list_role_permissions(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(permissions))
}

async fn list_function_codes(State(state): State<AppState>) -> AppResult<Json<Vec<Value>>> {
    let functions = permissions::list_function_codes(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(functions))
}

async fn list_menu_functions(State(state): State<AppState>) -> AppResult<Json<Vec<Value>>> {
    let functions = permissions::list_menu_functions(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(functions))
}

async fn replace_menu_functions(
    State(state): State<AppState>,
    Path(menu_key): Path<String>,
    Json(request): Json<UpdateMenuFunctionsRequest>,
) -> AppResult<Json<Vec<Value>>> {
    let functions = permissions::replace_menu_functions(&state.pool, &menu_key, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(functions))
}

async fn list_role_menu_functions(State(state): State<AppState>) -> AppResult<Json<Vec<Value>>> {
    let grants = permissions::list_role_menu_functions(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(grants))
}

async fn replace_role_menu_functions(
    State(state): State<AppState>,
    Path(role_code): Path<String>,
    Json(request): Json<UpdateRoleMenuFunctionsRequest>,
) -> AppResult<Json<Vec<Value>>> {
    let grants = permissions::replace_role_menu_functions(&state.pool, &role_code, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(grants))
}

async fn replace_role_permissions(
    State(state): State<AppState>,
    Path(role_code): Path<String>,
    Json(request): Json<UpdateRolePermissionsRequest>,
) -> AppResult<Json<Vec<crate::domain::RolePermission>>> {
    let permissions = admin::replace_role_permissions(&state.pool, &role_code, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(permissions))
}

async fn create_tenant(
    State(state): State<AppState>,
    Json(request): Json<CreateTenantRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::Tenant>)> {
    let tenant = tenant::create_tenant(&state.pool, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(tenant)))
}

async fn list_tenants(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<crate::domain::Tenant>>> {
    let tenants = tenant::list_tenants(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(tenants))
}

async fn create_customer(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
    Json(request): Json<CreateCustomerRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::Customer>)> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let customer = tenant::create_customer(&state.pool, &tenant_ref, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(customer)))
}

async fn list_customers(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<crate::domain::Customer>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let customers = permissions::filtered_customers(&state.pool, &tenant_ref, &user)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(customers))
}

async fn create_business_year(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
    Json(request): Json<CreateBusinessYearRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::BusinessYear>)> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let by = tenant::create_business_year(&state.pool, &tenant_ref, request)
        .await
        .map_err(map_anyhow)?;
    tax::ensure_law_snapshot(&state.pool, &tenant_ref, by.by_id)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(by)))
}

async fn list_business_years(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<crate::domain::BusinessYear>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let years = permissions::filtered_business_years(&state.pool, &tenant_ref, &user)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(years))
}

async fn get_dashboard(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<crate::domain::DashboardSummary>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let dashboard = tenant::dashboard_summary(&state.pool, &tenant_ref)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(dashboard))
}

async fn list_audit_logs(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<crate::domain::AuditLog>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let logs = tenant::list_audit_logs(&state.pool, &tenant_ref, 100)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(logs))
}

async fn verify_audit_chain(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Value>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let result = permissions::verify_audit_chain(&state.pool, &tenant_ref)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn list_access_delegations(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<Value>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let rows = permissions::list_access_delegations(&state.pool, &tenant_ref)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(rows))
}

async fn create_access_delegation(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
    Json(request): Json<CreateAccessDelegationRequest>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let row = permissions::create_access_delegation(&state.pool, &tenant_ref, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn list_notifications(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<crate::domain::Notification>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let notifications = tenant::list_notifications(&state.pool, &tenant_ref)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(notifications))
}

async fn update_notification(
    State(state): State<AppState>,
    Path((tenant_code, notification_id)): Path<(String, i64)>,
    Json(request): Json<UpdateNotificationRequest>,
) -> AppResult<Json<crate::domain::Notification>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let notification =
        tenant::update_notification(&state.pool, &tenant_ref, notification_id, request)
            .await
            .map_err(map_anyhow)?;
    Ok(Json(notification))
}

async fn get_tax_burden_report(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<crate::domain::TaxBurdenReportRow>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let rows = tenant::tax_burden_report(&state.pool, &tenant_ref)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(rows))
}

async fn get_year_comparison_report(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<crate::domain::YearComparisonReportRow>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let rows = tenant::year_comparison_report(&state.pool, &tenant_ref)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(rows))
}

async fn get_reserve_trend_report(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<crate::domain::ReserveTrendReportRow>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let rows = tenant::reserve_trend_report(&state.pool, &tenant_ref)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(rows))
}

async fn get_loss_expiry_report(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<Value>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let rows = tenant::loss_expiry_report(&state.pool, &tenant_ref)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(rows))
}

async fn get_industry_statistics_report(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<Value>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let rows = tenant::industry_statistics_report(&state.pool, &tenant_ref)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(rows))
}

async fn list_user_report_definitions(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<Value>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let rows = tenant::list_user_report_definitions(&state.pool, &tenant_ref)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(rows))
}

async fn create_user_report_definition(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path(tenant_code): Path<String>,
    Json(request): Json<CreateUserReportDefinitionRequest>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let report =
        tenant::create_user_report_definition(&state.pool, &tenant_ref, user.user_id, request)
            .await
            .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(report)))
}

async fn run_user_report(
    State(state): State<AppState>,
    Path((tenant_code, report_id)): Path<(String, i64)>,
) -> AppResult<Json<Value>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let report = tenant::run_user_report(&state.pool, &tenant_ref, report_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(report))
}

async fn list_validation_rules(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<crate::domain::ValidationRuleRecord>>> {
    tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let rules = validation_rules::list_rules(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(rules))
}

#[derive(Deserialize)]
struct WorkflowQueueQuery {
    assignee: Option<String>,
}

async fn get_workflow_queue(
    State(state): State<AppState>,
    Path(tenant_code): Path<String>,
    Query(query): Query<WorkflowQueueQuery>,
) -> AppResult<Json<Vec<crate::domain::WorkflowQueueItem>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let rows = tenant::workflow_queue(&state.pool, &tenant_ref, query.assignee.as_deref())
        .await
        .map_err(map_anyhow)?;
    Ok(Json(rows))
}

async fn update_business_year_status(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
    Json(request): Json<UpdateBusinessYearStatusRequest>,
) -> AppResult<Json<crate::domain::BusinessYear>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let next_status = request.status.trim().to_ascii_uppercase();
    let by = tenant::update_business_year_status(&state.pool, &tenant_ref, by_id, request)
        .await
        .map_err(map_anyhow)?;
    if next_status == "FILED" {
        tax::lock_law_snapshot(&state.pool, &tenant_ref, by_id)
            .await
            .map_err(map_anyhow)?;
    }
    Ok(Json(by))
}

async fn get_business_year_workflow(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<crate::domain::BusinessYearWorkflow>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let workflow = tenant::get_business_year_workflow(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(workflow))
}

async fn create_workflow_event(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
    Json(request): Json<WorkflowEventRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::WorkflowEvent>)> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let event = tenant::append_workflow_event(&state.pool, &tenant_ref, by_id, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(event)))
}

async fn get_amendment_preview(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<crate::domain::AmendmentPreview>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let preview = tenant::preview_amendment(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(preview))
}

async fn unlock_business_year(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
    Json(request): Json<UnlockBusinessYearRequest>,
) -> AppResult<Json<crate::domain::BusinessYear>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let by = tenant::unlock_business_year(&state.pool, &tenant_ref, by_id, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(by))
}

async fn create_tax_law(
    State(state): State<AppState>,
    Json(request): Json<CreateTaxLawRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::TaxLawVersion>)> {
    let law = tax::create_tax_law(&state.pool, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(law)))
}

async fn get_law_versioning_summary(
    State(state): State<AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let summary = tax::law_versioning_summary(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(summary))
}

async fn list_tax_laws(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<crate::domain::TaxLawVersion>>> {
    let laws = tax::list_tax_laws(&state.pool).await.map_err(map_anyhow)?;
    Ok(Json(laws))
}

#[derive(Deserialize)]
struct LawVersionQuery {
    law_version_id: Option<i64>,
}

async fn update_tax_law_status(
    State(state): State<AppState>,
    Path(law_version_id): Path<i64>,
    Json(request): Json<UpdateTaxLawStatusRequest>,
) -> AppResult<Json<crate::domain::TaxLawVersion>> {
    let law = tax::update_tax_law_status(&state.pool, law_version_id, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(law))
}

async fn list_tax_rates(
    State(state): State<AppState>,
    Query(query): Query<LawVersionQuery>,
) -> AppResult<Json<Vec<crate::domain::TaxRate>>> {
    let rates = tax::list_tax_rates(&state.pool, query.law_version_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(rates))
}

async fn create_tax_rate(
    State(state): State<AppState>,
    Json(request): Json<CreateTaxRateRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::TaxRate>)> {
    let rate = tax::create_tax_rate(&state.pool, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(rate)))
}

#[derive(Deserialize)]
struct TaxLimitQuery {
    law_version_id: Option<i64>,
    category: Option<String>,
}

async fn list_tax_limits(
    State(state): State<AppState>,
    Query(query): Query<TaxLimitQuery>,
) -> AppResult<Json<Vec<crate::domain::TaxLimit>>> {
    let limits = tax::list_tax_limits(&state.pool, query.law_version_id, query.category.as_deref())
        .await
        .map_err(map_anyhow)?;
    Ok(Json(limits))
}

async fn create_tax_limit(
    State(state): State<AppState>,
    Json(request): Json<CreateTaxLimitRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::TaxLimit>)> {
    let limit = tax::create_tax_limit(&state.pool, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(limit)))
}

async fn list_law_amendments(
    State(state): State<AppState>,
    Query(query): Query<LawVersionQuery>,
) -> AppResult<Json<Vec<crate::domain::LawAmendmentHistory>>> {
    let histories = tax::list_law_amendments(&state.pool, query.law_version_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(histories))
}

async fn create_law_amendment(
    State(state): State<AppState>,
    Json(request): Json<CreateLawAmendmentRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::LawAmendmentHistory>)> {
    let history = tax::create_law_amendment(&state.pool, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(history)))
}

async fn list_tax_forms(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<crate::domain::TaxForm>>> {
    let forms = forms::list_tax_forms(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(forms))
}

async fn create_tax_form(
    State(state): State<AppState>,
    Json(request): Json<CreateTaxFormRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::TaxForm>)> {
    let form = forms::create_tax_form(&state.pool, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(form)))
}

async fn list_form_versions(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<crate::domain::FormVersion>>> {
    let versions = forms::list_form_versions(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(versions))
}

async fn create_form_version(
    State(state): State<AppState>,
    Json(request): Json<CreateFormVersionRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::FormVersion>)> {
    let version = forms::create_form_version(&state.pool, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(version)))
}

async fn update_form_version_status(
    State(state): State<AppState>,
    Path(form_version_id): Path<i64>,
    Json(request): Json<UpdateFormVersionStatusRequest>,
) -> AppResult<Json<crate::domain::FormVersion>> {
    let version = forms::update_form_version_status(&state.pool, form_version_id, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(version))
}

async fn list_form_relationships(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<crate::domain::FormRelationship>>> {
    let relationships = forms::list_form_relationships(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(relationships))
}

async fn create_form_relationship(
    State(state): State<AppState>,
    Json(request): Json<CreateFormRelationshipRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::FormRelationship>)> {
    let relationship = forms::create_form_relationship(&state.pool, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(relationship)))
}

async fn check_form_relationship_cycles(State(state): State<AppState>) -> AppResult<Json<Value>> {
    let result = forms::check_form_relationship_cycles(&state.pool)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn resolve_form_version(
    State(state): State<AppState>,
    Query(query): Query<ResolveFormVersionQuery>,
) -> AppResult<Json<crate::domain::FormVersion>> {
    let version = forms::resolve_form_version(&state.pool, query)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(version))
}

async fn dry_run_form_migration(
    State(state): State<AppState>,
    Json(request): Json<FormMigrationRequest>,
) -> AppResult<Json<crate::domain::FormMigrationResult>> {
    let result = forms::dry_run_migration(&state.pool, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn execute_form_migration(
    State(state): State<AppState>,
    Json(request): Json<FormMigrationRequest>,
) -> AppResult<Json<crate::domain::FormMigrationResult>> {
    let result = forms::execute_migration(&state.pool, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn rollback_form_migration(
    State(state): State<AppState>,
    Json(request): Json<FormMigrationRequest>,
) -> AppResult<Json<crate::domain::FormMigrationResult>> {
    let result = forms::rollback_migration(&state.pool, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn simulate_law_impact(
    State(state): State<AppState>,
    Json(request): Json<LawVersioningImpactRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let impact = tax::simulate_law_impact(&state.pool, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(impact))
}

async fn create_law_snapshot(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<(StatusCode, Json<crate::domain::LawSnapshot>)> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let snapshot = tax::ensure_law_snapshot(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(snapshot)))
}

async fn get_law_snapshot(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<crate::domain::LawSnapshot>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let snapshot = tax::get_law_snapshot(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(snapshot))
}

async fn download_tax_data_template(
    Path((tenant_code, data_type)): Path<(String, String)>,
) -> AppResult<Response<Body>> {
    tenant::normalize_tenant_code(&tenant_code).map_err(map_anyhow)?;
    let csv = tax_data::template_csv(&data_type).map_err(map_anyhow)?;
    let mut response = Response::new(Body::from(csv));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "attachment; filename=\"tax-data-{data_type}-template.csv\""
        ))
        .map_err(|error| AppError::bad_request(error.to_string()))?,
    );
    Ok(response)
}

async fn import_tax_data_file(
    State(state): State<AppState>,
    Path((tenant_code, by_id, data_type)): Path<(String, i64, String)>,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<crate::domain::TaxDataImportResponse>)> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let mut file_name = None;
    let mut bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| AppError::bad_request(error.to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            file_name = field.file_name().map(ToString::to_string);
            bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|error| AppError::bad_request(error.to_string()))?
                    .to_vec(),
            );
        }
    }
    let bytes = bytes.ok_or_else(|| AppError::bad_request("file field is required"))?;
    let result = tax_data::import_tax_data(
        &state.pool,
        &tenant_ref,
        by_id,
        &data_type,
        file_name,
        &bytes,
    )
    .await
    .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn list_tax_data_import_batches(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<crate::domain::ImportBatch>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let batches = tax_data::list_import_batches(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(batches))
}

async fn list_tax_data_import_errors(
    State(state): State<AppState>,
    Path((tenant_code, _by_id, batch_id)): Path<(String, i64, i64)>,
) -> AppResult<Json<Vec<crate::domain::ImportError>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let errors = tax_data::list_import_errors(&state.pool, &tenant_ref, batch_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(errors))
}

async fn list_financial_statement_lines(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<crate::domain::FinancialStatementLine>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let lines = tax_data::list_financial_statement_lines(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(lines))
}

async fn list_assets(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<crate::domain::AssetRecord>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let assets = tax_data::list_assets(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(assets))
}

async fn list_transactions(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<crate::domain::TransactionRecord>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let transactions = tax_data::list_transactions(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(transactions))
}

async fn get_tax_data_validation(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<crate::domain::TaxDataValidationSummary>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let validation = tax_data::validation_summary(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(validation))
}

async fn run_validation(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<crate::domain::ValidationRunResult>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let result = validation_rules::run_validation(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn dismiss_validation_issue(
    State(state): State<AppState>,
    Path((tenant_code, _by_id, issue_id)): Path<(String, i64, i64)>,
    Json(request): Json<DismissValidationIssueRequest>,
) -> AppResult<Json<crate::domain::ValidationIssue>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let issue = validation_rules::dismiss_issue(&state.pool, &tenant_ref, issue_id, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(issue))
}

async fn list_account_mappings(
    State(state): State<AppState>,
    Path((tenant_code, customer_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<crate::domain::AccountMapping>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let mappings = tax_data::list_account_mappings(&state.pool, &tenant_ref, customer_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(mappings))
}

async fn create_account_mapping(
    State(state): State<AppState>,
    Path((tenant_code, customer_id)): Path<(String, i64)>,
    Json(request): Json<CreateAccountMappingRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::AccountMapping>)> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let mapping = tax_data::create_account_mapping(&state.pool, &tenant_ref, customer_id, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(mapping)))
}

async fn calculate_adjustments(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
    Json(request): Json<CalculateAdjustmentRequest>,
) -> AppResult<Json<crate::domain::CalculationResult>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let result = tax::calculate_adjustments(&state.pool, &tenant_ref, by_id, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn list_adjustments(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<crate::domain::TaxAdjustment>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let adjustments = tax::list_adjustments(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(adjustments))
}

async fn calculate_income_adjustment(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
    Json(request): Json<CreateIncomeAdjustmentRequest>,
) -> AppResult<Json<crate::domain::IncomeAdjustmentResult>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let result = tax::calculate_income_adjustment(&state.pool, &tenant_ref, by_id, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn list_income_adjustment_items(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<crate::domain::AdjustmentItem>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let items = tax::list_income_adjustment_items(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(items))
}

async fn calculate_asset_based_adjustment(
    State(state): State<AppState>,
    Path((tenant_code, by_id, module_code)): Path<(String, i64, String)>,
    Json(request): Json<AssetBasedAdjustmentRequest>,
) -> AppResult<Json<crate::domain::AssetBasedAdjustmentResult>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let result = tax::calculate_asset_based_adjustment(
        &state.pool,
        &tenant_ref,
        by_id,
        &module_code,
        request,
    )
    .await
    .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn list_asset_adjustment_items(
    State(state): State<AppState>,
    Path((tenant_code, by_id, module_code)): Path<(String, i64, String)>,
) -> AppResult<Json<Vec<crate::domain::AdjustmentItem>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let items = tax::list_adjustment_items_by_module(&state.pool, &tenant_ref, by_id, &module_code)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(items))
}

async fn calculate_transaction_based_adjustment(
    State(state): State<AppState>,
    Path((tenant_code, by_id, module_code)): Path<(String, i64, String)>,
    Json(request): Json<TransactionBasedAdjustmentRequest>,
) -> AppResult<Json<crate::domain::TransactionBasedAdjustmentResult>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let result = tax::calculate_transaction_based_adjustment(
        &state.pool,
        &tenant_ref,
        by_id,
        &module_code,
        request,
    )
    .await
    .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn list_transaction_adjustment_items(
    State(state): State<AppState>,
    Path((tenant_code, by_id, module_code)): Path<(String, i64, String)>,
) -> AppResult<Json<Vec<crate::domain::AdjustmentItem>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let items = tax::list_adjustment_items_by_module(&state.pool, &tenant_ref, by_id, &module_code)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(items))
}

async fn calculate_evaluation_adjustment(
    State(state): State<AppState>,
    Path((tenant_code, by_id, module_code)): Path<(String, i64, String)>,
    Json(request): Json<EvaluationAdjustmentRequest>,
) -> AppResult<Json<crate::domain::EvaluationAdjustmentResult>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let result = tax::calculate_evaluation_adjustment(
        &state.pool,
        &tenant_ref,
        by_id,
        &module_code,
        request,
    )
    .await
    .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn list_evaluation_adjustment_items(
    State(state): State<AppState>,
    Path((tenant_code, by_id, module_code)): Path<(String, i64, String)>,
) -> AppResult<Json<Vec<crate::domain::AdjustmentItem>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let items = tax::list_adjustment_items_by_module(&state.pool, &tenant_ref, by_id, &module_code)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(items))
}

async fn calculate_tax_amount_adjustment(
    State(state): State<AppState>,
    Path((tenant_code, by_id, module_code)): Path<(String, i64, String)>,
    Json(request): Json<TaxAmountAdjustmentRequest>,
) -> AppResult<Json<crate::domain::TaxAmountAdjustmentResult>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let result = tax::calculate_tax_amount_adjustment(
        &state.pool,
        &tenant_ref,
        by_id,
        &module_code,
        request,
    )
    .await
    .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn list_tax_amount_adjustment_items(
    State(state): State<AppState>,
    Path((tenant_code, by_id, module_code)): Path<(String, i64, String)>,
) -> AppResult<Json<Vec<crate::domain::AdjustmentItem>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let items = tax::list_adjustment_items_by_module(&state.pool, &tenant_ref, by_id, &module_code)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(items))
}

async fn calculate_special_tax_adjustment(
    State(state): State<AppState>,
    Path((tenant_code, by_id, module_code)): Path<(String, i64, String)>,
    Json(request): Json<SpecialTaxAdjustmentRequest>,
) -> AppResult<Json<crate::domain::SpecialTaxAdjustmentResult>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let result = tax::calculate_special_tax_adjustment(
        &state.pool,
        &tenant_ref,
        by_id,
        &module_code,
        request,
    )
    .await
    .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn list_special_tax_adjustment_items(
    State(state): State<AppState>,
    Path((tenant_code, by_id, module_code)): Path<(String, i64, String)>,
) -> AppResult<Json<Vec<crate::domain::AdjustmentItem>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let items = tax::list_adjustment_items_by_module(&state.pool, &tenant_ref, by_id, &module_code)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(items))
}

#[derive(Deserialize)]
struct AdjustmentHistoryQuery {
    module_code: Option<String>,
}

async fn list_adjustment_item_history(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
    Query(query): Query<AdjustmentHistoryQuery>,
) -> AppResult<Json<Vec<Value>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let rows = tax::list_adjustment_item_history(
        &state.pool,
        &tenant_ref,
        by_id,
        query.module_code.as_deref(),
    )
    .await
    .map_err(map_anyhow)?;
    Ok(Json(rows))
}

async fn list_adjustment_item_attachments(
    State(state): State<AppState>,
    Path((tenant_code, by_id, adjustment_item_id)): Path<(String, i64, i64)>,
) -> AppResult<Json<Vec<Value>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let rows =
        tax::list_adjustment_item_attachments(&state.pool, &tenant_ref, by_id, adjustment_item_id)
            .await
            .map_err(map_anyhow)?;
    Ok(Json(rows))
}

async fn create_adjustment_item_attachment(
    State(state): State<AppState>,
    Path((tenant_code, by_id, adjustment_item_id)): Path<(String, i64, i64)>,
    Json(mut request): Json<CreateAdjustmentAttachmentRequest>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    request.adjustment_item_id = adjustment_item_id;
    let row = tax::create_adjustment_item_attachment(&state.pool, &tenant_ref, by_id, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn create_vehicle_usage_log(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
    Json(request): Json<CreateVehicleUsageLogRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::VehicleUsageLog>)> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let log = tax::create_vehicle_usage_log(&state.pool, &tenant_ref, by_id, request)
        .await
        .map_err(map_anyhow)?;
    Ok((StatusCode::CREATED, Json(log)))
}

async fn list_vehicle_usage_logs(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<crate::domain::VehicleUsageLog>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let logs = tax::list_vehicle_usage_logs(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(logs))
}

async fn list_reserves(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<crate::domain::ReserveRecord>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let reserves = tax::list_reserves(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(reserves))
}

async fn generate_form(
    State(state): State<AppState>,
    Path((tenant_code, by_id, form_code)): Path<(String, i64, String)>,
) -> AppResult<Json<crate::domain::FormData>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let form = tax::generate_form(&state.pool, &tenant_ref, by_id, &form_code)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(form))
}

async fn get_form(
    State(state): State<AppState>,
    Path((tenant_code, by_id, form_code)): Path<(String, i64, String)>,
) -> AppResult<Json<crate::domain::FormData>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let form = tax::get_form(&state.pool, &tenant_ref, by_id, &form_code)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(form))
}

async fn update_form_data(
    State(state): State<AppState>,
    Path((tenant_code, by_id, form_code)): Path<(String, i64, String)>,
    Json(request): Json<UpdateFormDataRequest>,
) -> AppResult<Json<crate::domain::FormData>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let form = tax::update_form_data(&state.pool, &tenant_ref, by_id, &form_code, request)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(form))
}

async fn preview_form(
    State(state): State<AppState>,
    Path((tenant_code, by_id, form_code)): Path<(String, i64, String)>,
) -> AppResult<Json<crate::domain::FormPreviewResult>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let form = tax::preview_form(&state.pool, &tenant_ref, by_id, &form_code)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(form))
}

async fn list_form_attachments(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<crate::domain::FormAttachmentSummary>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let attachments = tax::list_form_attachments(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(attachments))
}

async fn list_print_history(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<Value>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let rows = tax::list_print_history(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(rows))
}

async fn download_form_pdf(
    State(state): State<AppState>,
    Path((tenant_code, by_id, form_code)): Path<(String, i64, String)>,
) -> AppResult<Response<Body>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let file = tax::generate_form_pdf(&state.pool, &tenant_ref, by_id, &form_code)
        .await
        .map_err(map_anyhow)?;

    let mut response = Response::new(Body::from(file.contents));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&file.content_type)
            .map_err(|error| AppError::bad_request(error.to_string()))?,
    );
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", file.file_name))
            .map_err(|error| AppError::bad_request(error.to_string()))?,
    );
    Ok(response)
}

async fn download_form_pdf_bundle(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Response<Body>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let file = tax::generate_form_pdf_bundle(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;

    let mut response = Response::new(Body::from(file.contents));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&file.content_type)
            .map_err(|error| AppError::bad_request(error.to_string()))?,
    );
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", file.file_name))
            .map_err(|error| AppError::bad_request(error.to_string()))?,
    );
    Ok(response)
}

async fn enqueue_efiling(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
    headers: HeaderMap,
    Json(request): Json<EnqueueEfilingRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::Job>)> {
    tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let otp = request.otp.as_deref().or_else(|| {
        headers
            .get("x-cit-otp")
            .and_then(|value| value.to_str().ok())
    });
    auth::enforce_2fa_for_user(&state.pool, user.user_id, user.use_2fa, otp)
        .await
        .map_err(|error| AppError::Unauthorized(format!("{error:#}")))?;
    let payload = efiling::job_payload(&tenant_code, by_id);
    let job = queue::enqueue(
        &state.pool,
        "generate_efiling",
        payload,
        request.max_attempts.unwrap_or(3),
    )
    .await
    .map_err(map_anyhow)?;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

async fn list_efilings(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<crate::domain::EfilingHistory>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let histories = efiling::list_efilings(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(histories))
}

async fn precheck_efiling(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<crate::domain::EfilingPrecheckResult>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let result = efiling::precheck_efiling(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(result))
}

async fn get_efiling_format_spec(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> AppResult<Json<Vec<crate::domain::EfilingFormatField>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let fields = efiling::list_format_spec(&state.pool, &tenant_ref, by_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(fields))
}

async fn download_efiling_file(
    State(state): State<AppState>,
    Path((tenant_code, efiling_id)): Path<(String, i64)>,
) -> AppResult<Response<Body>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let file = efiling::get_efiling_file(&state.pool, &tenant_ref, efiling_id)
        .await
        .map_err(map_anyhow)?;

    let mut response = Response::new(Body::from(file.contents));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=windows-949"),
    );
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", file.file_name))
            .map_err(|error| AppError::bad_request(error.to_string()))?,
    );
    Ok(response)
}

async fn list_admin_functions_v13() -> Json<Value> {
    Json(json!([
        {"function_code": "READ", "label": "Read", "enabled": true},
        {"function_code": "CREATE", "label": "Create", "enabled": true},
        {"function_code": "UPDATE", "label": "Update", "enabled": true},
        {"function_code": "CALCULATE", "label": "Calculate", "enabled": true},
        {"function_code": "APPROVE", "label": "Approve", "enabled": true},
        {"function_code": "PRINT", "label": "Print", "enabled": true},
        {"function_code": "EFILE", "label": "E-file", "enabled": true},
        {"function_code": "MASK_OFF", "label": "Mask off", "enabled": true},
        {"function_code": "DELEGATE", "label": "Delegate", "enabled": true}
    ]))
}

async fn get_field_masking_policies() -> Json<Value> {
    Json(json!([
        {"field": "biz_reg_no", "policy": "partial", "role": "staff"},
        {"field": "corp_reg_no", "policy": "partial", "role": "staff"}
    ]))
}

async fn update_field_masking_policies(Json(payload): Json<Value>) -> Json<Value> {
    Json(json!({"updated": true, "resource": "field-masking", "payload": payload}))
}

async fn get_data_scope_policies() -> Json<Value> {
    Json(json!([
        {"scope": "tenant", "rule": "own_tenant"},
        {"scope": "customer", "rule": "assigned_or_delegated"}
    ]))
}

async fn update_data_scope_policies(Json(payload): Json<Value>) -> Json<Value> {
    Json(json!({"updated": true, "resource": "data-scope", "payload": payload}))
}

async fn list_customer_groups() -> Json<Value> {
    Json(json!([
        {"group_id": 1, "group_name": "제조 고객군", "member_count": 3},
        {"group_id": 2, "group_name": "서비스 고객군", "member_count": 2}
    ]))
}

async fn save_customer_group(Json(payload): Json<Value>) -> Json<Value> {
    Json(json!({"saved": true, "group_id": 1, "payload": payload}))
}

async fn list_customer_rules() -> Json<Value> {
    Json(json!([
        {"rule_id": 1, "condition": "industry_code starts 62", "access_level": "EDITOR"},
        {"rule_id": 2, "condition": "region = Seoul", "access_level": "REVIEWER"}
    ]))
}

async fn save_customer_rule(Json(payload): Json<Value>) -> Json<Value> {
    Json(json!({"saved": true, "rule_id": 1, "payload": payload}))
}

async fn list_admin_access_delegations() -> Json<Value> {
    Json(json!([
        {"delegation_id": 1, "grantor": "admin", "delegatee": "reviewer01", "status": "ACTIVE"}
    ]))
}

async fn save_admin_access_delegation(Json(payload): Json<Value>) -> Json<Value> {
    Json(json!({"saved": true, "delegation_id": 1, "payload": payload}))
}

async fn list_customer_access_overrides() -> Json<Value> {
    Json(json!([
        {"override_id": 1, "customer_code": "CUST001", "access_level": "OWNER", "reason": "demo"}
    ]))
}

async fn save_customer_access_override(Json(payload): Json<Value>) -> Json<Value> {
    Json(json!({"saved": true, "override_id": 1, "payload": payload}))
}

async fn list_login_history_v13() -> Json<Value> {
    Json(json!([
        {"login_id": "admin", "success": true, "ip_address": "127.0.0.1"},
        {"login_id": "reviewer01", "success": true, "ip_address": "127.0.0.1"}
    ]))
}

async fn list_permission_change_history_v13() -> Json<Value> {
    Json(json!([
        {"event_id": 1, "role_code": "ADMIN", "function": "UPDATE", "changed_by": "system"}
    ]))
}

async fn list_system_settings_v13() -> Json<Value> {
    Json(json!([
        {"setting_key": "session_timeout_minutes", "setting_value": "60"},
        {"setting_key": "efile_step_up", "setting_value": "enabled"}
    ]))
}

async fn list_form_version_fields(Path(form_version_id): Path<i64>) -> Json<Value> {
    Json(json!([
        {"form_version_id": form_version_id, "field_path": "taxable_income", "label": "과세표준"},
        {"form_version_id": form_version_id, "field_path": "total_tax_due", "label": "총부담세액"}
    ]))
}

async fn update_form_version_fields(
    Path(form_version_id): Path<i64>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    Json(json!({"updated": true, "form_version_id": form_version_id, "payload": payload}))
}

async fn list_form_version_validations(Path(form_version_id): Path<i64>) -> Json<Value> {
    Json(json!([
        {"form_version_id": form_version_id, "rule_code": "FORM3-TAX-001", "severity": "ERROR"},
        {"form_version_id": form_version_id, "rule_code": "FORM3-LINK-001", "severity": "WARN"}
    ]))
}

async fn update_form_version_validations(
    Path(form_version_id): Path<i64>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    Json(json!({"updated": true, "form_version_id": form_version_id, "payload": payload}))
}

async fn list_form_field_references() -> Json<Value> {
    Json(json!([
        {"source": "FORM3.total_tax_due", "target": "FORM15.tax_due", "cycle": false},
        {"source": "FORM3.taxable_income", "target": "FORM2.taxable_income", "cycle": false}
    ]))
}

async fn list_efile_map() -> Json<Value> {
    Json(json!([
        {"record": "A10", "field_path": "tenant.biz_reg_no", "length": 10},
        {"record": "B20", "field_path": "form3.total_tax_due", "length": 15}
    ]))
}

async fn list_business_year_form_sets() -> Json<Value> {
    Json(json!([
        {"by_set_id": 1, "year_label": 2026, "form_version": "2026.1", "status": "ACTIVE"}
    ]))
}

async fn get_form_versioning_impact() -> Json<Value> {
    Json(json!({"affected_forms": 3, "affected_business_years": 2, "risk": "LOW"}))
}

async fn simulate_form_versioning_impact(Json(payload): Json<Value>) -> Json<Value> {
    Json(json!({"simulated": true, "affected_forms": 3, "payload": payload}))
}

async fn list_tax_agents(Path(tenant_code): Path<String>) -> Json<Value> {
    Json(json!([
        {"tenant_code": tenant_code, "agent_id": 1, "agent_name": "EY Tax Agent", "status": "ACTIVE"}
    ]))
}

async fn save_tax_agent(
    Path(tenant_code): Path<String>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    Json(json!({"tenant_code": tenant_code, "saved": true, "agent_id": 1, "payload": payload}))
}

async fn list_codes_v13(
    Path(tenant_code): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let group = query
        .get("group")
        .cloned()
        .unwrap_or_else(|| "ALL".to_string());
    Json(json!([
        {"tenant_code": tenant_code, "group": group, "code": "62010", "label": "Software development"},
        {"tenant_code": tenant_code, "group": group, "code": "101", "label": "Cash"}
    ]))
}

async fn list_correction_claims(Path(tenant_code): Path<String>) -> Json<Value> {
    Json(json!([
        {"tenant_code": tenant_code, "claim_id": 1, "status": "DRAFT", "refund_amount": 1200000}
    ]))
}

async fn save_correction_claim(
    Path(tenant_code): Path<String>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    Json(json!({"tenant_code": tenant_code, "saved": true, "claim_id": 1, "payload": payload}))
}

async fn run_leaf_action(
    Path(tenant_code): Path<String>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    Json(json!({
        "tenant_code": tenant_code,
        "leaf_key": payload.get("leaf_key").cloned().unwrap_or(Value::Null),
        "status": "OK",
        "action": "executed",
        "payload": payload
    }))
}

async fn get_industry_stats_report(Path(tenant_code): Path<String>) -> Json<Value> {
    Json(json!([
        {"tenant_code": tenant_code, "industry_code": "62010", "company_count": 8, "effective_tax_rate_bps": 1425},
        {"tenant_code": tenant_code, "industry_code": " 제조", "company_count": 5, "effective_tax_rate_bps": 1680}
    ]))
}

async fn list_custom_reports_v13(Path(tenant_code): Path<String>) -> Json<Value> {
    Json(json!([
        {"tenant_code": tenant_code, "report_id": 1, "report_name": "유보 및 세부담", "column_count": 6}
    ]))
}

async fn save_custom_report_v13(
    Path(tenant_code): Path<String>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    Json(json!({"tenant_code": tenant_code, "saved": true, "report_id": 1, "payload": payload}))
}

async fn get_custom_report_v13(Path((tenant_code, report_id)): Path<(String, i64)>) -> Json<Value> {
    Json(json!({
        "tenant_code": tenant_code,
        "report_id": report_id,
        "columns": ["customer", "year", "tax_due"],
        "rows": [{"customer": "Demo Corp", "year": 2026, "tax_due": 12000000}]
    }))
}

async fn list_workflow_events_v13(
    Path(tenant_code): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    Json(json!([
        {"tenant_code": tenant_code, "status": query.get("status").cloned().unwrap_or_else(|| "REJECTED".to_string()), "event": "REJECTED", "actor": "reviewer01"}
    ]))
}

async fn list_business_year_workflow_events(
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> Json<Value> {
    Json(json!([
        {"tenant_code": tenant_code, "by_id": by_id, "event": "REQUESTED", "actor": "admin"},
        {"tenant_code": tenant_code, "by_id": by_id, "event": "APPROVED", "actor": "reviewer01"}
    ]))
}

async fn request_business_year_workflow(
    Path((tenant_code, by_id)): Path<(String, i64)>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    Json(json!({"tenant_code": tenant_code, "by_id": by_id, "requested": true, "payload": payload}))
}

async fn get_amendment_version_mode(
    Path((tenant_code, by_id)): Path<(String, i64)>,
) -> Json<Value> {
    Json(json!({
        "tenant_code": tenant_code,
        "by_id": by_id,
        "mode": "AMENDMENT",
        "versions": [{"version": 1, "label": "original"}, {"version": 2, "label": "latest"}]
    }))
}

async fn resubmit_business_year(
    Path((tenant_code, by_id)): Path<(String, i64)>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    Json(
        json!({"tenant_code": tenant_code, "by_id": by_id, "resubmitted": true, "payload": payload}),
    )
}

async fn list_validation_issues(Path((tenant_code, by_id)): Path<(String, i64)>) -> Json<Value> {
    Json(json!([
        {"tenant_code": tenant_code, "by_id": by_id, "issue_id": 1, "severity": "WARN", "message": "demo issue"}
    ]))
}

async fn list_print_history_v13(Path((tenant_code, by_id)): Path<(String, i64)>) -> Json<Value> {
    Json(json!([
        {"tenant_code": tenant_code, "by_id": by_id, "form_code": "FORM3", "printed_by": "admin", "status": "PRINTED"}
    ]))
}

async fn get_forms_linkage_check(Path((tenant_code, by_id)): Path<(String, i64)>) -> Json<Value> {
    Json(json!({
        "tenant_code": tenant_code,
        "by_id": by_id,
        "balanced": true,
        "differences": [
            {"source": "FORM3.total_tax_due", "target": "FORM15.tax_due", "delta": 0}
        ]
    }))
}

async fn get_latest_efiling_v13(Path((tenant_code, by_id)): Path<(String, i64)>) -> Json<Value> {
    Json(json!({
        "tenant_code": tenant_code,
        "by_id": by_id,
        "efiling_id": 1,
        "status": "ACCEPTED",
        "receipt_no": "R-2026-0001"
    }))
}

async fn get_efiling_v13(
    Path((tenant_code, by_id, efiling_id)): Path<(String, i64, i64)>,
) -> Json<Value> {
    Json(json!({
        "tenant_code": tenant_code,
        "by_id": by_id,
        "efiling_id": efiling_id,
        "status": "ACCEPTED",
        "checksum": "demo-checksum"
    }))
}

async fn submit_efiling_v13(
    Path((tenant_code, by_id, efiling_id)): Path<(String, i64, i64)>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    Json(json!({
        "tenant_code": tenant_code,
        "by_id": by_id,
        "efiling_id": efiling_id,
        "submitted": true,
        "receipt_no": "R-2026-0001",
        "payload": payload
    }))
}

#[derive(Deserialize)]
struct JobQuery {
    status: Option<String>,
}

async fn enqueue_job(
    State(state): State<AppState>,
    Json(request): Json<EnqueueJobRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::Job>)> {
    let job = queue::enqueue(
        &state.pool,
        &request.job_type,
        request.payload,
        request.max_attempts.unwrap_or(3),
    )
    .await
    .map_err(map_anyhow)?;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

async fn list_jobs(
    State(state): State<AppState>,
    Query(query): Query<JobQuery>,
) -> AppResult<Json<Vec<crate::domain::Job>>> {
    let jobs = queue::list_jobs(&state.pool, query.status.as_deref())
        .await
        .map_err(map_anyhow)?;
    Ok(Json(jobs))
}

async fn get_job(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> AppResult<Json<crate::domain::Job>> {
    let job = queue::get_job(&state.pool, job_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(job))
}

async fn retry_job(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> AppResult<Json<crate::domain::Job>> {
    let job = queue::retry_dead_letter(&state.pool, job_id)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(job))
}

fn map_anyhow(error: anyhow::Error) -> AppError {
    let message = format!("{error:#}");
    if message.contains("not found") {
        AppError::not_found(message)
    } else if message.contains("invalid or expired session")
        || message.contains("missing authorization")
        || message.contains("access denied")
    {
        AppError::Unauthorized(message)
    } else if message.contains("duplicate key")
        || message.contains("unique constraint")
        || message.contains("blocked")
        || message.contains("locked after FILED")
    {
        AppError::Conflict(message)
    } else if message.contains("invalid")
        || message.contains("unsupported")
        || message.contains("required")
    {
        AppError::bad_request(message)
    } else {
        AppError::Internal(error)
    }
}
