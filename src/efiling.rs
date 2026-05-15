use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use encoding_rs::Encoding;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{
    db::quote_ident,
    domain::{
        EfilingFile, EfilingFormatField, EfilingHistory, EfilingPrecheckResult,
        EfilingValidationIssue, FormData, TenantRef,
    },
    tax, tenant,
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

pub async fn generate_efiling(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<EfilingResult> {
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = tax::ensure_law_snapshot(pool, tenant, by_id).await?;
    let master = load_efile_master(pool, by.start_date, by.end_date).await?;
    let spec = load_record_spec(pool, master.efile_master_id).await?;
    let customer_year = load_customer_year(pool, tenant, by_id).await?;

    let form3 = match tax::get_form(pool, tenant, by_id, "FORM3").await {
        Ok(form) => form,
        Err(_) => tax::generate_form(pool, tenant, by_id, "FORM3").await?,
    };

    let contents = build_records(
        &master.encoding,
        &spec,
        &customer_year,
        &form3,
        snapshot.snapshot_id,
    )?;
    let issues = validate_efiling(
        pool,
        master.efile_master_id,
        &spec,
        &customer_year,
        &form3,
        &contents,
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
                  checksum, created_at, submitted_at
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
    let master = load_efile_master(pool, by.start_date, by.end_date).await?;
    let spec = load_record_spec(pool, master.efile_master_id).await?;
    let customer_year = load_customer_year(pool, tenant, by_id).await?;
    let form3 = match tax::get_form(pool, tenant, by_id, "FORM3").await {
        Ok(form) => form,
        Err(_) => tax::generate_form(pool, tenant, by_id, "FORM3").await?,
    };
    let contents = build_records(
        &master.encoding,
        &spec,
        &customer_year,
        &form3,
        snapshot.snapshot_id,
    )?;
    let issues = validate_efiling(
        pool,
        master.efile_master_id,
        &spec,
        &customer_year,
        &form3,
        &contents,
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
               checksum, created_at, submitted_at
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

fn build_records(
    encoding: &str,
    spec: &[RecordLayoutSpec],
    customer_year: &CustomerYear,
    form3: &FormData,
    snapshot_id: i64,
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

    let record_count = 3_i64;
    let partial_checksum = checksum(&records);
    values.insert("record_type".to_string(), "T".to_string());
    values.insert("record_count".to_string(), record_count.to_string());
    values.insert("checksum".to_string(), partial_checksum);
    records.extend(build_record(layout_by_type(spec, "T")?, &values, encoding)?);
    records.extend(b"\r\n");

    Ok(records)
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
