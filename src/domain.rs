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

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Role {
    pub role_code: String,
    pub role_name: String,
    pub description: Option<String>,
    pub system_role: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RolePermission {
    pub role_code: String,
    pub module_code: String,
    pub function_code: String,
    pub effect: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RolePermissionInput {
    pub module_code: String,
    pub function_code: String,
    pub effect: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRolePermissionsRequest {
    pub permissions: Vec<RolePermissionInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserCustomerAccess {
    pub customer_id: i64,
    pub access_level: String,
    pub is_primary: bool,
    pub valid_from: Option<NaiveDate>,
    pub valid_to: Option<NaiveDate>,
    pub work_scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminUser {
    pub user_id: i64,
    pub tenant_id: i64,
    pub tenant_code: String,
    pub login_id: String,
    pub user_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub status: String,
    pub locked: bool,
    pub use_2fa: bool,
    pub pwd_fail_count: i32,
    pub last_login_at: Option<DateTime<Utc>>,
    pub roles: Vec<String>,
    pub customer_access: Vec<UserCustomerAccess>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserCustomerAccessInput {
    pub customer_id: i64,
    pub access_level: Option<String>,
    pub is_primary: Option<bool>,
    pub valid_from: Option<NaiveDate>,
    pub valid_to: Option<NaiveDate>,
    pub work_scopes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAdminUserRequest {
    pub login_id: String,
    pub password: String,
    pub user_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub use_2fa: Option<bool>,
    pub status: Option<String>,
    pub roles: Option<Vec<String>>,
    pub customer_access: Option<Vec<UserCustomerAccessInput>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAdminUserRequest {
    pub user_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub use_2fa: Option<bool>,
    pub roles: Option<Vec<String>>,
    pub customer_access: Option<Vec<UserCustomerAccessInput>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAdminUserStatusRequest {
    pub status: Option<String>,
    pub locked: Option<bool>,
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
    pub work_scopes: Vec<String>,
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
    pub work_scopes: Option<Vec<String>>,
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

#[derive(Debug, Deserialize)]
pub struct UpdateBusinessYearStatusRequest {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ImportBatch {
    pub batch_id: i64,
    pub by_id: i64,
    pub customer_id: Option<i64>,
    pub data_type: String,
    pub source_file_name: Option<String>,
    pub row_count: i32,
    pub valid_count: i32,
    pub error_count: i32,
    pub auto_mapped_count: i32,
    pub status: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ImportError {
    pub error_id: i64,
    pub batch_id: i64,
    pub row_no: i32,
    pub field_name: Option<String>,
    pub severity: String,
    pub message: String,
    pub raw_row: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaxDataImportResponse {
    pub batch: ImportBatch,
    pub errors: Vec<ImportError>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AccountMapping {
    pub mapping_id: i64,
    pub customer_id: i64,
    pub statement_type: String,
    pub source_account_code: String,
    pub source_account_name: String,
    pub standard_account_code: String,
    pub standard_account_name: String,
    pub use_count: i32,
    pub last_used_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccountMappingRequest {
    pub statement_type: Option<String>,
    pub source_account_code: String,
    pub source_account_name: String,
    pub standard_account_code: String,
    pub standard_account_name: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FinancialStatementLine {
    pub line_id: i64,
    pub fs_id: i64,
    pub batch_id: Option<i64>,
    pub statement_type: String,
    pub row_no: Option<i32>,
    pub account_code: String,
    pub account_name: String,
    pub standard_account_code: Option<String>,
    pub standard_account_name: Option<String>,
    pub amount: i64,
    pub debit_credit: String,
    pub debit: i64,
    pub credit: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AssetRecord {
    pub asset_id: i64,
    pub by_id: i64,
    pub batch_id: Option<i64>,
    pub asset_code: String,
    pub asset_name: String,
    pub asset_category: String,
    pub is_business_vehicle: bool,
    pub acquisition_date: NaiveDate,
    pub acquisition_cost: i64,
    pub useful_life_years: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TransactionRecord {
    pub transaction_id: i64,
    pub by_id: i64,
    pub batch_id: Option<i64>,
    pub tx_date: NaiveDate,
    pub partner_name: String,
    pub category: String,
    pub account_code: Option<String>,
    pub description: Option<String>,
    pub amount: i64,
    pub evidence_type: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaxDataValidationSummary {
    pub by_id: i64,
    pub debit_total: i64,
    pub credit_total: i64,
    pub balanced: bool,
    pub fs_line_count: i64,
    pub unresolved_mapping_count: i64,
    pub asset_count: i64,
    pub business_vehicle_count: i64,
    pub transaction_count: i64,
    pub batch_error_count: i64,
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

#[derive(Debug, Clone, Deserialize)]
pub struct IncomeAdjustmentItemInput {
    pub section: String,
    pub item_code: String,
    pub item_name: String,
    pub amount: i64,
    pub disposition: Option<String>,
    pub temporary: Option<bool>,
    pub law_ref: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateIncomeAdjustmentRequest {
    pub accounting_income: Option<i64>,
    pub items: Vec<IncomeAdjustmentItemInput>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AdjustmentItem {
    pub adjustment_item_id: i64,
    pub by_id: i64,
    pub adjustment_id: Option<i64>,
    pub section: String,
    pub item_code: String,
    pub item_name: String,
    pub amount: i64,
    pub direction: String,
    pub disposition: String,
    pub source_module: String,
    pub law_ref: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ReserveRecord {
    pub reserve_id: i64,
    pub by_id: i64,
    pub adjustment_id: Option<i64>,
    pub reserve_code: String,
    pub amount: i64,
    pub direction: String,
    pub carryforward_to: Option<i32>,
    pub source_module: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IncomeAdjustmentResult {
    pub accounting_income: i64,
    pub gross_income_inclusion: i64,
    pub gross_income_exclusion: i64,
    pub loss_inclusion: i64,
    pub loss_disallowance: i64,
    pub addbacks: i64,
    pub deductions: i64,
    pub taxable_income: i64,
    pub snapshot_id: i64,
    pub law_banner: Value,
    pub items: Vec<AdjustmentItem>,
    pub reserves_created: Vec<ReserveRecord>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetBasedAdjustmentRequest {
    pub book_reserve: Option<i64>,
    pub estimated_liability: Option<i64>,
    pub external_fund: Option<i64>,
    pub receivable_balance: Option<i64>,
    pub rate_bps: Option<i32>,
    pub actual_bad_debt: Option<i64>,
    pub business_use_bps: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateVehicleUsageLogRequest {
    pub asset_id: i64,
    pub usage_month: NaiveDate,
    pub total_distance_km: f64,
    pub business_distance_km: f64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct VehicleUsageLog {
    pub usage_log_id: i64,
    pub by_id: i64,
    pub asset_id: i64,
    pub usage_month: NaiveDate,
    pub total_distance_km: f64,
    pub business_distance_km: f64,
    pub business_use_bps: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetBasedAdjustmentResult {
    pub module_code: String,
    pub addbacks: i64,
    pub deductions: i64,
    pub snapshot_id: i64,
    pub law_banner: Value,
    pub items: Vec<AdjustmentItem>,
    pub reserves_created: Vec<ReserveRecord>,
    pub details: Value,
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
pub struct TaxForm {
    pub form_id: i64,
    pub form_code: String,
    pub form_name: String,
    pub form_group: Option<String>,
    pub description: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaxFormRequest {
    pub form_code: String,
    pub form_name: String,
    pub form_group: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FormVersion {
    pub form_version_id: i64,
    pub form_code: String,
    pub form_name: String,
    pub version_no: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub template_json: Value,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateFormVersionRequest {
    pub form_code: String,
    pub form_name: String,
    pub version_no: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub template_json: Option<Value>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFormVersionStatusRequest {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FormRelationship {
    pub relationship_id: i64,
    pub source_form: String,
    pub source_field: String,
    pub target_form: String,
    pub target_field: String,
    pub rule_json: Value,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFormRelationshipRequest {
    pub source_form: String,
    pub source_field: String,
    pub target_form: String,
    pub target_field: String,
    pub rule_json: Option<Value>,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveFormVersionQuery {
    pub tenant_code: String,
    pub by_id: i64,
    pub form_code: String,
}

#[derive(Debug, Deserialize)]
pub struct FormMigrationRequest {
    pub tenant_code: String,
    pub by_id: i64,
    pub form_code: String,
    pub to_version_id: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormMigrationResult {
    pub mode: String,
    pub tenant_code: String,
    pub by_id: i64,
    pub form_code: String,
    pub from_version_id: Option<i64>,
    pub to_version_id: i64,
    pub added_fields: Vec<String>,
    pub removed_fields: Vec<String>,
    pub common_fields: Vec<String>,
    pub executable: bool,
    pub message: String,
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
