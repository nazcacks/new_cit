use anyhow::{anyhow, Context, Result};
use encoding_rs::Encoding;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{
    db::quote_ident,
    domain::{EfilingFile, EfilingHistory, FormData, TenantRef},
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

pub async fn generate_efiling(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<EfilingResult> {
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = tax::ensure_law_snapshot(pool, tenant, by_id).await?;
    let master = load_efile_master(pool, by.start_date, by.end_date).await?;
    let customer_year = load_customer_year(pool, tenant, by_id).await?;

    let form3 = match tax::get_form(pool, tenant, by_id, "FORM3").await {
        Ok(form) => form,
        Err(_) => tax::generate_form(pool, tenant, by_id, "FORM3").await?,
    };

    let contents = build_records(
        &master.encoding,
        &customer_year,
        &form3,
        snapshot.snapshot_id,
    )?;
    let total_records = contents
        .split(|byte| *byte == b'\n')
        .filter(|record| !record.is_empty())
        .count() as i32;
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

    let mut records = Vec::new();
    records.extend(fixed_text("H", 1, encoding)?);
    records.extend(fixed_text(
        &digits_only(&customer_year.biz_reg_no),
        13,
        encoding,
    )?);
    records.extend(fixed_text(&customer_year.customer_name, 30, encoding)?);
    records.extend(fixed_number(i64::from(customer_year.year_label), 4));
    records.extend(fixed_number(snapshot_id, 12));
    records.extend(fixed_number(total_tax_due, 20));
    pad_record(&mut records, 80);
    records.extend(b"\r\n");

    records.extend(fixed_text("D", 1, encoding)?);
    records.extend(fixed_text("FORM3", 10, encoding)?);
    records.extend(fixed_number(taxable_income, 15));
    records.extend(fixed_number(corporate_tax, 15));
    records.extend(fixed_number(local_income_tax, 15));
    records.extend(fixed_number(tax_credits, 15));
    pad_record(&mut records, 162);
    records.extend(b"\r\n");

    let record_count = 3_i64;
    let partial_checksum = checksum(&records);
    records.extend(fixed_text("T", 1, encoding)?);
    records.extend(fixed_number(record_count, 6));
    records.extend(fixed_text(&partial_checksum, 20, encoding)?);
    pad_record(&mut records, 244);
    records.extend(b"\r\n");

    Ok(records)
}

fn json_i64(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn fixed_text(value: &str, width: usize, encoding: &str) -> Result<Vec<u8>> {
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
    output.resize(width, b' ');
    Ok(output)
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

fn pad_record(records: &mut Vec<u8>, next_line_start: usize) {
    if records.len() < next_line_start {
        records.resize(next_line_start, b' ');
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
