use std::{
    collections::{BTreeMap, HashMap},
    io::Cursor,
};

use anyhow::{anyhow, Context, Result};
use calamine::{Reader, Xlsx};
use chrono::NaiveDate;
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, QueryBuilder, Row};

use crate::{
    db::quote_ident,
    domain::{
        AccountMapping, AssetRecord, CreateAccountMappingRequest, FinancialStatementLine,
        ImportBatch, ImportError, TaxDataImportResponse, TaxDataValidationSummary, TenantRef,
        TransactionRecord,
    },
    tenant as tenant_service,
};

#[derive(Debug, Clone)]
struct TabularRow {
    row_no: i32,
    values: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct ImportIssue {
    row_no: i32,
    field_name: Option<String>,
    message: String,
    raw_row: Value,
}

#[derive(Debug, Clone)]
struct FinancialRow {
    row_no: i32,
    statement_type: String,
    account_code: String,
    account_name: String,
    standard_account_code: Option<String>,
    standard_account_name: Option<String>,
    debit: i64,
    credit: i64,
}

#[derive(Debug, Clone)]
struct AssetImportRow {
    asset_code: String,
    asset_name: String,
    asset_category: String,
    is_business_vehicle: bool,
    acquisition_date: NaiveDate,
    acquisition_cost: i64,
    useful_life_years: i32,
}

#[derive(Debug, Clone)]
struct TransactionImportRow {
    tx_date: NaiveDate,
    partner_name: String,
    category: String,
    account_code: Option<String>,
    description: Option<String>,
    amount: i64,
    evidence_type: Option<String>,
}

#[derive(Debug, Clone)]
struct MappingCandidate {
    statement_type: String,
    source_account_code: String,
    source_account_name: String,
    standard_account_code: String,
    standard_account_name: String,
}

#[derive(Debug, Clone)]
struct FinancialParseResult {
    row: FinancialRow,
    learned_mapping: Option<MappingCandidate>,
    auto_mapped: bool,
}

struct MappingWrite<'a> {
    customer_id: i64,
    statement_type: &'a str,
    source_account_code: &'a str,
    source_account_name: &'a str,
    standard_account_code: &'a str,
    standard_account_name: &'a str,
}

pub async fn import_tax_data(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    data_type: &str,
    file_name: Option<String>,
    bytes: &[u8],
) -> Result<TaxDataImportResponse> {
    tenant_service::ensure_business_year_editable(pool, tenant, by_id, "tax-data").await?;
    let data_type = normalize_data_type(data_type)?;
    let rows = parse_tabular(bytes, file_name.as_deref())?;
    let customer_id = business_year_customer_id(pool, tenant, by_id).await?;
    let batch = create_import_batch(
        pool,
        tenant,
        by_id,
        customer_id,
        &data_type,
        file_name.as_deref(),
        rows.len() as i32,
    )
    .await?;

    let outcome = match data_type.as_str() {
        "FINANCIAL_STATEMENT" => {
            import_financial_statements(pool, tenant, by_id, customer_id, batch.batch_id, &rows)
                .await?
        }
        "ASSET" => import_assets(pool, tenant, by_id, batch.batch_id, &rows).await?,
        "TRANSACTION" => import_transactions(pool, tenant, by_id, batch.batch_id, &rows).await?,
        _ => return Err(anyhow!("unsupported tax-data type")),
    };

    let errors = insert_import_errors(pool, tenant, batch.batch_id, &outcome.issues).await?;
    let updated = update_import_batch(
        pool,
        tenant,
        batch.batch_id,
        outcome.valid_count,
        errors.len() as i32,
        outcome.auto_mapped_count,
        outcome.metadata,
    )
    .await?;

    Ok(TaxDataImportResponse {
        batch: updated,
        errors,
    })
}

pub async fn list_import_batches(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<ImportBatch>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT batch_id, by_id, customer_id, data_type, source_file_name, row_count,
               valid_count, error_count, auto_mapped_count, status, metadata, created_at
        FROM {schema}.import_batches
        WHERE by_id = $1
        ORDER BY created_at DESC, batch_id DESC
        "#
    );
    sqlx::query_as::<_, ImportBatch>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list import batches")
}

pub async fn list_import_errors(
    pool: &PgPool,
    tenant: &TenantRef,
    batch_id: i64,
) -> Result<Vec<ImportError>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT error_id, batch_id, row_no, field_name, severity, message, raw_row, created_at
        FROM {schema}.import_errors
        WHERE batch_id = $1
        ORDER BY row_no, error_id
        "#
    );
    sqlx::query_as::<_, ImportError>(&sql)
        .bind(batch_id)
        .fetch_all(pool)
        .await
        .context("failed to list import errors")
}

pub async fn list_financial_statement_lines(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<FinancialStatementLine>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT l.line_id,
               l.fs_id,
               l.batch_id,
               f.statement_type,
               l.row_no,
               l.account_code,
               l.account_name,
               l.standard_account_code,
               l.standard_account_name,
               l.amount,
               l.debit_credit,
               CASE WHEN l.debit_credit = 'DEBIT' THEN l.amount ELSE 0 END AS debit,
               CASE WHEN l.debit_credit = 'CREDIT' THEN l.amount ELSE 0 END AS credit
        FROM {schema}.fs_lines l
        JOIN {schema}.financial_statements f ON f.fs_id = l.fs_id
        WHERE f.by_id = $1
        ORDER BY f.statement_type, l.line_id
        "#
    );
    sqlx::query_as::<_, FinancialStatementLine>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list financial statement lines")
}

pub async fn list_assets(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<AssetRecord>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT asset_id, by_id, batch_id, asset_code, asset_name, asset_category,
               is_business_vehicle, acquisition_date, acquisition_cost, useful_life_years, created_at
        FROM {schema}.assets
        WHERE by_id = $1
        ORDER BY asset_code
        "#
    );
    sqlx::query_as::<_, AssetRecord>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list assets")
}

pub async fn list_transactions(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<TransactionRecord>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT transaction_id, by_id, batch_id, tx_date, partner_name, category,
               account_code, description, amount, evidence_type, created_at
        FROM {schema}.transactions
        WHERE by_id = $1
        ORDER BY tx_date, transaction_id
        "#
    );
    sqlx::query_as::<_, TransactionRecord>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list transactions")
}

pub async fn list_account_mappings(
    pool: &PgPool,
    tenant: &TenantRef,
    customer_id: i64,
) -> Result<Vec<AccountMapping>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT mapping_id, customer_id, statement_type, source_account_code, source_account_name,
               standard_account_code, standard_account_name, use_count, last_used_at,
               created_at, updated_at
        FROM {schema}.account_mappings
        WHERE customer_id = $1
        ORDER BY statement_type, source_account_code
        "#
    );
    sqlx::query_as::<_, AccountMapping>(&sql)
        .bind(customer_id)
        .fetch_all(pool)
        .await
        .context("failed to list account mappings")
}

pub async fn create_account_mapping(
    pool: &PgPool,
    tenant: &TenantRef,
    customer_id: i64,
    request: CreateAccountMappingRequest,
) -> Result<AccountMapping> {
    let statement_type = request
        .statement_type
        .unwrap_or_else(|| "BS".to_string())
        .trim()
        .to_ascii_uppercase();
    upsert_account_mapping(
        pool,
        tenant,
        MappingWrite {
            customer_id,
            statement_type: &statement_type,
            source_account_code: request.source_account_code.trim(),
            source_account_name: request.source_account_name.trim(),
            standard_account_code: request.standard_account_code.trim(),
            standard_account_name: request.standard_account_name.trim(),
        },
    )
    .await
}

pub async fn validation_summary(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<TaxDataValidationSummary> {
    let schema = quote_ident(&tenant.schema_name)?;
    let totals = sqlx::query(&format!(
        r#"
        SELECT
            COALESCE(SUM(CASE WHEN l.debit_credit = 'DEBIT' THEN l.amount ELSE 0 END), 0)::BIGINT AS debit_total,
            COALESCE(SUM(CASE WHEN l.debit_credit = 'CREDIT' THEN l.amount ELSE 0 END), 0)::BIGINT AS credit_total,
            COUNT(l.line_id) AS fs_line_count,
            COALESCE(SUM(CASE WHEN l.standard_account_code IS NULL THEN 1 ELSE 0 END), 0)::BIGINT AS unresolved_mapping_count
        FROM {schema}.financial_statements f
        LEFT JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
        WHERE f.by_id = $1
        "#
    ))
    .bind(by_id)
    .fetch_one(pool)
    .await
    .context("failed to summarize financial statement validation")?;

    let asset_counts = sqlx::query(&format!(
        r#"
        SELECT COUNT(*) AS asset_count,
               COALESCE(SUM(CASE WHEN is_business_vehicle THEN 1 ELSE 0 END), 0)::BIGINT AS business_vehicle_count
        FROM {schema}.assets
        WHERE by_id = $1
        "#
    ))
    .bind(by_id)
    .fetch_one(pool)
    .await
    .context("failed to summarize asset validation")?;

    let transaction_count = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT COUNT(*) FROM {schema}.transactions WHERE by_id = $1"
    ))
    .bind(by_id)
    .fetch_one(pool)
    .await
    .context("failed to summarize transactions")?;

    let batch_error_count = sqlx::query_scalar::<_, i64>(&format!(
        r#"
        SELECT COUNT(*)
        FROM {schema}.import_errors e
        JOIN {schema}.import_batches b ON b.batch_id = e.batch_id
        WHERE b.by_id = $1
        "#
    ))
    .bind(by_id)
    .fetch_one(pool)
    .await
    .context("failed to summarize import errors")?;

    let debit_total = totals.get::<i64, _>("debit_total");
    let credit_total = totals.get::<i64, _>("credit_total");
    Ok(TaxDataValidationSummary {
        by_id,
        debit_total,
        credit_total,
        balanced: debit_total == credit_total && batch_error_count == 0,
        fs_line_count: totals.get::<i64, _>("fs_line_count"),
        unresolved_mapping_count: totals.get::<i64, _>("unresolved_mapping_count"),
        asset_count: asset_counts.get::<i64, _>("asset_count"),
        business_vehicle_count: asset_counts.get::<i64, _>("business_vehicle_count"),
        transaction_count,
        batch_error_count,
    })
}

pub fn template_csv(data_type: &str) -> Result<String> {
    match normalize_data_type(data_type)?.as_str() {
        "FINANCIAL_STATEMENT" => Ok(
            "statement_type,account_code,account_name,debit,credit,standard_account_code,standard_account_name\nBS,10100,현금,1000000,0,STD_CASH,현금\nBS,20100,미지급금,0,1000000,STD_PAYABLE,미지급금\n"
                .to_string(),
        ),
        "ASSET" => Ok(
            "asset_code,asset_name,asset_category,acquisition_date,acquisition_cost,useful_life_years\nCAR001,업무용 승용차,VEHICLE,2026-01-10,55000000,5\nMACH001,CNC 장비,MACHINERY,2026-02-01,120000000,8\n"
                .to_string(),
        ),
        "TRANSACTION" => Ok(
            "tx_date,partner_name,category,account_code,description,amount,evidence_type\n2026-03-01,좋은나눔재단,DONATION,53100,기부금 영수증,3000000,RECEIPT\n2026-04-05,거래처 만찬,ENTERTAINMENT,53200,저녁 회의,700000,CARD\n"
                .to_string(),
        ),
        _ => Err(anyhow!("unsupported tax-data type")),
    }
}

#[derive(Debug)]
struct ImportOutcome {
    valid_count: i32,
    auto_mapped_count: i32,
    issues: Vec<ImportIssue>,
    metadata: Value,
}

async fn import_financial_statements(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    customer_id: i64,
    batch_id: i64,
    rows: &[TabularRow],
) -> Result<ImportOutcome> {
    let existing_mappings = load_mapping_index(pool, tenant, customer_id).await?;
    let mut parsed = Vec::new();
    let mut issues = Vec::new();
    let mut debit_total = 0_i64;
    let mut credit_total = 0_i64;
    let mut auto_mapped_count = 0_i32;
    let mut learned_mappings = Vec::new();

    for row in rows {
        match parse_financial_row(row, &existing_mappings) {
            Ok(result) => {
                debit_total += result.row.debit;
                credit_total += result.row.credit;
                if result.auto_mapped {
                    auto_mapped_count += 1;
                }
                if let Some(mapping) = result.learned_mapping {
                    learned_mappings.push(mapping);
                }
                parsed.push(result.row);
            }
            Err(issue) => issues.push(issue),
        }
    }

    if debit_total != credit_total {
        issues.push(ImportIssue {
            row_no: 0,
            field_name: Some("debit_credit".to_string()),
            message: format!(
                "차변 합계 {debit_total}와 대변 합계 {credit_total}가 일치하지 않습니다."
            ),
            raw_row: json!({
                "debit_total": debit_total,
                "credit_total": credit_total
            }),
        });
    }

    if issues.is_empty() {
        for mapping in &learned_mappings {
            upsert_account_mapping(
                pool,
                tenant,
                MappingWrite {
                    customer_id,
                    statement_type: &mapping.statement_type,
                    source_account_code: &mapping.source_account_code,
                    source_account_name: &mapping.source_account_name,
                    standard_account_code: &mapping.standard_account_code,
                    standard_account_name: &mapping.standard_account_name,
                },
            )
            .await?;
        }
        for row in parsed
            .iter()
            .filter(|row| row.standard_account_code.is_some())
        {
            increment_mapping_use(
                pool,
                tenant,
                customer_id,
                &row.statement_type,
                &row.account_code,
            )
            .await?;
        }
        insert_financial_rows(pool, tenant, by_id, batch_id, &parsed).await?;
    }

    let mapping_rate = if rows.is_empty() {
        1.0
    } else {
        auto_mapped_count as f64 / rows.len() as f64
    };

    Ok(ImportOutcome {
        valid_count: if issues.is_empty() {
            parsed.len() as i32
        } else {
            0
        },
        auto_mapped_count,
        issues,
        metadata: json!({
            "debit_total": debit_total,
            "credit_total": credit_total,
            "mapping_rate": mapping_rate
        }),
    })
}

async fn import_assets(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    batch_id: i64,
    rows: &[TabularRow],
) -> Result<ImportOutcome> {
    let mut parsed = Vec::new();
    let mut issues = Vec::new();
    for row in rows {
        match parse_asset_row(row) {
            Ok(asset) => parsed.push(asset),
            Err(issue) => issues.push(issue),
        }
    }
    if issues.is_empty() {
        insert_assets(pool, tenant, by_id, batch_id, &parsed).await?;
    }
    let business_vehicle_count = parsed
        .iter()
        .filter(|asset| asset.is_business_vehicle)
        .count() as i32;
    Ok(ImportOutcome {
        valid_count: if issues.is_empty() {
            parsed.len() as i32
        } else {
            0
        },
        auto_mapped_count: 0,
        issues,
        metadata: json!({
            "business_vehicle_count": business_vehicle_count
        }),
    })
}

async fn import_transactions(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    batch_id: i64,
    rows: &[TabularRow],
) -> Result<ImportOutcome> {
    let mut parsed = Vec::new();
    let mut issues = Vec::new();
    for row in rows {
        match parse_transaction_row(row) {
            Ok(transaction) => parsed.push(transaction),
            Err(issue) => issues.push(issue),
        }
    }
    if issues.is_empty() {
        insert_transactions(pool, tenant, by_id, batch_id, &parsed).await?;
    }
    Ok(ImportOutcome {
        valid_count: if issues.is_empty() {
            parsed.len() as i32
        } else {
            0
        },
        auto_mapped_count: 0,
        issues,
        metadata: json!({
            "categories": parsed.iter().map(|row| row.category.clone()).collect::<Vec<_>>()
        }),
    })
}

async fn business_year_customer_id(pool: &PgPool, tenant: &TenantRef, by_id: i64) -> Result<i64> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!("SELECT customer_id FROM {schema}.business_years WHERE by_id = $1");
    sqlx::query_scalar::<_, i64>(&sql)
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("business year not found")
}

async fn create_import_batch(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    customer_id: i64,
    data_type: &str,
    file_name: Option<&str>,
    row_count: i32,
) -> Result<ImportBatch> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.import_batches (
            by_id, customer_id, data_type, source_file_name, row_count, status
        )
        VALUES ($1, $2, $3, $4, $5, 'PENDING')
        RETURNING batch_id, by_id, customer_id, data_type, source_file_name, row_count,
                  valid_count, error_count, auto_mapped_count, status, metadata, created_at
        "#
    );
    sqlx::query_as::<_, ImportBatch>(&sql)
        .bind(by_id)
        .bind(customer_id)
        .bind(data_type)
        .bind(file_name)
        .bind(row_count)
        .fetch_one(pool)
        .await
        .context("failed to create import batch")
}

async fn update_import_batch(
    pool: &PgPool,
    tenant: &TenantRef,
    batch_id: i64,
    valid_count: i32,
    error_count: i32,
    auto_mapped_count: i32,
    metadata: Value,
) -> Result<ImportBatch> {
    let status = if error_count == 0 {
        "IMPORTED"
    } else {
        "VALIDATION_FAILED"
    };
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.import_batches
        SET valid_count = $2,
            error_count = $3,
            auto_mapped_count = $4,
            status = $5,
            metadata = $6
        WHERE batch_id = $1
        RETURNING batch_id, by_id, customer_id, data_type, source_file_name, row_count,
                  valid_count, error_count, auto_mapped_count, status, metadata, created_at
        "#
    );
    sqlx::query_as::<_, ImportBatch>(&sql)
        .bind(batch_id)
        .bind(valid_count)
        .bind(error_count)
        .bind(auto_mapped_count)
        .bind(status)
        .bind(metadata)
        .fetch_one(pool)
        .await
        .context("failed to update import batch")
}

async fn insert_import_errors(
    pool: &PgPool,
    tenant: &TenantRef,
    batch_id: i64,
    issues: &[ImportIssue],
) -> Result<Vec<ImportError>> {
    let mut errors = Vec::new();
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.import_errors (
            batch_id, row_no, field_name, message, raw_row
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING error_id, batch_id, row_no, field_name, severity, message, raw_row, created_at
        "#
    );
    for issue in issues {
        let error = sqlx::query_as::<_, ImportError>(&sql)
            .bind(batch_id)
            .bind(issue.row_no)
            .bind(&issue.field_name)
            .bind(&issue.message)
            .bind(&issue.raw_row)
            .fetch_one(pool)
            .await
            .context("failed to insert import error")?;
        errors.push(error);
    }
    Ok(errors)
}

async fn insert_financial_rows(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    batch_id: i64,
    rows: &[FinancialRow],
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let mut fs_ids = BTreeMap::new();
    for statement_type in rows.iter().map(|row| row.statement_type.clone()) {
        if fs_ids.contains_key(&statement_type) {
            continue;
        }
        let fs_sql = format!(
            r#"
            INSERT INTO {schema}.financial_statements (by_id, batch_id, statement_type)
            VALUES ($1, $2, $3)
            RETURNING fs_id
            "#
        );
        let fs_id = sqlx::query_scalar::<_, i64>(&fs_sql)
            .bind(by_id)
            .bind(batch_id)
            .bind(&statement_type)
            .fetch_one(pool)
            .await
            .context("failed to insert financial statement")?;
        fs_ids.insert(statement_type, fs_id);
    }

    let line_sql = format!(
        r#"
        INSERT INTO {schema}.fs_lines (
            fs_id, batch_id, row_no, account_code, account_name,
            standard_account_code, standard_account_name, amount, debit_credit
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#
    );
    for row in rows {
        let fs_id = fs_ids
            .get(&row.statement_type)
            .copied()
            .context("financial statement header missing")?;
        let (amount, debit_credit) = if row.debit > 0 {
            (row.debit, "DEBIT")
        } else {
            (row.credit, "CREDIT")
        };
        sqlx::query(&line_sql)
            .bind(fs_id)
            .bind(batch_id)
            .bind(row.row_no)
            .bind(&row.account_code)
            .bind(&row.account_name)
            .bind(&row.standard_account_code)
            .bind(&row.standard_account_name)
            .bind(amount)
            .bind(debit_credit)
            .execute(pool)
            .await
            .context("failed to insert financial statement line")?;
    }
    Ok(())
}

async fn insert_assets(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    batch_id: i64,
    rows: &[AssetImportRow],
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    for chunk in rows.chunks(5_000) {
        let mut query = QueryBuilder::<Postgres>::new(format!(
            r#"
            INSERT INTO {schema}.assets (
                by_id, batch_id, asset_code, asset_name, asset_category, is_business_vehicle,
                acquisition_date, acquisition_cost, useful_life_years
            )
            "#
        ));
        query.push_values(chunk, |mut row_builder, row| {
            row_builder
                .push_bind(by_id)
                .push_bind(batch_id)
                .push_bind(&row.asset_code)
                .push_bind(&row.asset_name)
                .push_bind(&row.asset_category)
                .push_bind(row.is_business_vehicle)
                .push_bind(row.acquisition_date)
                .push_bind(row.acquisition_cost)
                .push_bind(row.useful_life_years);
        });
        query
            .build()
            .execute(pool)
            .await
            .context("failed to insert assets")?;
    }
    Ok(())
}

async fn insert_transactions(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    batch_id: i64,
    rows: &[TransactionImportRow],
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.transactions (
            by_id, batch_id, tx_date, partner_name, category, account_code,
            description, amount, evidence_type
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#
    );
    for row in rows {
        sqlx::query(&sql)
            .bind(by_id)
            .bind(batch_id)
            .bind(row.tx_date)
            .bind(&row.partner_name)
            .bind(&row.category)
            .bind(&row.account_code)
            .bind(&row.description)
            .bind(row.amount)
            .bind(&row.evidence_type)
            .execute(pool)
            .await
            .context("failed to insert transaction")?;
    }
    Ok(())
}

async fn load_mapping_index(
    pool: &PgPool,
    tenant: &TenantRef,
    customer_id: i64,
) -> Result<HashMap<(String, String), (String, String)>> {
    let mappings = list_account_mappings(pool, tenant, customer_id).await?;
    Ok(mappings
        .into_iter()
        .map(|mapping| {
            (
                (mapping.statement_type, mapping.source_account_code),
                (mapping.standard_account_code, mapping.standard_account_name),
            )
        })
        .collect())
}

async fn upsert_account_mapping(
    pool: &PgPool,
    tenant: &TenantRef,
    mapping: MappingWrite<'_>,
) -> Result<AccountMapping> {
    if mapping.source_account_code.trim().is_empty()
        || mapping.standard_account_code.trim().is_empty()
    {
        return Err(anyhow!("invalid account mapping"));
    }
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.account_mappings (
            customer_id, statement_type, source_account_code, source_account_name,
            standard_account_code, standard_account_name
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (customer_id, statement_type, source_account_code)
        DO UPDATE SET
            source_account_name = EXCLUDED.source_account_name,
            standard_account_code = EXCLUDED.standard_account_code,
            standard_account_name = EXCLUDED.standard_account_name,
            use_count = account_mappings.use_count + 1,
            last_used_at = NOW(),
            updated_at = NOW()
        RETURNING mapping_id, customer_id, statement_type, source_account_code, source_account_name,
                  standard_account_code, standard_account_name, use_count, last_used_at,
                  created_at, updated_at
        "#
    );
    sqlx::query_as::<_, AccountMapping>(&sql)
        .bind(mapping.customer_id)
        .bind(mapping.statement_type)
        .bind(mapping.source_account_code.trim())
        .bind(mapping.source_account_name.trim())
        .bind(mapping.standard_account_code.trim())
        .bind(mapping.standard_account_name.trim())
        .fetch_one(pool)
        .await
        .context("failed to upsert account mapping")
}

async fn increment_mapping_use(
    pool: &PgPool,
    tenant: &TenantRef,
    customer_id: i64,
    statement_type: &str,
    source_account_code: &str,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.account_mappings
        SET use_count = use_count + 1,
            last_used_at = NOW(),
            updated_at = NOW()
        WHERE customer_id = $1 AND statement_type = $2 AND source_account_code = $3
        "#
    );
    sqlx::query(&sql)
        .bind(customer_id)
        .bind(statement_type)
        .bind(source_account_code)
        .execute(pool)
        .await
        .context("failed to update account mapping usage")?;
    Ok(())
}

fn parse_tabular(bytes: &[u8], file_name: Option<&str>) -> Result<Vec<TabularRow>> {
    if file_name
        .unwrap_or_default()
        .to_ascii_lowercase()
        .ends_with(".xlsx")
    {
        parse_xlsx(bytes)
    } else {
        parse_csv(bytes)
    }
}

fn parse_csv(bytes: &[u8]) -> Result<Vec<TabularRow>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(bytes);
    let headers = reader
        .headers()
        .context("failed to read import headers")?
        .iter()
        .map(normalize_header)
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record.context("failed to read import record")?;
        let mut values = HashMap::new();
        for (header_index, header) in headers.iter().enumerate() {
            values.insert(
                header.clone(),
                record
                    .get(header_index)
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            );
        }
        if values.values().any(|value| !value.is_empty()) {
            rows.push(TabularRow {
                row_no: index as i32 + 2,
                values,
            });
        }
    }
    Ok(rows)
}

fn parse_xlsx(bytes: &[u8]) -> Result<Vec<TabularRow>> {
    let cursor = Cursor::new(bytes.to_vec());
    let mut workbook = Xlsx::new(cursor).context("failed to open xlsx workbook")?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| anyhow!("xlsx workbook has no worksheet"))?
        .context("failed to read xlsx worksheet")?;
    let mut rows = range.rows();
    let headers = rows
        .next()
        .ok_or_else(|| anyhow!("xlsx worksheet has no header row"))?
        .iter()
        .map(|cell| normalize_header(&cell.to_string()))
        .collect::<Vec<_>>();
    let mut parsed = Vec::new();
    for (index, row) in rows.enumerate() {
        let mut values = HashMap::new();
        for (header_index, header) in headers.iter().enumerate() {
            values.insert(
                header.clone(),
                row.get(header_index)
                    .map(|cell| cell.to_string().trim().to_string())
                    .unwrap_or_default(),
            );
        }
        if values.values().any(|value| !value.is_empty()) {
            parsed.push(TabularRow {
                row_no: index as i32 + 2,
                values,
            });
        }
    }
    Ok(parsed)
}

fn parse_financial_row(
    row: &TabularRow,
    mappings: &HashMap<(String, String), (String, String)>,
) -> Result<FinancialParseResult, ImportIssue> {
    let statement_type = row
        .get_any(&["statement_type", "statement", "fs_type"])
        .unwrap_or_else(|| "BS".to_string())
        .to_ascii_uppercase();
    let account_code = row.required("account_code")?;
    let account_name = row.required("account_name")?;
    let mut debit = row.amount_or_zero(&["debit"])?;
    let mut credit = row.amount_or_zero(&["credit"])?;
    if debit == 0 && credit == 0 {
        if let Some(amount) = row.get_any(&["amount"]) {
            let amount = parse_amount(&amount).map_err(|message| row.issue("amount", message))?;
            let dc = row
                .get_any(&["debit_credit", "dc"])
                .unwrap_or_else(|| "DEBIT".to_string())
                .to_ascii_uppercase();
            if dc == "CREDIT" || dc == "CR" {
                credit = amount;
            } else {
                debit = amount;
            }
        }
    }
    if debit < 0 || credit < 0 {
        return Err(row.issue("amount", "amount must be zero or positive"));
    }
    if debit == 0 && credit == 0 {
        return Err(row.issue("amount", "debit or credit amount is required"));
    }
    if debit > 0 && credit > 0 {
        return Err(row.issue(
            "debit_credit",
            "only one of debit or credit can have an amount",
        ));
    }

    let standard_account_code = row.get_any(&["standard_account_code", "standard_code"]);
    let standard_account_name = row.get_any(&["standard_account_name", "standard_name"]);
    let key = (statement_type.clone(), account_code.clone());
    let (resolved_code, resolved_name, learned_mapping, auto_mapped) =
        if let Some(code) = standard_account_code.filter(|value| !value.trim().is_empty()) {
            let name = standard_account_name
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| account_name.clone());
            (
                Some(code.clone()),
                Some(name.clone()),
                Some(MappingCandidate {
                    statement_type: statement_type.clone(),
                    source_account_code: account_code.clone(),
                    source_account_name: account_name.clone(),
                    standard_account_code: code,
                    standard_account_name: name,
                }),
                false,
            )
        } else if let Some((code, name)) = mappings.get(&key) {
            (Some(code.clone()), Some(name.clone()), None, true)
        } else {
            (None, None, None, false)
        };

    Ok(FinancialParseResult {
        row: FinancialRow {
            row_no: row.row_no,
            statement_type,
            account_code,
            account_name,
            standard_account_code: resolved_code,
            standard_account_name: resolved_name,
            debit,
            credit,
        },
        learned_mapping,
        auto_mapped,
    })
}

fn parse_asset_row(row: &TabularRow) -> Result<AssetImportRow, ImportIssue> {
    let asset_code = row.required("asset_code")?;
    let asset_name = row.required("asset_name")?;
    let asset_category = row
        .get_any(&["asset_category", "category"])
        .unwrap_or_else(|| "GENERAL".to_string())
        .to_ascii_uppercase();
    let acquisition_date = row.date("acquisition_date")?;
    let acquisition_cost = row.amount("acquisition_cost")?;
    let useful_life_years = row
        .get_any(&["useful_life_years", "useful_life"])
        .unwrap_or_else(|| "5".to_string())
        .parse::<i32>()
        .map_err(|_| row.issue("useful_life_years", "useful life must be an integer"))?;
    if useful_life_years <= 0 {
        return Err(row.issue("useful_life_years", "useful life must be positive"));
    }
    Ok(AssetImportRow {
        is_business_vehicle: is_business_vehicle(&asset_category, &asset_name),
        asset_code,
        asset_name,
        asset_category,
        acquisition_date,
        acquisition_cost,
        useful_life_years,
    })
}

fn parse_transaction_row(row: &TabularRow) -> Result<TransactionImportRow, ImportIssue> {
    let tx_date = row.date("tx_date")?;
    let partner_name = row.required("partner_name")?;
    let category = row
        .required("category")?
        .trim()
        .to_ascii_uppercase()
        .replace(' ', "_");
    let allowed = ["DONATION", "ENTERTAINMENT", "INTEREST", "OTHER"];
    if !allowed.contains(&category.as_str()) {
        return Err(row.issue("category", "invalid transaction category"));
    }
    let amount = row.amount("amount")?;
    if amount <= 0 {
        return Err(row.issue("amount", "amount must be positive"));
    }
    Ok(TransactionImportRow {
        tx_date,
        partner_name,
        category,
        account_code: row.get_any(&["account_code"]),
        description: row.get_any(&["description"]),
        amount,
        evidence_type: row.get_any(&["evidence_type"]),
    })
}

fn is_business_vehicle(asset_category: &str, asset_name: &str) -> bool {
    let text = format!(
        "{} {}",
        asset_category.to_ascii_uppercase(),
        asset_name.to_ascii_uppercase()
    );
    text.contains("VEHICLE")
        || text.contains("CAR")
        || asset_name.contains("차량")
        || asset_name.contains("승용")
        || asset_name.contains("자동차")
}

fn normalize_data_type(data_type: &str) -> Result<String> {
    let normalized = data_type.trim().to_ascii_lowercase().replace('_', "-");
    match normalized.as_str() {
        "financial-statements" | "financial-statement" | "fs" => {
            Ok("FINANCIAL_STATEMENT".to_string())
        }
        "assets" | "asset" => Ok("ASSET".to_string()),
        "transactions" | "transaction" => Ok("TRANSACTION".to_string()),
        _ => Err(anyhow!("unsupported tax-data type")),
    }
}

fn normalize_header(header: &str) -> String {
    header
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '.'], "_")
}

fn parse_amount(value: &str) -> Result<i64, &'static str> {
    let cleaned = value
        .trim()
        .replace([',', '_'], "")
        .replace("KRW", "")
        .trim()
        .to_string();
    if cleaned.is_empty() {
        return Ok(0);
    }
    cleaned
        .parse::<i64>()
        .map_err(|_| "amount must be an integer")
}

impl TabularRow {
    fn get_any(&self, names: &[&str]) -> Option<String> {
        names.iter().find_map(|name| {
            self.values.get(*name).and_then(|value| {
                if value.trim().is_empty() {
                    None
                } else {
                    Some(value.trim().to_string())
                }
            })
        })
    }

    fn required(&self, name: &str) -> Result<String, ImportIssue> {
        self.get_any(&[name])
            .ok_or_else(|| self.issue(name, "required field is missing"))
    }

    fn amount(&self, name: &str) -> Result<i64, ImportIssue> {
        let value = self.required(name)?;
        parse_amount(&value).map_err(|message| self.issue(name, message))
    }

    fn amount_or_zero(&self, names: &[&str]) -> Result<i64, ImportIssue> {
        match self.get_any(names) {
            Some(value) => parse_amount(&value).map_err(|message| self.issue(names[0], message)),
            None => Ok(0),
        }
    }

    fn date(&self, name: &str) -> Result<NaiveDate, ImportIssue> {
        let value = self.required(name)?;
        NaiveDate::parse_from_str(&value, "%Y-%m-%d")
            .map_err(|_| self.issue(name, "date must use YYYY-MM-DD"))
    }

    fn issue(&self, field_name: &str, message: impl Into<String>) -> ImportIssue {
        ImportIssue {
            row_no: self.row_no,
            field_name: Some(field_name.to_string()),
            message: message.into(),
            raw_row: json!(self.values),
        }
    }
}
