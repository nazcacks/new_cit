use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{
        header::AUTHORIZATION,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderMap, HeaderValue, Response, StatusCode,
    },
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

use crate::{
    auth,
    domain::{
        CalculateAdjustmentRequest, CreateBusinessYearRequest, CreateCustomerRequest,
        CreateLawAmendmentRequest, CreateTaxLawRequest, CreateTaxLimitRequest,
        CreateTaxRateRequest, CreateTenantRequest, EnqueueEfilingRequest, EnqueueJobRequest,
        HealthResponse, LawVersioningImpactRequest, LoginRequest, UpdateTaxLawStatusRequest,
    },
    efiling,
    error::{AppError, AppResult},
    modules, queue,
    state::AppState,
    tax, tenant, web,
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
            "/api/tenants/:tenant_code/customers",
            get(list_customers).post(create_customer),
        )
        .route(
            "/api/tenants/:tenant_code/business-years",
            post(create_business_year),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/snapshot",
            get(get_law_snapshot).post(create_law_snapshot),
        )
        .route(
            "/api/tenants/:tenant_code/business-years/:by_id/adjustments",
            get(list_adjustments).post(calculate_adjustments),
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
    Ok((StatusCode::CREATED, Json(by)))
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
    } else if message.contains("invalid") || message.contains("unsupported") {
        AppError::bad_request(message)
    } else {
        AppError::Internal(error)
    }
}
