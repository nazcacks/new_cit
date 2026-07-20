use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use encoding_rs::Encoding;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    db::quote_ident,
    domain::{
        EfilingFile, EfilingFormatField, EfilingHistory, EfilingPrecheckResult,
        EfilingValidationIssue, FormData, StdFsValidationResult, TenantRef,
    },
    std_fs, tax, tenant,
};

#[derive(Debug, Serialize)]
pub struct EfilingResult {
    pub efiling_id: i64,
    pub file_id: i64,
    pub file_name: String,
    pub encoding: String,
    pub total_records: i32,
    pub checksum: String,
}

#[derive(Debug, sqlx::FromRow)]
struct CustomerYear {
    customer_name: String,
    biz_reg_no: String,
    year_label: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct EfileMaster {
    efile_master_id: i64,
    master_code: String,
    encoding: String,
}

#[derive(Debug, Clone)]
struct RecordLayoutSpec {
    record_type: String,
    fixed_length: i32,
    fields: Vec<RecordFieldSpec>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct RecordFieldSpec {
    record_type: String,
    fixed_length: i32,
    field_name: String,
    start_pos: i32,
    byte_length: i32,
    data_type: String,
    align: String,
    pad_char: String,
    required: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct EfileValidationRule {
    rule_code: String,
    severity: String,
    field_path: Option<String>,
    message: String,
    rule_json: Value,
}

#[derive(Debug, Clone)]
struct StdFsEfilingContext {
    validation: StdFsValidationResult,
    xml_records: Vec<StdFsXmlRecord>,
    missing_xml_fields: Vec<String>,
    total_mismatches: Vec<String>,
}

#[derive(Debug, Clone)]
struct StdFsXmlRecord {
    stmt_type: String,
    item_code: String,
    xml_field_id: Option<String>,
    amount: i64,
    account_class: Option<String>,
    normal_balance: Option<String>,
    is_subtotal: bool,
}

#[derive(Debug, Clone, Default)]
struct FinancialStatementTotals {
    bs_debit_total: i64,
    is_debit_total: i64,
    is_credit_total: i64,
}

pub async fn generate_efiling(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<EfilingResult> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "efiling").await?;
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = tax::ensure_law_snapshot(pool, tenant, by_id).await?;
    let master = load_efile_master(pool, by.start_date, by.end_date).await?;
    let spec = load_record_spec(pool, master.efile_master_id).await?;
    let customer_year = load_customer_year(pool, tenant, by_id).await?;

    let form3 = match tax::get_form(pool, tenant, by_id, "FORM3").await {
        Ok(form) => form,
        Err(_) => tax::generate_form(pool, tenant, by_id, "FORM3").await?,
    };
    let std_fs_context = load_std_fs_efiling_context(pool, tenant, by_id).await?;

    let contents = build_records(
        &master.encoding,
        &spec,
        &customer_year,
        &form3,
        snapshot.snapshot_id,
        &std_fs_context.xml_records,
    )?;
    let issues = validate_efiling(
        pool,
        master.efile_master_id,
        &spec,
        &customer_year,
        &form3,
        &contents,
        &std_fs_context,
    )
    .await?;
    if let Some(issue) = issues.iter().find(|issue| issue.severity == "ERROR") {
        return Err(anyhow!(
            "e-filing precheck failed: {} {}",
            issue.validation_code,
            issue.message
        ));
    }
    let total_records = count_records(&contents);
    let checksum = checksum(&contents);

    let schema = quote_ident(&tenant.schema_name)?;
    let history_sql = format!(
        r#"
        INSERT INTO {schema}.efiling_history (
            by_id, efile_master_id, status, total_records, checksum
        )
        VALUES ($1, $2, 'GENERATED', $3, $4)
        RETURNING efiling_id, by_id, efile_master_id, status, total_records,
                  checksum, created_at, submitted_at, receipt_no, receipt_at
        "#
    );
    let history = sqlx::query_as::<_, EfilingHistory>(&history_sql)
        .bind(by_id)
        .bind(master.efile_master_id)
        .bind(total_records)
        .bind(&checksum)
        .fetch_one(pool)
        .await
        .context("failed to insert efiling history")?;
    insert_efiling_validations(pool, tenant, history.efiling_id, &issues).await?;

    let file_name = format!(
        "{}_{}_{}.txt",
        tenant.tenant_code, customer_year.year_label, master.master_code
    );
    let file_sql = format!(
        r#"
        INSERT INTO {schema}.efiling_files (efiling_id, file_name, encoding, contents)
        VALUES ($1, $2, $3, $4)
        RETURNING file_id
        "#
    );
    let file_id = sqlx::query_scalar::<_, i64>(&file_sql)
        .bind(history.efiling_id)
        .bind(&file_name)
        .bind(&master.encoding)
        .bind(contents)
        .fetch_one(pool)
        .await
        .context("failed to insert efiling file")?;

    Ok(EfilingResult {
        efiling_id: history.efiling_id,
        file_id,
        file_name,
        encoding: master.encoding,
        total_records,
        checksum,
    })
}

pub async fn precheck_efiling(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<EfilingPrecheckResult> {
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = tax::ensure_law_snapshot(pool, tenant, by_id).await?;
    let master = match load_efile_master(pool, by.start_date, by.end_date).await {
        Ok(master) => master,
        Err(error) if format!("{error:#}").contains("no applicable e-file master") => {
            return Ok(EfilingPrecheckResult {
                tenant_code: tenant.tenant_code.clone(),
                by_id,
                master_code: "-".to_string(),
                encoding: "-".to_string(),
                valid: false,
                record_count: 0,
                checksum_preview: String::new(),
                issues: vec![EfilingValidationIssue {
                    validation_code: "EFILE_MASTER_MISSING".to_string(),
                    severity: "ERROR".to_string(),
                    message: "No applicable e-file master is configured for this business year."
                        .to_string(),
                    field_path: Some("efile_master".to_string()),
                }],
            });
        }
        Err(error) => return Err(error),
    };
    let spec = load_record_spec(pool, master.efile_master_id).await?;
    let customer_year = load_customer_year(pool, tenant, by_id).await?;
    let form3 = match tax::get_form(pool, tenant, by_id, "FORM3").await {
        Ok(form) => form,
        Err(_) => tax::generate_form(pool, tenant, by_id, "FORM3").await?,
    };
    let std_fs_context = load_std_fs_efiling_context(pool, tenant, by_id).await?;
    let contents = build_records(
        &master.encoding,
        &spec,
        &customer_year,
        &form3,
        snapshot.snapshot_id,
        &std_fs_context.xml_records,
    )?;
    let issues = validate_efiling(
        pool,
        master.efile_master_id,
        &spec,
        &customer_year,
        &form3,
        &contents,
        &std_fs_context,
    )
    .await?;
    let valid = issues.iter().all(|issue| issue.severity != "ERROR");
    Ok(EfilingPrecheckResult {
        tenant_code: tenant.tenant_code.clone(),
        by_id,
        master_code: master.master_code,
        encoding: master.encoding,
        valid,
        record_count: count_records(&contents),
        checksum_preview: checksum(&contents),
        issues,
    })
}

pub async fn list_format_spec(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<EfilingFormatField>> {
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let master = load_efile_master(pool, by.start_date, by.end_date).await?;
    sqlx::query_as::<_, EfilingFormatField>(
        r#"
        SELECT m.master_code, m.version_no, m.encoding,
               l.record_type, l.record_name, l.sort_order, l.fixed_length,
               f.field_name, f.start_pos, f.byte_length, f.data_type,
               f.align, f.pad_char, f.required, f.source_path, f.description
        FROM efile_masters m
        JOIN efile_record_layouts l ON l.efile_master_id = m.efile_master_id
        JOIN efile_record_fields f ON f.layout_id = l.layout_id
        WHERE m.efile_master_id = $1
        ORDER BY l.sort_order, f.start_pos
        "#,
    )
    .bind(master.efile_master_id)
    .fetch_all(pool)
    .await
    .context("failed to list e-filing format spec")
}

pub async fn list_efilings(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<EfilingHistory>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT efiling_id, by_id, efile_master_id, status, total_records,
               checksum, created_at, submitted_at, receipt_no, receipt_at
        FROM {schema}.efiling_history
        WHERE by_id = $1
        ORDER BY created_at DESC
        "#
    );
    sqlx::query_as::<_, EfilingHistory>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list efilings")
}

pub async fn get_efiling_history(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    efiling_id: i64,
) -> Result<EfilingHistory> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT efiling_id, by_id, efile_master_id, status, total_records,
               checksum, created_at, submitted_at, receipt_no, receipt_at
        FROM {schema}.efiling_history
        WHERE by_id = $1 AND efiling_id = $2
        "#
    );
    sqlx::query_as::<_, EfilingHistory>(&sql)
        .bind(by_id)
        .bind(efiling_id)
        .fetch_one(pool)
        .await
        .context("efiling history not found")
}

pub async fn latest_efiling(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<EfilingHistory> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT efiling_id, by_id, efile_master_id, status, total_records,
               checksum, created_at, submitted_at, receipt_no, receipt_at
        FROM {schema}.efiling_history
        WHERE by_id = $1
        ORDER BY created_at DESC, efiling_id DESC
        LIMIT 1
        "#
    );
    sqlx::query_as::<_, EfilingHistory>(&sql)
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("efiling history not found")
}

pub async fn submit_efiling(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    efiling_id: i64,
    receipt_no: Option<&str>,
) -> Result<EfilingHistory> {
    let history = get_efiling_history(pool, tenant, by_id, efiling_id).await?;
    let has_blocking_issue = efiling_error_count(pool, tenant, efiling_id).await? > 0;
    if has_blocking_issue {
        return Err(anyhow!("e-filing validation errors block submission"));
    }
    if history.status == "ACCEPTED" {
        return Ok(history);
    }
    if history.status != "GENERATED" {
        return Err(anyhow!("invalid e-filing status for submission"));
    }
    let receipt_no = receipt_no
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("R-{}-{efiling_id:06}", Utc::now().format("%Y%m%d%H%M%S")));
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.efiling_history
        SET status = 'ACCEPTED',
            submitted_at = NOW(),
            receipt_no = $3,
            receipt_at = NOW()
        WHERE by_id = $1 AND efiling_id = $2
        RETURNING efiling_id, by_id, efile_master_id, status, total_records,
                  checksum, created_at, submitted_at, receipt_no, receipt_at
        "#
    );
    sqlx::query_as::<_, EfilingHistory>(&sql)
        .bind(by_id)
        .bind(efiling_id)
        .bind(receipt_no)
        .fetch_one(pool)
        .await
        .context("failed to submit e-filing")
}

pub async fn get_efiling_file(
    pool: &PgPool,
    tenant: &TenantRef,
    efiling_id: i64,
) -> Result<EfilingFile> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT file_id, efiling_id, file_name, encoding, contents, created_at
        FROM {schema}.efiling_files
        WHERE efiling_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#
    );
    sqlx::query_as::<_, EfilingFile>(&sql)
        .bind(efiling_id)
        .fetch_one(pool)
        .await
        .context("efiling file not found")
}

async fn efiling_error_count(pool: &PgPool, tenant: &TenantRef, efiling_id: i64) -> Result<i64> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT COUNT(*)
        FROM {schema}.efiling_validation
        WHERE efiling_id = $1 AND severity = 'ERROR'
        "#
    );
    sqlx::query_scalar::<_, i64>(&sql)
        .bind(efiling_id)
        .fetch_one(pool)
        .await
        .context("failed to count e-filing validation errors")
}

async fn insert_efiling_validations(
    pool: &PgPool,
    tenant: &TenantRef,
    efiling_id: i64,
    issues: &[EfilingValidationIssue],
) -> Result<()> {
    if issues.is_empty() {
        return Ok(());
    }
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.efiling_validation (
            efiling_id, validation_code, severity, message, field_path
        )
        VALUES ($1, $2, $3, $4, $5)
        "#
    );
    for issue in issues {
        sqlx::query(&sql)
            .bind(efiling_id)
            .bind(&issue.validation_code)
            .bind(&issue.severity)
            .bind(&issue.message)
            .bind(&issue.field_path)
            .execute(pool)
            .await
            .context("failed to insert e-filing validation issue")?;
    }
    Ok(())
}

async fn load_efile_master(
    pool: &PgPool,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
) -> Result<EfileMaster> {
    sqlx::query_as::<_, EfileMaster>(
        r#"
        SELECT efile_master_id, master_code, encoding
        FROM efile_masters
        WHERE status = 'APPROVED'
          AND effective_from <= $1
          AND (effective_to IS NULL OR effective_to >= $2)
        ORDER BY effective_from DESC, efile_master_id DESC
        LIMIT 1
        "#,
    )
    .bind(end_date)
    .bind(start_date)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("no applicable e-file master"))
}

async fn load_record_spec(pool: &PgPool, efile_master_id: i64) -> Result<Vec<RecordLayoutSpec>> {
    let fields = sqlx::query_as::<_, RecordFieldSpec>(
        r#"
        SELECT l.record_type, l.fixed_length,
               f.field_name, f.start_pos, f.byte_length, f.data_type,
               f.align, f.pad_char, f.required
        FROM efile_record_layouts l
        JOIN efile_record_fields f ON f.layout_id = l.layout_id
        WHERE l.efile_master_id = $1
        ORDER BY l.sort_order, f.start_pos
        "#,
    )
    .bind(efile_master_id)
    .fetch_all(pool)
    .await
    .context("failed to load e-file record fields")?;

    let mut layouts = Vec::new();
    for field in fields {
        if layouts
            .last()
            .map(|layout: &RecordLayoutSpec| layout.record_type.as_str())
            != Some(field.record_type.as_str())
        {
            layouts.push(RecordLayoutSpec {
                record_type: field.record_type.clone(),
                fixed_length: field.fixed_length,
                fields: Vec::new(),
            });
        }
        layouts
            .last_mut()
            .expect("layout exists after push")
            .fields
            .push(field);
    }
    if layouts.is_empty() {
        return Err(anyhow!("e-file format spec has no record fields"));
    }
    Ok(layouts)
}

async fn load_customer_year(pool: &PgPool, tenant: &TenantRef, by_id: i64) -> Result<CustomerYear> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT c.customer_name, c.biz_reg_no, b.year_label
        FROM {schema}.business_years b
        JOIN {schema}.customers c ON c.customer_id = b.customer_id
        WHERE b.by_id = $1
        "#
    );
    sqlx::query_as::<_, CustomerYear>(&sql)
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("customer/business year not found")
}

async fn load_std_fs_efiling_context(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<StdFsEfilingContext> {
    let validation = std_fs::validate_workspace_statements(pool, tenant, by_id).await?;
    let xml_records =
        load_confirmed_std_fs_xml_records(pool, tenant, by_id, validation.version_id).await?;
    let missing_xml_fields = xml_records
        .iter()
        .filter(|record| {
            record
                .xml_field_id
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
        })
        .map(|record| format!("{}.{}", record.stmt_type, record.item_code))
        .collect::<Vec<_>>();
    let total_mismatches = if xml_records.is_empty() {
        Vec::new()
    } else {
        let source_totals = load_financial_statement_totals(pool, tenant, by_id).await?;
        std_fs_total_mismatches(&xml_records, &source_totals)
    };

    Ok(StdFsEfilingContext {
        validation,
        xml_records,
        missing_xml_fields,
        total_mismatches,
    })
}

async fn load_confirmed_std_fs_xml_records(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    version_id: Uuid,
) -> Result<Vec<StdFsXmlRecord>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT s.stmt_type,
               s.item_code,
               i.xml_field_id,
               s.amount,
               i.account_class,
               i.normal_balance,
               i.is_subtotal
        FROM {schema}.std_fs_statements s
        JOIN public.std_fs_items i
          ON i.version_id = s.version_id
         AND i.stmt_type = s.stmt_type
         AND i.item_code = s.item_code
         AND i.is_active = TRUE
        WHERE s.business_year_id = $1
          AND s.version_id = $2
          AND s.status = 'CONFIRMED'
          AND s.stmt_type IN ('STD_BS', 'STD_IS')
        ORDER BY s.stmt_type, i.sort_order NULLS LAST, s.item_code
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(by_id)
        .bind(version_id)
        .fetch_all(pool)
        .await
        .context("failed to load confirmed std-fs XML records")?;
    Ok(rows
        .into_iter()
        .map(|row| StdFsXmlRecord {
            stmt_type: row.get("stmt_type"),
            item_code: row.get("item_code"),
            xml_field_id: row.get("xml_field_id"),
            amount: row.get("amount"),
            account_class: row.get("account_class"),
            normal_balance: row.get("normal_balance"),
            is_subtotal: row.get("is_subtotal"),
        })
        .collect())
}

async fn load_financial_statement_totals(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<FinancialStatementTotals> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT COALESCE(SUM(CASE
                   WHEN f.statement_type IN ('BS', 'STD_BS') AND l.debit_credit = 'DEBIT'
                   THEN l.amount ELSE 0 END), 0)::BIGINT AS bs_debit_total,
               COALESCE(SUM(CASE
                   WHEN f.statement_type IN ('IS', 'STD_IS') AND l.debit_credit = 'DEBIT'
                   THEN l.amount ELSE 0 END), 0)::BIGINT AS is_debit_total,
               COALESCE(SUM(CASE
                   WHEN f.statement_type IN ('IS', 'STD_IS') AND l.debit_credit = 'CREDIT'
                   THEN l.amount ELSE 0 END), 0)::BIGINT AS is_credit_total
        FROM {schema}.financial_statements f
        JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
        WHERE f.by_id = $1
          AND f.statement_type IN ('BS', 'IS', 'STD_BS', 'STD_IS')
        "#
    );
    let row = sqlx::query(&sql)
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("failed to load financial statement totals for e-filing")?;
    Ok(FinancialStatementTotals {
        bs_debit_total: row.get("bs_debit_total"),
        is_debit_total: row.get("is_debit_total"),
        is_credit_total: row.get("is_credit_total"),
    })
}

fn std_fs_total_mismatches(
    records: &[StdFsXmlRecord],
    source_totals: &FinancialStatementTotals,
) -> Vec<String> {
    let std_bs_assets = std_fs_amount(records, "1000");
    let std_bs_liabilities_equity = std_fs_amount(records, "2000") + std_fs_amount(records, "3000");
    let source_is_profit_loss = source_totals.is_credit_total - source_totals.is_debit_total;
    let std_is_profit_loss = std_fs_income_profit_loss(records);
    let mut mismatches = Vec::new();
    if std_bs_assets != std_bs_liabilities_equity {
        mismatches.push(format!(
            "STD_BS balance assets={} liabilities_plus_equity={}",
            std_bs_assets, std_bs_liabilities_equity
        ));
    }
    if std_bs_assets != source_totals.bs_debit_total {
        mismatches.push(format!(
            "STD_BS vs source BS assets={} source_bs_debit={}",
            std_bs_assets, source_totals.bs_debit_total
        ));
    }
    if std_is_profit_loss != source_is_profit_loss {
        mismatches.push(format!(
            "STD_IS vs source IS profit_loss={} source_profit_loss={}",
            std_is_profit_loss, source_is_profit_loss
        ));
    }
    mismatches
}

fn std_fs_amount(records: &[StdFsXmlRecord], item_code: &str) -> i64 {
    records
        .iter()
        .find(|record| record.item_code == item_code)
        .map(|record| record.amount)
        .unwrap_or(0)
}

fn std_fs_income_profit_loss(records: &[StdFsXmlRecord]) -> i64 {
    records
        .iter()
        .filter(|record| record.stmt_type == "STD_IS" && !record.is_subtotal)
        .map(
            |record| match record.account_class.as_deref().unwrap_or_default() {
                "REVENUE" | "GAIN" => record.amount,
                "EXPENSE" | "LOSS" => -record.amount,
                _ => match record.normal_balance.as_deref() {
                    Some("CREDIT") => record.amount,
                    Some("DEBIT") => -record.amount,
                    _ => 0,
                },
            },
        )
        .sum()
}

fn build_records(
    encoding: &str,
    spec: &[RecordLayoutSpec],
    customer_year: &CustomerYear,
    form3: &FormData,
    snapshot_id: i64,
    std_fs_records: &[StdFsXmlRecord],
) -> Result<Vec<u8>> {
    let data = &form3.data_json;
    let taxable_income = json_i64(data, "taxable_income");
    let corporate_tax = json_i64(data, "corporate_tax");
    let local_income_tax = json_i64(data, "local_income_tax");
    let tax_credits = json_i64(data, "tax_credits");
    let total_tax_due = json_i64(data, "total_tax_due");

    let mut values = HashMap::from([
        ("record_type".to_string(), "H".to_string()),
        (
            "biz_reg_no".to_string(),
            digits_only(&customer_year.biz_reg_no),
        ),
        (
            "customer_name".to_string(),
            customer_year.customer_name.clone(),
        ),
        (
            "year_label".to_string(),
            customer_year.year_label.to_string(),
        ),
        ("snapshot_id".to_string(), snapshot_id.to_string()),
        ("total_tax_due".to_string(), total_tax_due.to_string()),
    ]);
    let mut records = Vec::new();
    records.extend(build_record(layout_by_type(spec, "H")?, &values, encoding)?);
    records.extend(b"\r\n");

    values.insert("record_type".to_string(), "D".to_string());
    values.insert("form_code".to_string(), "FORM3".to_string());
    values.insert("taxable_income".to_string(), taxable_income.to_string());
    values.insert("corporate_tax".to_string(), corporate_tax.to_string());
    values.insert("local_income_tax".to_string(), local_income_tax.to_string());
    values.insert("tax_credits".to_string(), tax_credits.to_string());
    records.extend(build_record(layout_by_type(spec, "D")?, &values, encoding)?);
    records.extend(b"\r\n");

    for record in std_fs_records {
        if let Some(line) = build_std_fs_xml_record(record) {
            append_encoded_line(&mut records, &line, encoding)?;
        }
    }

    let record_count = 3_i64 + std_fs_xml_record_count(std_fs_records) as i64;
    let partial_checksum = checksum(&records);
    values.insert("record_type".to_string(), "T".to_string());
    values.insert("record_count".to_string(), record_count.to_string());
    values.insert("checksum".to_string(), partial_checksum);
    records.extend(build_record(layout_by_type(spec, "T")?, &values, encoding)?);
    records.extend(b"\r\n");

    Ok(records)
}

fn build_std_fs_xml_record(record: &StdFsXmlRecord) -> Option<String> {
    let xml_field_id = record.xml_field_id.as_deref()?.trim();
    if xml_field_id.is_empty() {
        return None;
    }
    Some(format!(
        "<stdFsRecord stmtType=\"{}\" itemCode=\"{}\" xmlFieldId=\"{}\" amount=\"{}\" />",
        xml_escape(&record.stmt_type),
        xml_escape(&record.item_code),
        xml_escape(xml_field_id),
        record.amount
    ))
}

fn std_fs_xml_record_count(records: &[StdFsXmlRecord]) -> usize {
    records
        .iter()
        .filter(|record| {
            !record
                .xml_field_id
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
        })
        .count()
}

fn append_encoded_line(contents: &mut Vec<u8>, line: &str, encoding: &str) -> Result<()> {
    let encoder = Encoding::for_label(encoding.as_bytes())
        .ok_or_else(|| anyhow!("unsupported encoding {encoding}"))?;
    let (encoded, _, had_errors) = encoder.encode(line);
    if had_errors {
        return Err(anyhow!("failed to encode XML record for {encoding}"));
    }
    contents.extend_from_slice(&encoded);
    contents.extend(b"\r\n");
    Ok(())
}

fn layout_by_type<'a>(
    spec: &'a [RecordLayoutSpec],
    record_type: &str,
) -> Result<&'a RecordLayoutSpec> {
    spec.iter()
        .find(|layout| layout.record_type == record_type)
        .ok_or_else(|| anyhow!("missing e-file record layout {record_type}"))
}

fn build_record(
    layout: &RecordLayoutSpec,
    values: &HashMap<String, String>,
    encoding: &str,
) -> Result<Vec<u8>> {
    let fixed_length = usize::try_from(layout.fixed_length)
        .map_err(|_| anyhow!("invalid fixed length for {}", layout.record_type))?;
    let mut record = vec![b' '; fixed_length];
    for field in &layout.fields {
        let start = usize::try_from(field.start_pos - 1)
            .map_err(|_| anyhow!("invalid start position for {}", field.field_name))?;
        let width = usize::try_from(field.byte_length)
            .map_err(|_| anyhow!("invalid byte length for {}", field.field_name))?;
        let end = start + width;
        if end > record.len() {
            return Err(anyhow!(
                "field {} exceeds {} record length",
                field.field_name,
                layout.record_type
            ));
        }
        let value = values
            .get(&field.field_name)
            .map(String::as_str)
            .unwrap_or_default();
        if field.required && value.is_empty() {
            return Err(anyhow!(
                "required e-file field {} is empty",
                field.field_name
            ));
        }
        let rendered = format_field(value, field, encoding)?;
        record[start..end].copy_from_slice(&rendered);
    }
    Ok(record)
}

fn format_field(value: &str, field: &RecordFieldSpec, encoding: &str) -> Result<Vec<u8>> {
    if field.data_type == "N" {
        return Ok(fixed_number(
            value.parse::<i64>().unwrap_or_default(),
            usize::try_from(field.byte_length)?,
        ));
    }
    fixed_text_aligned(
        value,
        usize::try_from(field.byte_length)?,
        encoding,
        &field.align,
        field.pad_char.as_bytes().first().copied().unwrap_or(b' '),
    )
}

fn json_i64(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0)
}

#[cfg(test)]
fn fixed_text(value: &str, width: usize, encoding: &str) -> Result<Vec<u8>> {
    fixed_text_aligned(value, width, encoding, "LEFT", b' ')
}

fn fixed_text_aligned(
    value: &str,
    width: usize,
    encoding: &str,
    align: &str,
    pad_char: u8,
) -> Result<Vec<u8>> {
    let encoder = Encoding::for_label(encoding.as_bytes())
        .ok_or_else(|| anyhow!("unsupported encoding {encoding}"))?;
    let mut output = Vec::with_capacity(width);
    for ch in value.chars() {
        let piece = ch.to_string();
        let (encoded, _, had_errors) = encoder.encode(&piece);
        if had_errors {
            return Err(anyhow!("failed to encode character for {encoding}"));
        }
        if output.len() + encoded.len() > width {
            break;
        }
        output.extend_from_slice(&encoded);
    }
    let pad_len = width.saturating_sub(output.len());
    if align == "RIGHT" {
        let mut padded = vec![pad_char; pad_len];
        padded.extend_from_slice(&output);
        Ok(padded)
    } else {
        output.resize(width, pad_char);
        Ok(output)
    }
}

fn fixed_number(value: i64, width: usize) -> Vec<u8> {
    let normalized = value.max(0).to_string();
    let mut padded = vec![b'0'; width.saturating_sub(normalized.len())];
    if normalized.len() <= width {
        padded.extend_from_slice(normalized.as_bytes());
        padded
    } else {
        normalized.as_bytes()[normalized.len() - width..].to_vec()
    }
}

fn digits_only(value: &str) -> String {
    value.chars().filter(char::is_ascii_digit).collect()
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn checksum(contents: &[u8]) -> String {
    let sum = contents
        .iter()
        .fold(0_u64, |acc, byte| acc.wrapping_add(u64::from(*byte)));
    format!("{sum:020}")
}

fn count_records(contents: &[u8]) -> i32 {
    contents
        .split(|byte| *byte == b'\n')
        .filter(|record| !record.is_empty())
        .count() as i32
}

async fn validate_efiling(
    pool: &PgPool,
    efile_master_id: i64,
    spec: &[RecordLayoutSpec],
    customer_year: &CustomerYear,
    form3: &FormData,
    contents: &[u8],
    std_fs_context: &StdFsEfilingContext,
) -> Result<Vec<EfilingValidationIssue>> {
    let rules = load_validation_rules(pool, efile_master_id).await?;
    let mut issues = Vec::new();
    for rule in rules {
        let rule_type = rule
            .rule_json
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let failed = match rule_type {
            "digits_length" => {
                let expected = rule
                    .rule_json
                    .get("length")
                    .and_then(Value::as_u64)
                    .unwrap_or(10) as usize;
                digits_only(&customer_year.biz_reg_no).len() != expected
            }
            "biz_reg_no_checksum" => !valid_biz_reg_no(&customer_year.biz_reg_no),
            "min" => {
                let minimum = rule
                    .rule_json
                    .get("min")
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                json_i64(&form3.data_json, "total_tax_due") < minimum
            }
            "record_length" => !record_lengths_match(spec, contents),
            "std_fs_confirmed" => !std_fs_context.validation.confirmed,
            "std_fs_xml_field" => !std_fs_context.missing_xml_fields.is_empty(),
            "std_fs_totals" => !std_fs_context.total_mismatches.is_empty(),
            _ => false,
        };
        if failed {
            issues.push(EfilingValidationIssue {
                validation_code: rule.rule_code,
                severity: rule.severity,
                message: rule.message,
                field_path: rule.field_path,
            });
        }
    }
    Ok(issues)
}

async fn load_validation_rules(
    pool: &PgPool,
    efile_master_id: i64,
) -> Result<Vec<EfileValidationRule>> {
    sqlx::query_as::<_, EfileValidationRule>(
        r#"
        SELECT rule_code, severity, field_path, message, rule_json
        FROM efile_validation_rules
        WHERE efile_master_id = $1 AND active = TRUE
        ORDER BY rule_id
        "#,
    )
    .bind(efile_master_id)
    .fetch_all(pool)
    .await
    .context("failed to load e-file validation rules")
}

fn record_lengths_match(spec: &[RecordLayoutSpec], contents: &[u8]) -> bool {
    let records: Vec<&[u8]> = contents
        .split(|byte| *byte == b'\n')
        .filter(|record| !record.is_empty())
        .map(|record| record.strip_suffix(b"\r").unwrap_or(record))
        .filter(|record| {
            matches!(
                record.first().copied(),
                Some(b'H') | Some(b'D') | Some(b'T')
            )
        })
        .collect();
    if records.len() != spec.len() {
        return false;
    }
    records
        .iter()
        .zip(spec)
        .all(|(record, layout)| record.len() == layout.fixed_length as usize)
}

fn valid_biz_reg_no(value: &str) -> bool {
    let digits = digits_only(value);
    if digits.len() != 10 {
        return false;
    }
    let numbers: Vec<i32> = digits
        .bytes()
        .map(|byte| i32::from(byte.saturating_sub(b'0')))
        .collect();
    let weights = [1, 3, 7, 1, 3, 7, 1, 3];
    let mut sum = weights
        .iter()
        .enumerate()
        .map(|(index, weight)| numbers[index] * weight)
        .sum::<i32>();
    let ninth = numbers[8] * 5;
    sum += ninth / 10 + ninth % 10;
    let check = (10 - (sum % 10)) % 10;
    check == numbers[9]
}

pub fn job_payload(tenant_code: &str, by_id: i64) -> Value {
    json!({
        "tenant_code": tenant_code,
        "by_id": by_id
    })
}

#[cfg(test)]
mod tests {
    use super::{fixed_number, fixed_text};

    #[test]
    fn fixed_width_text_respects_encoded_byte_length() {
        let bytes = fixed_text("가나다", 5, "windows-949").expect("encoding succeeds");
        assert_eq!(bytes.len(), 5);
    }

    #[test]
    fn fixed_width_number_zero_pads() {
        assert_eq!(fixed_number(42, 5), b"00042");
    }
}
