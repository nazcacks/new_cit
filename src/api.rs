use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{
        header::AUTHORIZATION,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderMap, HeaderValue, Response, StatusCode,
    },
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

use crate::{
    admin, auth,
    domain::{
        AssetBasedAdjustmentRequest, CalculateAdjustmentRequest, CreateAccountMappingRequest,
        CreateAdminUserRequest, CreateBusinessYearRequest, CreateCustomerRequest,
        CreateFormRelationshipRequest, CreateFormVersionRequest, CreateIncomeAdjustmentRequest,
        CreateLawAmendmentRequest, CreateTaxFormRequest, CreateTaxLawRequest,
        CreateTaxLimitRequest, CreateTaxRateRequest, CreateTenantRequest,
        CreateVehicleUsageLogRequest, EnqueueEfilingRequest, EnqueueJobRequest,
        FormMigrationRequest, HealthResponse, LawVersioningImpactRequest, LoginRequest,
        ResolveFormVersionQuery, TransactionBasedAdjustmentRequest, UpdateAdminUserRequest,
        UpdateAdminUserStatusRequest, UpdateBusinessYearStatusRequest,
        UpdateFormVersionStatusRequest, UpdateRolePermissionsRequest, UpdateTaxLawStatusRequest,
    },
    efiling,
    error::{AppError, AppResult},
    forms, modules, queue,
    state::AppState,
    tax, tax_data, tenant, web,
};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(web::index))
        .route("/app.css", get(web::app_css))
        .route("/app.js", get(web::app_js))
        .route("/health", get(health))
        .route("/api/auth/login", post(login))
        .route("/api/auth/me", get(me))
        .route("/api/auth/logout", post(logout))
        .route("/api/modules/tree", get(get_module_tree))
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
        .route(
            "/api/admin/roles/:role_code/permissions",
            put(replace_role_permissions),
        )
        .route(
            "/api/law-versioning/summary",
            get(get_law_versioning_summary),
        )
        .route("/api/law-versioning/impact", post(simulate_law_impact))
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
            "/api/form-versioning/relationships",
            get(list_form_relationships).post(create_form_relationship),
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
        .route(
            "/api/tenants/:tenant_code/business-years",
            get(list_business_years).post(create_business_year),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/status",
            post(update_business_year_status),
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
            get(get_tax_data_validation),
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
            "/api/tenants/:tenant_code/business-years/:by_id/vehicle-usage-logs",
            get(list_vehicle_usage_logs).post(create_vehicle_usage_log),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/reserves",
            get(list_reserves),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/forms/:form_code",
            get(get_form).post(generate_form),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/efilings",
            get(list_efilings).post(enqueue_efiling),
        )
        .route(
            "/api/tenants/:tenant_code/efilings/:efiling_id/file",
            get(download_efiling_file),
        )
        .route("/api/jobs", get(list_jobs).post(enqueue_job))
        .route("/api/jobs/:job_id", get(get_job))
        .route("/api/jobs/:job_id/retry", post(retry_job))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "cit-system",
    })
}

async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> AppResult<Json<crate::domain::LoginResponse>> {
    match auth::login(&state.pool, request.clone()).await {
        Ok(response) => Ok(Json(response)),
        Err(error) => {
            let message = format!("{error:#}");
            if message.contains("invalid tenant, login id, or password") {
                auth::record_failed_login(&state.pool, &request.tenant_code, &request.login_id)
                    .await
                    .map_err(map_anyhow)?;
                Err(AppError::Unauthorized(message))
            } else {
                Err(map_anyhow(error))
            }
        }
    }
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
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<crate::domain::Customer>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let customers = tenant::list_customers(&state.pool, &tenant_ref)
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
    Path(tenant_code): Path<String>,
) -> AppResult<Json<Vec<crate::domain::BusinessYear>>> {
    let tenant_ref = tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
    let years = tenant::list_business_years(&state.pool, &tenant_ref)
        .await
        .map_err(map_anyhow)?;
    Ok(Json(years))
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

async fn enqueue_efiling(
    State(state): State<AppState>,
    Path((tenant_code, by_id)): Path<(String, i64)>,
    Json(request): Json<EnqueueEfilingRequest>,
) -> AppResult<(StatusCode, Json<crate::domain::Job>)> {
    tenant::resolve_tenant(&state.pool, &tenant_code)
        .await
        .map_err(map_anyhow)?;
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
    {
        AppError::Unauthorized(message)
    } else if message.contains("duplicate key") || message.contains("unique constraint") {
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
