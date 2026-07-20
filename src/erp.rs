use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    db::quote_ident,
    domain::{CreateErpImportRequest, ErpImportEnqueueResponse, ErpImportRun, TenantRef},
    queue, tax_data, tenant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErpVendor {
    Douzone,
    Sap,
    OracleEbs,
}

impl ErpVendor {
    pub fn parse(value: &str) -> Result<Self> {
        let normalized = value.trim().replace(['-', ' '], "_").to_ascii_uppercase();
        match normalized.as_str() {
            "DOUZONE" | "DUZON" | "더존" => Ok(Self::Douzone),
            "SAP" | "SAP_FI" => Ok(Self::Sap),
            "ORACLE" | "ORACLE_EBS" | "EBS" => Ok(Self::OracleEbs),
            other => Err(anyhow!("unsupported ERP vendor {other}")),
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Douzone => "DOUZONE",
            Self::Sap => "SAP",
            Self::OracleEbs => "ORACLE_EBS",
        }
    }

    fn default_source_system(self) -> &'static str {
        match self {
            Self::Douzone => "DOUZONE_MOCK",
            Self::Sap => "SAP_FI_MOCK",
            Self::OracleEbs => "ORACLE_EBS_GL_MOCK",
        }
    }
}

pub fn supported_vendors() -> Vec<&'static str> {
    vec![
        ErpVendor::Douzone.code(),
        ErpVendor::Sap.code(),
        ErpVendor::OracleEbs.code(),
    ]
}

#[derive(Debug, Clone)]
pub struct ErpImportRequest {
    pub tenant_code: String,
    pub by_id: i64,
    pub run_id: i64,
    pub vendor: ErpVendor,
    pub source_system: String,
    pub mock_profile: String,
    pub attempt: i32,
}

#[derive(Debug, Clone)]
pub struct ErpFinancialLine {
    pub statement_type: &'static str,
    pub account_code: String,
    pub account_name: &'static str,
    pub debit: i64,
    pub credit: i64,
    pub standard_account_code: &'static str,
    pub standard_account_name: &'static str,
}

pub trait ErpImportAdapter {
    fn vendor(&self) -> ErpVendor;
    fn fetch_financial_statements(
        &self,
        request: &ErpImportRequest,
    ) -> Result<Vec<ErpFinancialLine>>;
}

#[derive(Debug, Clone, Copy)]
struct MockErpAdapter {
    vendor: ErpVendor,
}

impl ErpImportAdapter for MockErpAdapter {
    fn vendor(&self) -> ErpVendor {
        self.vendor
    }

    fn fetch_financial_statements(
        &self,
        request: &ErpImportRequest,
    ) -> Result<Vec<ErpFinancialLine>> {
        match request.mock_profile.as_str() {
            "FAIL" => Err(anyhow!(
                "{} mock adapter forced failure for run {}",
                self.vendor().code(),
                request.run_id
            )),
            "FAIL_ONCE" if request.attempt <= 1 => Err(anyhow!(
                "{} mock adapter forced transient failure for run {}",
                self.vendor().code(),
                request.run_id
            )),
            "BALANCED" | "FAIL_ONCE" => Ok(mock_balanced_lines(self.vendor())),
            other => Err(anyhow!("unsupported ERP mock profile {other}")),
        }
    }
}

pub async fn enqueue_import(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    by_id: i64,
    request: CreateErpImportRequest,
    actor: &str,
) -> Result<ErpImportEnqueueResponse> {
    tenant::ensure_business_year_editable(pool, tenant_ref, by_id, "erp-import").await?;
    let vendor = normalize_vendor(&request.vendor)?;
    let source_system = request
        .source_system
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| vendor.default_source_system().to_string());
    let mock_profile = normalize_mock_profile(request.mock_profile.as_deref());
    let run = insert_import_run(
        pool,
        tenant_ref,
        by_id,
        vendor,
        &source_system,
        &mock_profile,
    )
    .await?;
    let max_attempts = request.max_attempts.unwrap_or(3).clamp(1, 10);
    let job = queue::enqueue(
        pool,
        "erp_import",
        job_payload(
            &tenant_ref.tenant_code,
            by_id,
            run.run_id,
            vendor,
            &source_system,
            &mock_profile,
            actor,
        ),
        max_attempts,
    )
    .await?;
    let run = attach_job(pool, tenant_ref, run.run_id, job.job_id).await?;
    append_audit(
        pool,
        tenant_ref,
        run.run_id,
        "ERP_IMPORT_ENQUEUED",
        json!({
            "vendor": vendor.code(),
            "source_system": source_system,
            "mock_profile": mock_profile,
            "job_id": job.job_id,
        }),
        actor,
    )
    .await?;
    Ok(ErpImportEnqueueResponse { run, job })
}

pub async fn process_job(
    pool: &PgPool,
    payload: &Value,
    attempt: i32,
    max_attempts: i32,
) -> Result<Value> {
    let tenant_code = payload_string(payload, "tenant_code")?;
    let by_id = payload_i64(payload, "by_id")?;
    let run_id = payload_i64(payload, "run_id")?;
    let actor = payload
        .get("actor")
        .and_then(Value::as_str)
        .unwrap_or("erp-worker");
    let tenant_ref = tenant::resolve_tenant(pool, &tenant_code).await?;
    let run = get_import_run(pool, &tenant_ref, run_id).await?;
    if run.by_id != by_id {
        anyhow::bail!(
            "erp_import payload by_id {} does not match run {} by_id {}",
            by_id,
            run_id,
            run.by_id
        );
    }
    let vendor = normalize_vendor(&run.vendor)?;
    let request = ErpImportRequest {
        tenant_code,
        by_id,
        run_id,
        vendor,
        source_system: run.source_system.clone(),
        mock_profile: run
            .mock_profile
            .clone()
            .unwrap_or_else(|| "BALANCED".to_string()),
        attempt,
    };
    mark_running(pool, &tenant_ref, run_id, attempt).await?;
    match execute_import(pool, &tenant_ref, &request).await {
        Ok(result) => {
            let run = mark_imported(
                pool,
                &tenant_ref,
                run_id,
                attempt,
                result.batch.batch_id,
                result.batch.row_count,
                result.batch.valid_count,
                result.batch.error_count,
                json!({
                    "stdMapRate": result.std_map_rate,
                    "source_system": request.source_system,
                }),
            )
            .await?;
            append_audit(
                pool,
                &tenant_ref,
                run_id,
                "ERP_IMPORT_SUCCEEDED",
                json!({
                    "vendor": request.vendor.code(),
                    "attempt": attempt,
                    "batch_id": result.batch.batch_id,
                    "valid_count": result.batch.valid_count,
                    "error_count": result.batch.error_count,
                }),
                actor,
            )
            .await?;
            Ok(json!({
                "run": run,
                "batch": result.batch,
                "errors": result.errors,
                "stdMapRate": result.std_map_rate,
            }))
        }
        Err(error) => {
            let message = format!("{error:#}");
            let terminal = attempt >= max_attempts;
            mark_failed(pool, &tenant_ref, run_id, attempt, &message, terminal).await?;
            append_audit(
                pool,
                &tenant_ref,
                run_id,
                if terminal {
                    "ERP_IMPORT_FAILED"
                } else {
                    "ERP_IMPORT_RETRYING"
                },
                json!({
                    "vendor": request.vendor.code(),
                    "attempt": attempt,
                    "terminal": terminal,
                    "error": message,
                }),
                actor,
            )
            .await?;
            Err(error)
        }
    }
}

pub fn job_payload(
    tenant_code: &str,
    by_id: i64,
    run_id: i64,
    vendor: ErpVendor,
    source_system: &str,
    mock_profile: &str,
    actor: &str,
) -> Value {
    json!({
        "tenant_code": tenant_code,
        "by_id": by_id,
        "run_id": run_id,
        "vendor": vendor.code(),
        "source_system": source_system,
        "mock_profile": mock_profile,
        "actor": actor,
    })
}

pub async fn list_import_runs(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    by_id: i64,
) -> Result<Vec<ErpImportRun>> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        SELECT run_id, by_id, vendor, source_system, adapter_kind, mock_profile,
               status, attempt_count, last_error, job_id, import_batch_id,
               row_count, valid_count, error_count, metadata, created_at,
               updated_at, completed_at
        FROM {schema}.erp_import_runs
        WHERE by_id = $1
        ORDER BY created_at DESC, run_id DESC
        "#
    );
    sqlx::query_as::<_, ErpImportRun>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list ERP import runs")
}

pub async fn get_import_run(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    run_id: i64,
) -> Result<ErpImportRun> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        SELECT run_id, by_id, vendor, source_system, adapter_kind, mock_profile,
               status, attempt_count, last_error, job_id, import_batch_id,
               row_count, valid_count, error_count, metadata, created_at,
               updated_at, completed_at
        FROM {schema}.erp_import_runs
        WHERE run_id = $1
        "#
    );
    sqlx::query_as::<_, ErpImportRun>(&sql)
        .bind(run_id)
        .fetch_one(pool)
        .await
        .context("ERP import run not found")
}

async fn execute_import(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    request: &ErpImportRequest,
) -> Result<crate::domain::TaxDataImportResponse> {
    let adapter = MockErpAdapter {
        vendor: request.vendor,
    };
    let lines = adapter.fetch_financial_statements(request)?;
    let csv = financial_lines_csv(&lines)?;
    tax_data::import_tax_data(
        pool,
        tenant_ref,
        request.by_id,
        "financial-statements",
        Some(format!(
            "erp-{}-run-{}.csv",
            request.vendor.code().to_ascii_lowercase(),
            request.run_id
        )),
        csv.as_bytes(),
    )
    .await
}

fn mock_balanced_lines(vendor: ErpVendor) -> Vec<ErpFinancialLine> {
    let account_codes = match vendor {
        ErpVendor::Douzone => [
            "DZ-10100", "DZ-20100", "DZ-30100", "DZ-40100", "DZ-50100", "DZ-51100",
        ],
        ErpVendor::Sap => [
            "SAP-100000",
            "SAP-200000",
            "SAP-300000",
            "SAP-400000",
            "SAP-500000",
            "SAP-510000",
        ],
        ErpVendor::OracleEbs => [
            "EBS-01-101",
            "EBS-02-201",
            "EBS-03-301",
            "EBS-04-401",
            "EBS-05-501",
            "EBS-05-511",
        ],
    };
    vec![
        line("BS", account_codes[0], "Cash", 1_000, 0, "STD_CASH", "Cash"),
        line(
            "BS",
            account_codes[1],
            "Accounts payable",
            0,
            400,
            "STD_PAYABLE",
            "Accounts payable",
        ),
        line(
            "BS",
            account_codes[2],
            "Capital",
            0,
            600,
            "STD_CAPITAL",
            "Capital",
        ),
        line(
            "IS",
            account_codes[3],
            "Revenue",
            0,
            600,
            "STD_PRODUCT_REVENUE",
            "Revenue",
        ),
        line(
            "IS",
            account_codes[4],
            "Cost of goods sold",
            400,
            0,
            "STD_COGS",
            "Cost of goods sold",
        ),
        line(
            "IS",
            account_codes[5],
            "Salary expense",
            200,
            0,
            "STD_SALARY",
            "Salary expense",
        ),
    ]
}

fn line(
    statement_type: &'static str,
    account_code: &str,
    account_name: &'static str,
    debit: i64,
    credit: i64,
    standard_account_code: &'static str,
    standard_account_name: &'static str,
) -> ErpFinancialLine {
    ErpFinancialLine {
        statement_type,
        account_code: account_code.to_string(),
        account_name,
        debit,
        credit,
        standard_account_code,
        standard_account_name,
    }
}

fn financial_lines_csv(lines: &[ErpFinancialLine]) -> Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "statement_type",
        "account_code",
        "account_name",
        "debit",
        "credit",
        "standard_account_code",
        "standard_account_name",
    ])?;
    for line in lines {
        writer.write_record([
            line.statement_type,
            &line.account_code,
            line.account_name,
            &line.debit.to_string(),
            &line.credit.to_string(),
            line.standard_account_code,
            line.standard_account_name,
        ])?;
    }
    let bytes = writer.into_inner().context("failed to write ERP CSV")?;
    String::from_utf8(bytes).context("ERP CSV is not UTF-8")
}

async fn insert_import_run(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    by_id: i64,
    vendor: ErpVendor,
    source_system: &str,
    mock_profile: &str,
) -> Result<ErpImportRun> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.erp_import_runs (
            by_id, vendor, source_system, adapter_kind, mock_profile, status, metadata
        )
        VALUES ($1, $2, $3, 'MOCK', $4, 'QUEUED', $5)
        RETURNING run_id, by_id, vendor, source_system, adapter_kind, mock_profile,
                  status, attempt_count, last_error, job_id, import_batch_id,
                  row_count, valid_count, error_count, metadata, created_at,
                  updated_at, completed_at
        "#
    );
    sqlx::query_as::<_, ErpImportRun>(&sql)
        .bind(by_id)
        .bind(vendor.code())
        .bind(source_system)
        .bind(mock_profile)
        .bind(json!({ "supported_vendors": supported_vendors() }))
        .fetch_one(pool)
        .await
        .context("failed to create ERP import run")
}

async fn attach_job(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    run_id: i64,
    job_id: Uuid,
) -> Result<ErpImportRun> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.erp_import_runs
        SET job_id = $2,
            updated_at = NOW()
        WHERE run_id = $1
        RETURNING run_id, by_id, vendor, source_system, adapter_kind, mock_profile,
                  status, attempt_count, last_error, job_id, import_batch_id,
                  row_count, valid_count, error_count, metadata, created_at,
                  updated_at, completed_at
        "#
    );
    sqlx::query_as::<_, ErpImportRun>(&sql)
        .bind(run_id)
        .bind(job_id)
        .fetch_one(pool)
        .await
        .context("failed to attach ERP job")
}

async fn mark_running(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    run_id: i64,
    attempt: i32,
) -> Result<ErpImportRun> {
    update_run_status(
        pool, tenant_ref, run_id, "RUNNING", attempt, None, None, None,
    )
    .await
}

async fn mark_imported(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    run_id: i64,
    attempt: i32,
    batch_id: i64,
    row_count: i32,
    valid_count: i32,
    error_count: i32,
    metadata: Value,
) -> Result<ErpImportRun> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.erp_import_runs
        SET status = 'IMPORTED',
            attempt_count = $2,
            last_error = NULL,
            import_batch_id = $3,
            row_count = $4,
            valid_count = $5,
            error_count = $6,
            metadata = COALESCE(metadata, '{{}}'::jsonb) || $7::jsonb,
            completed_at = NOW(),
            updated_at = NOW()
        WHERE run_id = $1
        RETURNING run_id, by_id, vendor, source_system, adapter_kind, mock_profile,
                  status, attempt_count, last_error, job_id, import_batch_id,
                  row_count, valid_count, error_count, metadata, created_at,
                  updated_at, completed_at
        "#
    );
    sqlx::query_as::<_, ErpImportRun>(&sql)
        .bind(run_id)
        .bind(attempt)
        .bind(batch_id)
        .bind(row_count)
        .bind(valid_count)
        .bind(error_count)
        .bind(metadata)
        .fetch_one(pool)
        .await
        .context("failed to mark ERP import succeeded")
}

async fn mark_failed(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    run_id: i64,
    attempt: i32,
    error: &str,
    terminal: bool,
) -> Result<ErpImportRun> {
    update_run_status(
        pool,
        tenant_ref,
        run_id,
        if terminal { "FAILED" } else { "RETRYING" },
        attempt,
        Some(error),
        None,
        Some(json!({ "terminal": terminal })),
    )
    .await
}

async fn update_run_status(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    run_id: i64,
    status: &str,
    attempt: i32,
    last_error: Option<&str>,
    import_batch_id: Option<i64>,
    metadata: Option<Value>,
) -> Result<ErpImportRun> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let metadata = metadata.unwrap_or_else(|| json!({}));
    let sql = format!(
        r#"
        UPDATE {schema}.erp_import_runs
        SET status = $2,
            attempt_count = $3,
            last_error = $4,
            import_batch_id = COALESCE($5, import_batch_id),
            metadata = COALESCE(metadata, '{{}}'::jsonb) || $6::jsonb,
            completed_at = CASE WHEN $2 IN ('FAILED', 'IMPORTED') THEN NOW() ELSE completed_at END,
            updated_at = NOW()
        WHERE run_id = $1
        RETURNING run_id, by_id, vendor, source_system, adapter_kind, mock_profile,
                  status, attempt_count, last_error, job_id, import_batch_id,
                  row_count, valid_count, error_count, metadata, created_at,
                  updated_at, completed_at
        "#
    );
    sqlx::query_as::<_, ErpImportRun>(&sql)
        .bind(run_id)
        .bind(status)
        .bind(attempt)
        .bind(last_error)
        .bind(import_batch_id)
        .bind(metadata)
        .fetch_one(pool)
        .await
        .context("failed to update ERP import run")
}

async fn append_audit(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    run_id: i64,
    action: &str,
    new_data: Value,
    actor: &str,
) -> Result<()> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        WITH prev AS (
            SELECT hash_current
            FROM {schema}.audit_logs
            WHERE event_date = CURRENT_DATE
            ORDER BY audit_id DESC
            LIMIT 1
        )
        INSERT INTO {schema}.audit_logs (
            table_name, record_id, action, old_data, new_data, changed_by,
            event_date, prev_hash, hash_current
        )
        SELECT 'erp_import_runs', $1, $2, NULL, $3, $4, CURRENT_DATE,
               prev.hash_current,
               md5(COALESCE(prev.hash_current, '') || 'erp_import_runs' || $1 || $2 ||
                   COALESCE($3::jsonb::text, '') || $4)
        FROM (SELECT 1) seed
        LEFT JOIN prev ON TRUE
        "#
    );
    sqlx::query(&sql)
        .bind(run_id.to_string())
        .bind(action)
        .bind(new_data)
        .bind(actor)
        .execute(pool)
        .await
        .context("failed to insert ERP audit log")?;
    Ok(())
}

fn normalize_vendor(value: &str) -> Result<ErpVendor> {
    ErpVendor::parse(value)
}

fn normalize_mock_profile(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .unwrap_or("BALANCED")
        .replace(['-', ' '], "_")
        .to_ascii_uppercase()
}

fn payload_string(payload: &Value, field: &str) -> Result<String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| anyhow!("erp_import requires {field}"))
}

fn payload_i64(payload: &Value, field: &str) -> Result<i64> {
    payload
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("erp_import requires {field}"))
}
