use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub tenant_code: String,
    pub login_id: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AuthUser {
    pub user_id: i64,
    pub tenant_id: i64,
    pub tenant_code: String,
    pub tenant_name: String,
    pub schema_name: String,
    pub login_id: String,
    pub user_name: String,
    pub email: Option<String>,
    pub status: String,
    pub use_2fa: bool,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
    pub token: Uuid,
    pub token_type: &'static str,
    pub expires_at: DateTime<Utc>,
    pub user: AuthUser,
    pub modules: Value,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Tenant {
    pub tenant_id: i64,
    pub tenant_code: String,
    pub tenant_name: String,
    pub biz_reg_no: String,
    pub contract_start: NaiveDate,
    pub contract_end: Option<NaiveDate>,
    pub schema_name: String,
    pub status: String,
    pub allowed_ips: Option<String>,
    pub max_users: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TenantRef {
    pub tenant_id: i64,
    pub tenant_code: String,
    pub schema_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub tenant_code: String,
    pub tenant_name: String,
    pub biz_reg_no: String,
    pub contract_start: NaiveDate,
    pub contract_end: Option<NaiveDate>,
    pub allowed_ips: Option<String>,
    pub max_users: Option<i32>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Customer {
    pub customer_id: i64,
    pub tenant_id: i64,
    pub customer_code: String,
    pub customer_name: String,
    pub biz_reg_no: String,
    pub corp_reg_no: Option<String>,
    pub industry_code: Option<String>,
    pub is_sme: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCustomerRequest {
    pub customer_code: String,
    pub customer_name: String,
    pub biz_reg_no: String,
    pub corp_reg_no: Option<String>,
    pub industry_code: Option<String>,
    pub is_sme: Option<bool>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BusinessYear {
    pub by_id: i64,
    pub customer_id: i64,
    pub year_label: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
    pub locked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBusinessYearRequest {
    pub customer_id: i64,
    pub year_label: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TaxLawVersion {
    pub law_version_id: i64,
    pub version_code: String,
    pub law_name: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub status: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaxLawRequest {
    pub version_code: String,
    pub law_name: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TaxRate {
    pub tax_rate_id: i64,
    pub law_version_id: i64,
    pub item_code: String,
    pub taxable_from: i64,
    pub taxable_to: Option<i64>,
    pub base_tax: i64,
    pub rate_bps: i32,
    pub progressive_deduction: i64,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub metadata: Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaxRateRequest {
    pub law_version_id: i64,
    pub item_code: String,
    pub taxable_from: i64,
    pub taxable_to: Option<i64>,
    pub base_tax: Option<i64>,
    pub rate_bps: i32,
    pub progressive_deduction: Option<i64>,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TaxLimit {
    pub tax_limit_id: i64,
    pub law_version_id: i64,
    pub item_code: String,
    pub amount: i64,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub metadata: Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaxLimitRequest {
    pub law_version_id: i64,
    pub item_code: String,
    pub amount: i64,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaxLawStatusRequest {
    pub status: String,
    pub change_summary: Option<String>,
    pub approved_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LawAmendmentHistory {
    pub amendment_id: i64,
    pub law_version_id: i64,
    pub change_summary: String,
    pub approved_by: String,
    pub approved_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLawAmendmentRequest {
    pub law_version_id: i64,
    pub change_summary: String,
    pub approved_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LawVersioningImpactRequest {
    pub law_version_id: i64,
    pub include_locked: Option<bool>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LawSnapshot {
    pub snapshot_id: i64,
    pub by_id: i64,
    pub law_version_id: i64,
    pub rate_version_ids: Value,
    pub form_version_ids: Value,
    pub efile_master_ids: Value,
    pub snapshot_data: Value,
    pub locked: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CalculateAdjustmentRequest {
    pub accounting_income: i64,
    pub gross_revenue: Option<i64>,
    pub donations: Option<i64>,
    pub entertainment_expense: Option<i64>,
    pub depreciation_book: Option<i64>,
    pub depreciation_tax_limit: Option<i64>,
    pub carryforward_loss: Option<i64>,
    pub tax_credits: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationResult {
    pub accounting_income: i64,
    pub addbacks: i64,
    pub deductions: i64,
    pub taxable_income: i64,
    pub corporate_tax: i64,
    pub local_income_tax: i64,
    pub tax_credits: i64,
    pub total_tax_due: i64,
    pub snapshot_id: i64,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TaxAdjustment {
    pub adjustment_id: i64,
    pub by_id: i64,
    pub adj_category: String,
    pub adj_code: String,
    pub amount: i64,
    pub direction: String,
    pub description: Option<String>,
    pub snapshot_id: Option<i64>,
    pub metadata: Value,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FormData {
    pub form_data_id: i64,
    pub by_id: i64,
    pub form_code: String,
    pub form_version_id: i64,
    pub data_json: Value,
    pub snapshot_id: Option<i64>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct EfilingHistory {
    pub efiling_id: i64,
    pub by_id: i64,
    pub efile_master_id: i64,
    pub status: String,
    pub total_records: i32,
    pub checksum: String,
    pub created_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EfilingFile {
    pub file_id: i64,
    pub efiling_id: i64,
    pub file_name: String,
    pub encoding: String,
    pub contents: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Job {
    pub job_id: Uuid,
    pub job_type: String,
    pub payload: Value,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_run_at: DateTime<Utc>,
    pub locked_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub result: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub dead_lettered_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct EnqueueJobRequest {
    pub job_type: String,
    pub payload: Value,
    pub max_attempts: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct EnqueueEfilingRequest {
    pub max_attempts: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
}
