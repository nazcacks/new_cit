use std::{
    collections::{BTreeMap, HashMap},
    io::Cursor,
};

use anyhow::{anyhow, Context, Result};
use calamine::{Reader, Xlsx};
use chrono::NaiveDate;
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, QueryBuilder, Row, Transaction};

use crate::{
    db::quote_ident,
    domain::{
        AccountMapping, AssetBsReconcileIssue, AssetBsReconcileResult, AssetCarryForwardRequest,
        AssetCarryForwardResult, AssetRecord, CreateAccountMappingRequest, CreateAssetRequest,
        FinancialStatementLine, ImportBatch, ImportError, TaxDataImportResponse,
        TaxDataValidationSummary, TenantRef, TransactionIsReconcileIssue,
        TransactionIsReconcileResult, TransactionRecord, UpdateAssetRequest,
    },
    std_fs, tenant as tenant_service,
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
    is_auto_mapped: bool,
    map_confidence: Option<f64>,
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
    depr_method: String,
    residual_value: i64,
    accumulated_depr_prior: i64,
    acct_depr_current: i64,
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
    is_auto_mapped: bool,
    map_confidence: f64,
}

#[derive(Debug, Clone, Default)]
struct StatementTotals {
    debit_total: i64,
    credit_total: i64,
}

impl StatementTotals {
    fn add(&mut self, debit: i64, credit: i64) {
        self.debit_total += debit;
        self.credit_total += credit;
    }

    fn balanced(&self) -> bool {
        self.debit_total == self.credit_total
    }
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

    let outcome_result = match data_type.as_str() {
        "FINANCIAL_STATEMENT" => {
            import_financial_statements(pool, tenant, by_id, customer_id, batch.batch_id, &rows)
                .await
        }
        "ASSET" => import_assets(pool, tenant, by_id, batch.batch_id, &rows).await,
        "TRANSACTION" => import_transactions(pool, tenant, by_id, batch.batch_id, &rows).await,
        _ => return Err(anyhow!("unsupported tax-data type")),
    };
    let outcome = match outcome_result {
        Ok(outcome) => outcome,
        Err(error) => {
            let _ = mark_import_batch_failed(pool, tenant, batch.batch_id, &error).await;
            return Err(error);
        }
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
        std_map_rate: outcome.std_map_rate,
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
               COALESCE(l.std_account_code, l.standard_account_code) AS std_account_code,
               COALESCE(l.std_account_name, l.standard_account_name, sa.name_ko) AS std_account_name,
               COALESCE(l.is_auto_mapped, FALSE) AS is_auto_mapped,
               l.map_confidence::DOUBLE PRECISION AS map_confidence,
               COALESCE(l.standard_account_code, l.std_account_code) AS standard_account_code,
               COALESCE(l.standard_account_name, l.std_account_name, sa.name_ko) AS standard_account_name,
               l.std_fs_item_code,
               l.amount,
               l.debit_credit,
               CASE WHEN l.debit_credit = 'DEBIT' THEN l.amount ELSE 0 END AS debit,
               CASE WHEN l.debit_credit = 'CREDIT' THEN l.amount ELSE 0 END AS credit
        FROM {schema}.fs_lines l
        JOIN {schema}.financial_statements f ON f.fs_id = l.fs_id
        LEFT JOIN public.standard_accounts sa
               ON sa.code = COALESCE(l.std_account_code, l.standard_account_code)
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
               is_business_vehicle, acquisition_date, acquisition_cost, useful_life_years,
               depr_method, residual_value, accumulated_depr_prior, acct_depr_current,
               tax_depr_rate_bps, tax_depr_limit, depr_excess, depr_shortfall,
               prev_year_asset_id, created_at
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

pub async fn create_asset(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: CreateAssetRequest,
) -> Result<AssetRecord> {
    tenant_service::ensure_business_year_editable(pool, tenant, by_id, "asset register").await?;
    let asset_code = normalize_required_text(&request.asset_code, "asset_code")?;
    let asset_name = normalize_required_text(&request.asset_name, "asset_name")?;
    let asset_category = normalize_asset_category(request.asset_category.as_deref());
    let useful_life_years = request.useful_life_years.unwrap_or(5);
    validate_positive_i32(useful_life_years, "useful_life_years")?;
    let acquisition_cost = request.acquisition_cost.max(0);
    let residual_value = request.residual_value.unwrap_or(0).max(0);
    let accumulated_depr_prior = request.accumulated_depr_prior.unwrap_or(0).max(0);
    let acct_depr_current = request.acct_depr_current.unwrap_or(0).max(0);
    let depr_method = normalize_depr_method(request.depr_method.as_deref())?;
    let is_business_vehicle = request
        .is_business_vehicle
        .unwrap_or_else(|| is_business_vehicle(&asset_category, &asset_name));

    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.assets (
            by_id, batch_id, asset_code, asset_name, asset_category, is_business_vehicle,
            acquisition_date, acquisition_cost, useful_life_years, depr_method, residual_value,
            accumulated_depr_prior, acct_depr_current
        )
        VALUES ($1, NULL, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING asset_id, by_id, batch_id, asset_code, asset_name, asset_category,
                  is_business_vehicle, acquisition_date, acquisition_cost, useful_life_years,
                  depr_method, residual_value, accumulated_depr_prior, acct_depr_current,
                  tax_depr_rate_bps, tax_depr_limit, depr_excess, depr_shortfall,
                  prev_year_asset_id, created_at
        "#
    );
    sqlx::query_as::<_, AssetRecord>(&sql)
        .bind(by_id)
        .bind(asset_code)
        .bind(asset_name)
        .bind(asset_category)
        .bind(is_business_vehicle)
        .bind(request.acquisition_date)
        .bind(acquisition_cost)
        .bind(useful_life_years)
        .bind(depr_method)
        .bind(residual_value)
        .bind(accumulated_depr_prior)
        .bind(acct_depr_current)
        .fetch_one(pool)
        .await
        .context("failed to create asset")
}

pub async fn update_asset(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    asset_id: i64,
    request: UpdateAssetRequest,
) -> Result<AssetRecord> {
    tenant_service::ensure_business_year_editable(pool, tenant, by_id, "asset register").await?;
    let asset_code = request
        .asset_code
        .as_deref()
        .map(|value| normalize_required_text(value, "asset_code"))
        .transpose()?;
    let asset_name = request
        .asset_name
        .as_deref()
        .map(|value| normalize_required_text(value, "asset_name"))
        .transpose()?;
    if let Some(value) = request.useful_life_years {
        validate_positive_i32(value, "useful_life_years")?;
    }
    if let Some(value) = request.acquisition_cost {
        validate_nonnegative_i64(value, "acquisition_cost")?;
    }
    if let Some(value) = request.residual_value {
        validate_nonnegative_i64(value, "residual_value")?;
    }
    if let Some(value) = request.accumulated_depr_prior {
        validate_nonnegative_i64(value, "accumulated_depr_prior")?;
    }
    if let Some(value) = request.acct_depr_current {
        validate_nonnegative_i64(value, "acct_depr_current")?;
    }
    let asset_category = request
        .asset_category
        .as_deref()
        .map(|value| normalize_asset_category(Some(value)));
    let depr_method = request
        .depr_method
        .as_deref()
        .map(|value| normalize_depr_method(Some(value)))
        .transpose()?;

    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.assets
        SET asset_code = COALESCE($3, asset_code),
            asset_name = COALESCE($4, asset_name),
            asset_category = COALESCE($5, asset_category),
            is_business_vehicle = COALESCE($6, is_business_vehicle),
            acquisition_date = COALESCE($7, acquisition_date),
            acquisition_cost = COALESCE($8, acquisition_cost),
            useful_life_years = COALESCE($9, useful_life_years),
            depr_method = COALESCE($10, depr_method),
            residual_value = COALESCE($11, residual_value),
            accumulated_depr_prior = COALESCE($12, accumulated_depr_prior),
            acct_depr_current = COALESCE($13, acct_depr_current),
            updated_at = NOW()
        WHERE by_id = $1
          AND asset_id = $2
        RETURNING asset_id, by_id, batch_id, asset_code, asset_name, asset_category,
                  is_business_vehicle, acquisition_date, acquisition_cost, useful_life_years,
                  depr_method, residual_value, accumulated_depr_prior, acct_depr_current,
                  tax_depr_rate_bps, tax_depr_limit, depr_excess, depr_shortfall,
                  prev_year_asset_id, created_at
        "#
    );
    sqlx::query_as::<_, AssetRecord>(&sql)
        .bind(by_id)
        .bind(asset_id)
        .bind(asset_code)
        .bind(asset_name)
        .bind(asset_category)
        .bind(request.is_business_vehicle)
        .bind(request.acquisition_date)
        .bind(request.acquisition_cost)
        .bind(request.useful_life_years)
        .bind(depr_method)
        .bind(request.residual_value)
        .bind(request.accumulated_depr_prior)
        .bind(request.acct_depr_current)
        .fetch_one(pool)
        .await
        .context("asset not found")
}

pub async fn delete_asset(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    asset_id: i64,
) -> Result<()> {
    tenant_service::ensure_business_year_editable(pool, tenant, by_id, "asset register").await?;
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query(&format!(
        "DELETE FROM {schema}.vehicle_usage_logs WHERE by_id = $1 AND asset_id = $2"
    ))
    .bind(by_id)
    .bind(asset_id)
    .execute(pool)
    .await
    .context("failed to delete vehicle usage logs for asset")?;
    sqlx::query(&format!(
        "DELETE FROM {schema}.depreciation WHERE by_id = $1 AND asset_id = $2"
    ))
    .bind(by_id)
    .bind(asset_id)
    .execute(pool)
    .await
    .context("failed to delete depreciation rows for asset")?;
    let rows = sqlx::query(&format!(
        "DELETE FROM {schema}.assets WHERE by_id = $1 AND asset_id = $2"
    ))
    .bind(by_id)
    .bind(asset_id)
    .execute(pool)
    .await
    .context("failed to delete asset")?
    .rows_affected();
    if rows == 0 {
        anyhow::bail!("asset not found");
    }
    Ok(())
}

pub async fn carry_forward_assets(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: AssetCarryForwardRequest,
) -> Result<AssetCarryForwardResult> {
    tenant_service::ensure_business_year_editable(pool, tenant, by_id, "asset carry-forward")
        .await?;
    let target_by = tenant_service::get_business_year(pool, tenant, by_id).await?;
    let source_by_id = match request.source_by_id {
        Some(source_by_id) => source_by_id,
        None => previous_business_year_id(pool, tenant, &target_by).await?,
    };
    let source_by = tenant_service::get_business_year(pool, tenant, source_by_id).await?;
    if source_by.customer_id != target_by.customer_id {
        anyhow::bail!("asset carry-forward source must belong to the same customer");
    }
    if source_by.by_id == target_by.by_id {
        anyhow::bail!("asset carry-forward source must differ from target");
    }

    let schema = quote_ident(&tenant.schema_name)?;
    let source_count = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT COUNT(*) FROM {schema}.assets WHERE by_id = $1"
    ))
    .bind(source_by_id)
    .fetch_one(pool)
    .await
    .context("failed to count source assets")?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.assets (
            by_id, batch_id, asset_code, asset_name, asset_category, is_business_vehicle,
            acquisition_date, acquisition_cost, useful_life_years, depr_method, residual_value,
            accumulated_depr_prior, acct_depr_current, tax_depr_rate_bps, tax_depr_limit,
            depr_excess, depr_shortfall, prev_year_asset_id
        )
        SELECT $1, NULL, s.asset_code, s.asset_name, s.asset_category, s.is_business_vehicle,
               s.acquisition_date, s.acquisition_cost, s.useful_life_years, s.depr_method,
               s.residual_value, s.accumulated_depr_prior + s.acct_depr_current, 0,
               NULL, 0, 0, 0, s.asset_id
        FROM {schema}.assets s
        WHERE s.by_id = $2
          AND NOT EXISTS (
              SELECT 1
              FROM {schema}.assets t
              WHERE t.by_id = $1
                AND t.asset_code = s.asset_code
          )
        RETURNING asset_id, by_id, batch_id, asset_code, asset_name, asset_category,
                  is_business_vehicle, acquisition_date, acquisition_cost, useful_life_years,
                  depr_method, residual_value, accumulated_depr_prior, acct_depr_current,
                  tax_depr_rate_bps, tax_depr_limit, depr_excess, depr_shortfall,
                  prev_year_asset_id, created_at
        "#
    );
    let assets = sqlx::query_as::<_, AssetRecord>(&sql)
        .bind(by_id)
        .bind(source_by_id)
        .fetch_all(pool)
        .await
        .context("failed to carry forward assets")?;
    let copied_count = assets.len();
    let skipped_count = source_count.saturating_sub(copied_count as i64) as usize;
    Ok(AssetCarryForwardResult {
        source_by_id,
        copied_count,
        skipped_count,
        assets,
    })
}

pub async fn asset_bs_reconcile(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<AssetBsReconcileResult> {
    let assets = list_assets(pool, tenant, by_id).await?;
    let mut register_ppe_cost = 0_i64;
    let mut register_accum_depr = 0_i64;
    let mut register_intangible = 0_i64;
    for asset in &assets {
        let accumulated = asset.accumulated_depr_prior + asset.acct_depr_current;
        if is_intangible_category(&asset.asset_category) {
            register_intangible += (asset.acquisition_cost - accumulated).max(0);
        } else {
            register_ppe_cost += asset.acquisition_cost;
            register_accum_depr += accumulated;
        }
    }

    let bs = load_asset_bs_totals(pool, tenant, by_id).await?;
    let std_bs = load_asset_std_bs_totals(pool, tenant, by_id).await;
    let (std_ppe_cost, std_intangible) = std_bs.unwrap_or((0, 0));
    let totals = json!({
        "asset_register": {
            "ppe_cost": register_ppe_cost,
            "accumulated_depr": register_accum_depr,
            "intangible": register_intangible
        },
        "bs": {
            "ppe_cost": bs.ppe_cost,
            "accumulated_depr": bs.accumulated_depr,
            "intangible": bs.intangible
        },
        "std_bs": {
            "ppe_cost": std_ppe_cost,
            "intangible": std_intangible
        }
    });

    let issues = vec![
        asset_reconcile_issue(
            "CHK_PPE_COST",
            "ERROR",
            "Asset register PPE cost must match BS and standard BS PPE cost",
            bs.ppe_cost,
            register_ppe_cost,
            Some(std_ppe_cost),
            json!({
                "asset_register": register_ppe_cost,
                "bs": bs.ppe_cost,
                "std_bs": std_ppe_cost,
                "std_fs_item_codes": ["1521", "1522", "1523", "1524"]
            }),
        ),
        asset_reconcile_issue(
            "CHK_ACCUM_DEPR",
            "ERROR",
            "Asset register accumulated depreciation must match BS accumulated depreciation",
            bs.accumulated_depr,
            register_accum_depr,
            None,
            json!({
                "asset_register": register_accum_depr,
                "bs": bs.accumulated_depr,
                "standard_account_code": "ACCUM_DEPR"
            }),
        ),
        asset_reconcile_issue(
            "CHK_INTANGIBLE",
            "ERROR",
            "Asset register intangible assets must match BS and standard BS intangible assets",
            bs.intangible,
            register_intangible,
            Some(std_intangible),
            json!({
                "asset_register": register_intangible,
                "bs": bs.intangible,
                "std_bs": std_intangible,
                "std_fs_item_code": "1530"
            }),
        ),
    ];
    let error_count = issues
        .iter()
        .filter(|issue| !issue.passed && issue.severity == "ERROR")
        .count();
    let warn_count = issues
        .iter()
        .filter(|issue| !issue.passed && issue.severity == "WARN")
        .count();
    Ok(AssetBsReconcileResult {
        by_id,
        valid: error_count == 0,
        error_count,
        warn_count,
        totals,
        issues,
    })
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

pub async fn transaction_is_reconcile(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<TransactionIsReconcileResult> {
    let transaction_totals = load_transaction_reconcile_totals(pool, tenant, by_id).await?;
    let std_is_lines = std_fs::list_workspace_statements(pool, tenant, by_id, Some("STD_IS"))
        .await
        .context("failed to load standard income statement for transaction reconcile")?;

    let mut category_totals = serde_json::Map::new();
    let mut issues = Vec::new();
    for profile in transaction_reconcile_profiles() {
        let transaction_total = *transaction_totals.get(profile.category).unwrap_or(&0);
        let is_total =
            load_source_is_total(pool, tenant, by_id, profile.standard_account_codes).await?;
        let std_is_total = std_statement_total(&std_is_lines, profile.std_is_item_codes);
        let transaction_is_difference = transaction_total - is_total;
        let is_std_difference = is_total - std_is_total;
        let passed = transaction_is_difference.abs() <= profile.tolerance
            && is_std_difference.abs() <= profile.tolerance;

        category_totals.insert(
            profile.category.to_ascii_lowercase(),
            json!({
                "transaction_total": transaction_total,
                "is_total": is_total,
                "std_is_total": std_is_total,
                "transaction_is_difference": transaction_is_difference,
                "is_std_difference": is_std_difference,
                "module": profile.module_code,
            }),
        );
        issues.push(TransactionIsReconcileIssue {
            rule_code: profile.rule_code.to_string(),
            severity: profile.severity.to_string(),
            message: format!(
                "{} transactions must match source IS and STD_IS totals",
                profile.label
            ),
            passed,
            category: profile.category.to_string(),
            transaction_total,
            is_total,
            std_is_total,
            transaction_is_difference,
            is_std_difference,
            tolerance: profile.tolerance,
            metadata: json!({
                "category": profile.category,
                "standard_account_codes": profile.standard_account_codes,
                "std_is_item_codes": profile.std_is_item_codes,
                "module_code": profile.module_code,
            }),
        });
    }

    let error_count = issues
        .iter()
        .filter(|issue| !issue.passed && issue.severity == "ERROR")
        .count();
    let warn_count = issues
        .iter()
        .filter(|issue| !issue.passed && issue.severity == "WARN")
        .count();
    Ok(TransactionIsReconcileResult {
        by_id,
        valid: error_count == 0,
        error_count,
        warn_count,
        totals: json!({
            "categories": category_totals,
            "receivable_total": transaction_totals.get("RECEIVABLE").copied().unwrap_or(0),
            "other_total": transaction_totals.get("OTHER").copied().unwrap_or(0),
        }),
        issues,
    })
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
               COALESCE(std_account_code, standard_account_code) AS std_account_code,
               COALESCE(std_account_name, standard_account_name, sa.name_ko) AS std_account_name,
               COALESCE(is_auto_mapped, FALSE) AS is_auto_mapped,
               map_confidence::DOUBLE PRECISION AS map_confidence,
               COALESCE(standard_account_code, std_account_code) AS standard_account_code,
               COALESCE(standard_account_name, std_account_name, sa.name_ko) AS standard_account_name,
               use_count, last_used_at,
               created_at, updated_at
        FROM {schema}.account_mappings m
        LEFT JOIN public.standard_accounts sa
               ON sa.code = COALESCE(m.std_account_code, m.standard_account_code)
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
    let standard_account =
        resolve_standard_account(pool, request.standard_account_code.trim()).await?;
    let standard_account_name = request
        .standard_account_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&standard_account.name_ko);
    upsert_account_mapping(
        pool,
        tenant,
        MappingWrite {
            customer_id,
            statement_type: &statement_type,
            source_account_code: request.source_account_code.trim(),
            source_account_name: request.source_account_name.trim(),
            standard_account_code: &standard_account.code,
            standard_account_name,
            is_auto_mapped: false,
            map_confidence: 1.0,
        },
    )
    .await
}

pub async fn create_account_mapping_for_business_year(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: CreateAccountMappingRequest,
) -> Result<AccountMapping> {
    tenant_service::ensure_business_year_editable(pool, tenant, by_id, "account mapping").await?;
    let customer_id = business_year_customer_id(pool, tenant, by_id).await?;
    let mapping = create_account_mapping(pool, tenant, customer_id, request).await?;
    apply_account_mapping_to_business_year(pool, tenant, by_id, &mapping).await?;
    Ok(mapping)
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
            COALESCE(
                SUM(
                    CASE
                        WHEN COALESCE(l.std_account_code, l.standard_account_code) IS NULL THEN 1
                        ELSE 0
                    END
                ),
                0
            )::BIGINT AS unresolved_mapping_count
        FROM {schema}.financial_statements f
        LEFT JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
        WHERE f.by_id = $1
        "#
    ))
    .bind(by_id)
    .fetch_one(pool)
    .await
    .context("failed to summarize financial statement validation")?;
    let mandatory_missing_codes = mandatory_tax_mapping_missing_codes(pool, tenant, by_id).await?;

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
        mandatory_mapping_missing_count: mandatory_missing_codes.len() as i64,
        mandatory_mapping_missing_codes: mandatory_missing_codes,
        asset_count: asset_counts.get::<i64, _>("asset_count"),
        business_vehicle_count: asset_counts.get::<i64, _>("business_vehicle_count"),
        transaction_count,
        batch_error_count,
    })
}

#[derive(Debug, Clone)]
struct AssetBsTotals {
    ppe_cost: i64,
    accumulated_depr: i64,
    intangible: i64,
}

struct TransactionReconcileProfile {
    category: &'static str,
    label: &'static str,
    rule_code: &'static str,
    severity: &'static str,
    standard_account_codes: &'static [&'static str],
    std_is_item_codes: &'static [&'static str],
    module_code: &'static str,
    tolerance: i64,
}

fn transaction_reconcile_profiles() -> Vec<TransactionReconcileProfile> {
    vec![
        TransactionReconcileProfile {
            category: "DONATION",
            label: "Donation",
            rule_code: "CHK_DONATION_TXN",
            severity: "ERROR",
            standard_account_codes: &["DONATION_EXP", "STD_DONATION"],
            std_is_item_codes: &["5130"],
            module_code: "B2",
            tolerance: 0,
        },
        TransactionReconcileProfile {
            category: "ENTERTAINMENT",
            label: "Entertainment",
            rule_code: "CHK_ENTERTAIN_TXN",
            severity: "WARN",
            standard_account_codes: &["ENTERTAIN_EXP", "STD_ENTERTAINMENT"],
            std_is_item_codes: &["5140"],
            module_code: "B3",
            tolerance: 1,
        },
        TransactionReconcileProfile {
            category: "INTEREST",
            label: "Interest expense",
            rule_code: "CHK_INTEREST_TXN",
            severity: "ERROR",
            standard_account_codes: &["INTEREST_EXP", "STD_INTEREST_EXPENSE"],
            std_is_item_codes: &["5150"],
            module_code: "B9",
            tolerance: 0,
        },
    ]
}

async fn load_transaction_reconcile_totals(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<HashMap<String, i64>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let rows = sqlx::query(&format!(
        r#"
        SELECT category, COALESCE(SUM(amount), 0)::BIGINT AS amount
        FROM {schema}.transactions
        WHERE by_id = $1
        GROUP BY category
        "#
    ))
    .bind(by_id)
    .fetch_all(pool)
    .await
    .context("failed to summarize transaction reconcile totals")?;
    let mut totals = HashMap::new();
    for row in rows {
        let category = canonical_transaction_category(&row.get::<String, _>("category"));
        *totals.entry(category).or_insert(0) += row.get::<i64, _>("amount");
    }
    Ok(totals)
}

async fn load_source_is_total(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    standard_account_codes: &[&str],
) -> Result<i64> {
    let schema = quote_ident(&tenant.schema_name)?;
    let codes = standard_account_codes
        .iter()
        .map(|code| code.to_string())
        .collect::<Vec<_>>();
    sqlx::query_scalar::<_, i64>(&format!(
        r#"
        SELECT COALESCE(SUM(l.amount), 0)::BIGINT
        FROM {schema}.financial_statements f
        JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
        WHERE f.by_id = $1
          AND f.statement_type = 'IS'
          AND l.debit_credit = 'DEBIT'
          AND COALESCE(l.std_account_code, l.standard_account_code) = ANY($2)
        "#
    ))
    .bind(by_id)
    .bind(codes)
    .fetch_one(pool)
    .await
    .context("failed to load source IS total")
}

fn std_statement_total(lines: &[crate::domain::StdFsStatementLine], item_codes: &[&str]) -> i64 {
    lines
        .iter()
        .filter(|line| item_codes.contains(&line.item_code.as_str()))
        .map(|line| line.amount)
        .sum()
}

async fn previous_business_year_id(
    pool: &PgPool,
    tenant: &TenantRef,
    target_by: &crate::domain::BusinessYear,
) -> Result<i64> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query_scalar::<_, i64>(&format!(
        r#"
        SELECT by_id
        FROM {schema}.business_years
        WHERE customer_id = $1
          AND by_id <> $2
          AND (year_label < $3 OR end_date < $4)
        ORDER BY year_label DESC, end_date DESC, by_id DESC
        LIMIT 1
        "#
    ))
    .bind(target_by.customer_id)
    .bind(target_by.by_id)
    .bind(target_by.year_label)
    .bind(target_by.start_date)
    .fetch_optional(pool)
    .await
    .context("failed to resolve previous business year")?
    .ok_or_else(|| anyhow!("previous business year not found for asset carry-forward"))
}

async fn load_asset_bs_totals(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<AssetBsTotals> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT
            COALESCE(SUM(CASE
                WHEN f.statement_type = 'BS'
                 AND l.debit_credit = 'DEBIT'
                 AND (
                    COALESCE(l.std_account_code, l.standard_account_code) IN (
                        'PPE_GROSS', 'PPE_NET', 'STD_LAND', 'STD_BUILDING',
                        'STD_VEHICLE', 'STD_MACHINERY'
                    )
                    OR (
                        sa.tax_relevance = 'PPE'
                        AND sa.account_class = 'ASSET'
                        AND COALESCE(l.std_account_code, l.standard_account_code) <> 'ACCUM_DEPR'
                    )
                 )
                THEN l.amount ELSE 0 END), 0)::BIGINT AS ppe_cost,
            COALESCE(SUM(CASE
                WHEN f.statement_type = 'BS'
                 AND l.debit_credit = 'CREDIT'
                 AND (
                    COALESCE(l.std_account_code, l.standard_account_code) = 'ACCUM_DEPR'
                    OR (
                        sa.tax_relevance = 'PPE'
                        AND sa.account_class = 'CONTRA'
                    )
                    OR UPPER(l.account_name) LIKE '%DEPRECIATION%'
                    OR UPPER(l.account_name) LIKE '%DEPR%'
                    OR l.account_name LIKE '%媛먭??곴컖%'
                 )
                THEN l.amount ELSE 0 END), 0)::BIGINT AS accumulated_depr,
            COALESCE(SUM(CASE
                WHEN f.statement_type = 'BS'
                 AND l.debit_credit = 'DEBIT'
                 AND (
                    COALESCE(l.std_account_code, l.standard_account_code) IN ('STD_INTANGIBLE', 'INTANGIBLE')
                    OR sa.tax_relevance = 'INTANGIBLE'
                 )
                THEN l.amount ELSE 0 END), 0)::BIGINT AS intangible
        FROM {schema}.financial_statements f
        JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
        LEFT JOIN public.standard_accounts sa
               ON sa.code = COALESCE(l.std_account_code, l.standard_account_code)
        WHERE f.by_id = $1
          AND f.statement_type = 'BS'
        "#
    );
    let row = sqlx::query(&sql)
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("failed to load asset BS reconciliation totals")?;
    Ok(AssetBsTotals {
        ppe_cost: row.get("ppe_cost"),
        accumulated_depr: row.get("accumulated_depr"),
        intangible: row.get("intangible"),
    })
}

async fn load_asset_std_bs_totals(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<(i64, i64)> {
    let statements = std_fs::list_workspace_statements(pool, tenant, by_id, Some("STD_BS")).await?;
    let mut ppe_cost = 0_i64;
    let mut intangible = 0_i64;
    for line in statements {
        match line.item_code.as_str() {
            "1521" | "1522" | "1523" | "1524" => ppe_cost += line.amount,
            "1530" => intangible += line.amount,
            _ => {}
        }
    }
    Ok((ppe_cost, intangible))
}

fn asset_reconcile_issue(
    rule_code: &str,
    severity: &str,
    message: &str,
    expected: i64,
    actual: i64,
    comparable: Option<i64>,
    metadata: Value,
) -> AssetBsReconcileIssue {
    let primary_difference = actual - expected;
    let comparable_difference = comparable.map(|value| value - expected).unwrap_or(0);
    let difference = if primary_difference != 0 {
        primary_difference
    } else {
        comparable_difference
    };
    AssetBsReconcileIssue {
        rule_code: rule_code.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
        passed: primary_difference == 0 && comparable_difference == 0,
        expected,
        actual,
        difference,
        metadata,
    }
}

#[derive(Debug, Clone)]
struct StandardAccountLookup {
    code: String,
    name_ko: String,
}

async fn resolve_standard_account(pool: &PgPool, code: &str) -> Result<StandardAccountLookup> {
    let code = code.trim().to_ascii_uppercase();
    if code.is_empty() {
        return Err(anyhow!("standard account code is required"));
    }
    let row = sqlx::query(
        r#"
        SELECT code, name_ko
        FROM standard_accounts
        WHERE code = $1 AND is_active = TRUE
        "#,
    )
    .bind(&code)
    .fetch_optional(pool)
    .await
    .context("failed to resolve standard account")?
    .ok_or_else(|| anyhow!("invalid standard account code: {code}"))?;
    Ok(StandardAccountLookup {
        code: row.get("code"),
        name_ko: row.get("name_ko"),
    })
}

async fn load_standard_account_names(pool: &PgPool) -> Result<HashMap<String, String>> {
    let rows = sqlx::query(
        r#"
        SELECT code, name_ko
        FROM standard_accounts
        WHERE is_active = TRUE
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to load standard accounts")?;
    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.get::<String, _>("code").to_ascii_uppercase(),
                row.get::<String, _>("name_ko"),
            )
        })
        .collect())
}

async fn apply_account_mapping_to_business_year(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    mapping: &AccountMapping,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.fs_lines l
        SET std_account_code = $1,
            std_account_name = $2,
            standard_account_code = $1,
            standard_account_name = $2,
            is_auto_mapped = FALSE,
            map_confidence = 1.000
        FROM {schema}.financial_statements f
        WHERE f.fs_id = l.fs_id
          AND f.by_id = $3
          AND f.statement_type = $4
          AND l.account_code = $5
        "#
    );
    sqlx::query(&sql)
        .bind(&mapping.std_account_code)
        .bind(&mapping.std_account_name)
        .bind(by_id)
        .bind(&mapping.statement_type)
        .bind(&mapping.source_account_code)
        .execute(pool)
        .await
        .context("failed to apply account mapping to financial statement lines")?;
    Ok(())
}

async fn mandatory_tax_mapping_missing_codes(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<String>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        WITH requirements(required_code, tax_relevance, pattern) AS (
            VALUES
                ('NET_INCOME', 'NET_INCOME', '(NET.?INCOME|ACCOUNTING.?INCOME)'),
                ('REVENUE', 'REVENUE', '(REVENUE|SALES)'),
                ('PPE_NET', 'PPE', '(PPE|PROPERTY|PLANT|EQUIPMENT|MACHINERY|VEHICLE|BUILDING|LAND)'),
                ('ENTERTAIN_EXP', 'ENTERTAINMENT', '(ENTERTAIN)'),
                ('DONATION_EXP', 'DONATION', '(DONATION)'),
                ('INTEREST_EXP', 'INTEREST_EXP', '(INTEREST[ _-]*EXP)'),
                ('PENSION_PROV', 'PENSION', '(PENSION)'),
                ('BAD_DEBT_PROV', 'BAD_DEBT', '(BAD[ _-]*DEBT)')
        ),
        mapped_lines AS(
            SELECT UPPER(COALESCE(l.std_account_code, l.standard_account_code, '')) AS std_account_code,
                   UPPER(COALESCE(sa.tax_relevance, '')) AS tax_relevance,
                   UPPER(CONCAT_WS(' ', l.account_code, l.account_name)) AS searchable
            FROM {schema}.financial_statements f
            JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
            LEFT JOIN public.standard_accounts sa
                   ON sa.code = COALESCE(l.std_account_code, l.standard_account_code)
            WHERE f.by_id = $1
        )
        SELECT DISTINCT r.required_code
        FROM requirements r
        JOIN mapped_lines l ON l.searchable ~* r.pattern
        WHERE NOT (
            l.std_account_code = r.required_code
            OR l.tax_relevance = r.tax_relevance
        )
        ORDER BY r.required_code
        "#
    );
    let rows = sqlx::query_scalar::<_, String>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to verify mandatory tax account mappings")?;
    Ok(rows)
}

pub fn template_csv(data_type: &str) -> Result<String> {
    match normalize_data_type(data_type)?.as_str() {
        "FINANCIAL_STATEMENT" => Ok(
            "statement_type,account_code,account_name,debit,credit,standard_account_code,standard_account_name\nBS,10100,?꾧툑,1000000,0,STD_CASH,?꾧툑\nBS,20100,誘몄?湲됯툑,0,1000000,STD_PAYABLE,誘몄?湲됯툑\n"
                .to_string(),
        ),
        "ASSET" => Ok(
            "asset_code,asset_name,asset_category,acquisition_date,acquisition_cost,useful_life_years,depr_method,residual_value,accumulated_depr_prior,acct_depr_current\nCAR001,Business vehicle,VEHICLE,2026-01-10,55000000,5,SL,0,0,0\nMACH001,CNC machine,MACHINERY,2026-02-01,120000000,8,DB,0,0,0\n"
                .to_string(),
        ),
        "TRANSACTION" => Ok(
            "tx_date,partner_name,category,account_code,description,amount,evidence_type\n2026-03-01,Special Charity,DONATION,53100,Special donation receipt,3000000,RECEIPT\n2026-04-05,Client Dinner,ENTERTAINMENT,53200,Business meeting,700000,CARD\n2026-05-01,Trade Customer,RECEIVABLE,10200,Receivable balance for B6,12000000,AR_LEDGER\n2026-06-01,Main Bank,INTEREST,53300,General loan interest,900000,WIRE\n"
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
    std_map_rate: Option<f64>,
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
    let standard_accounts = load_standard_account_names(pool).await?;
    let mut parsed = Vec::new();
    let mut issues = Vec::new();
    let mut debit_total = 0_i64;
    let mut credit_total = 0_i64;
    let mut statement_totals = BTreeMap::<String, StatementTotals>::new();
    let mut auto_mapped_count = 0_i32;
    let mut learned_mappings = Vec::new();

    for row in rows {
        match parse_financial_row(row, &existing_mappings, &standard_accounts) {
            Ok(result) => {
                debit_total += result.row.debit;
                credit_total += result.row.credit;
                statement_totals
                    .entry(result.row.statement_type.clone())
                    .or_default()
                    .add(result.row.debit, result.row.credit);
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
                "李⑤? ?⑷퀎 {debit_total}? ?蹂 ?⑷퀎 {credit_total}媛 ?쇱튂?섏? ?딆뒿?덈떎."
            ),
            raw_row: json!({
                "debit_total": debit_total,
                "credit_total": credit_total
            }),
        });
    }
    for (statement_type, totals) in &statement_totals {
        if matches!(statement_type.as_str(), "BS" | "IS") && !totals.balanced() {
            issues.push(ImportIssue {
                row_no: 0,
                field_name: Some(format!("{statement_type}.debit_credit")),
                message: format!(
                    "{statement_type} 李⑤? ?⑷퀎 {}? ?蹂 ?⑷퀎 {}媛 ?쇱튂?섏? ?딆뒿?덈떎.",
                    totals.debit_total, totals.credit_total
                ),
                raw_row: json!({
                    "statement_type": statement_type,
                    "debit_total": totals.debit_total,
                    "credit_total": totals.credit_total
                }),
            });
        }
    }

    if issues.is_empty() {
        let mut tx = pool
            .begin()
            .await
            .context("failed to begin import transaction")?;
        for mapping in &learned_mappings {
            upsert_account_mapping_tx(
                &mut tx,
                tenant,
                MappingWrite {
                    customer_id,
                    statement_type: &mapping.statement_type,
                    source_account_code: &mapping.source_account_code,
                    source_account_name: &mapping.source_account_name,
                    standard_account_code: &mapping.standard_account_code,
                    standard_account_name: &mapping.standard_account_name,
                    is_auto_mapped: false,
                    map_confidence: 1.0,
                },
            )
            .await?;
        }
        for row in parsed
            .iter()
            .filter(|row| row.standard_account_code.is_some())
        {
            increment_mapping_use_tx(
                &mut tx,
                tenant,
                customer_id,
                &row.statement_type,
                &row.account_code,
            )
            .await?;
        }
        let std_map_rate = std_map_rate(&parsed, rows.len());
        insert_financial_rows_tx(&mut tx, tenant, by_id, batch_id, &parsed, std_map_rate).await?;
        tx.commit()
            .await
            .context("failed to commit import transaction")?;
    }

    let std_map_rate = std_map_rate(&parsed, rows.len());
    let auto_mapping_rate = if rows.is_empty() {
        1.0
    } else {
        auto_mapped_count as f64 / rows.len() as f64
    };
    let statement_balances = statement_totals
        .iter()
        .map(|(statement_type, totals)| {
            json!({
                "statementType": statement_type,
                "debitTotal": totals.debit_total,
                "creditTotal": totals.credit_total,
                "balanced": totals.balanced()
            })
        })
        .collect::<Vec<_>>();

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
            "mapping_rate": auto_mapping_rate,
            "stdMapRate": std_map_rate,
            "std_map_rate": std_map_rate,
            "statement_balances": statement_balances
        }),
        std_map_rate: Some(std_map_rate),
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
        std_map_rate: None,
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
        std_map_rate: None,
    })
}

fn std_map_rate(rows: &[FinancialRow], source_row_count: usize) -> f64 {
    if source_row_count == 0 {
        return 1.0;
    }
    let mapped = rows
        .iter()
        .filter(|row| {
            row.standard_account_code
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
        })
        .count();
    mapped as f64 / source_row_count as f64
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

async fn mark_import_batch_failed(
    pool: &PgPool,
    tenant: &TenantRef,
    batch_id: i64,
    error: &anyhow::Error,
) -> Result<ImportBatch> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.import_batches
        SET valid_count = 0,
            error_count = 1,
            auto_mapped_count = 0,
            status = 'FAILED',
            metadata = jsonb_build_object('error', $2::TEXT)
        WHERE batch_id = $1
        RETURNING batch_id, by_id, customer_id, data_type, source_file_name, row_count,
                  valid_count, error_count, auto_mapped_count, status, metadata, created_at
        "#
    );
    sqlx::query_as::<_, ImportBatch>(&sql)
        .bind(batch_id)
        .bind(error.to_string())
        .fetch_one(pool)
        .await
        .context("failed to mark import batch failed")
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

async fn insert_financial_rows_tx(
    tx: &mut Transaction<'_, Postgres>,
    tenant: &TenantRef,
    by_id: i64,
    batch_id: i64,
    rows: &[FinancialRow],
    std_map_rate: f64,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let mut fs_ids = BTreeMap::new();
    for statement_type in rows.iter().map(|row| row.statement_type.clone()) {
        if fs_ids.contains_key(&statement_type) {
            continue;
        }
        let fs_sql = format!(
            r#"
            INSERT INTO {schema}.financial_statements (by_id, batch_id, statement_type, std_map_rate)
            VALUES ($1, $2, $3, $4)
            RETURNING fs_id
            "#
        );
        let fs_id = sqlx::query_scalar::<_, i64>(&fs_sql)
            .bind(by_id)
            .bind(batch_id)
            .bind(&statement_type)
            .bind(std_map_rate)
            .fetch_one(&mut **tx)
            .await
            .context("failed to insert financial statement")?;
        fs_ids.insert(statement_type, fs_id);
    }

    let line_sql = format!(
        r#"
        INSERT INTO {schema}.fs_lines (
            fs_id, batch_id, row_no, account_code, account_name,
            std_account_code, std_account_name, is_auto_mapped, map_confidence,
            standard_account_code, standard_account_name, amount, debit_credit
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $6, $7, $10, $11)
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
            .bind(row.is_auto_mapped)
            .bind(row.map_confidence)
            .bind(amount)
            .bind(debit_credit)
            .execute(&mut **tx)
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
                acquisition_date, acquisition_cost, useful_life_years, depr_method, residual_value,
                accumulated_depr_prior, acct_depr_current
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
                .push_bind(row.useful_life_years)
                .push_bind(&row.depr_method)
                .push_bind(row.residual_value)
                .push_bind(row.accumulated_depr_prior)
                .push_bind(row.acct_depr_current);
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
            std_account_code, std_account_name, is_auto_mapped, map_confidence,
            standard_account_code, standard_account_name
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $5, $6)
        ON CONFLICT (customer_id, statement_type, source_account_code)
        DO UPDATE SET
            source_account_name = EXCLUDED.source_account_name,
            std_account_code = EXCLUDED.std_account_code,
            std_account_name = EXCLUDED.std_account_name,
            is_auto_mapped = EXCLUDED.is_auto_mapped,
            map_confidence = EXCLUDED.map_confidence,
            standard_account_code = EXCLUDED.standard_account_code,
            standard_account_name = EXCLUDED.standard_account_name,
            use_count = account_mappings.use_count + 1,
            last_used_at = NOW(),
            updated_at = NOW()
        RETURNING mapping_id, customer_id, statement_type, source_account_code, source_account_name,
                  std_account_code, std_account_name, is_auto_mapped,
                  map_confidence::DOUBLE PRECISION AS map_confidence,
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
        .bind(mapping.is_auto_mapped)
        .bind(mapping.map_confidence)
        .fetch_one(pool)
        .await
        .context("failed to upsert account mapping")
}

async fn upsert_account_mapping_tx(
    tx: &mut Transaction<'_, Postgres>,
    tenant: &TenantRef,
    mapping: MappingWrite<'_>,
) -> Result<()> {
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
            std_account_code, std_account_name, is_auto_mapped, map_confidence,
            standard_account_code, standard_account_name
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $5, $6)
        ON CONFLICT (customer_id, statement_type, source_account_code)
        DO UPDATE SET
            source_account_name = EXCLUDED.source_account_name,
            std_account_code = EXCLUDED.std_account_code,
            std_account_name = EXCLUDED.std_account_name,
            is_auto_mapped = EXCLUDED.is_auto_mapped,
            map_confidence = EXCLUDED.map_confidence,
            standard_account_code = EXCLUDED.standard_account_code,
            standard_account_name = EXCLUDED.standard_account_name,
            use_count = {schema}.account_mappings.use_count + 1,
            last_used_at = NOW(),
            updated_at = NOW()
        "#
    );
    sqlx::query(&sql)
        .bind(mapping.customer_id)
        .bind(mapping.statement_type)
        .bind(mapping.source_account_code.trim())
        .bind(mapping.source_account_name.trim())
        .bind(mapping.standard_account_code.trim())
        .bind(mapping.standard_account_name.trim())
        .bind(mapping.is_auto_mapped)
        .bind(mapping.map_confidence)
        .execute(&mut **tx)
        .await
        .context("failed to upsert account mapping")?;
    Ok(())
}

async fn increment_mapping_use_tx(
    tx: &mut Transaction<'_, Postgres>,
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
        WHERE customer_id = $1
          AND statement_type = $2
          AND source_account_code = $3
        "#
    );
    sqlx::query(&sql)
        .bind(customer_id)
        .bind(statement_type)
        .bind(source_account_code)
        .execute(&mut **tx)
        .await
        .context("failed to increment mapping use count")?;
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
    standard_accounts: &HashMap<String, String>,
) -> Result<FinancialParseResult, ImportIssue> {
    let raw_statement_type = row
        .get_any(&["statement_type", "statement", "fs_type"])
        .unwrap_or_else(|| "BS".to_string())
        .to_ascii_uppercase();
    let statement_type = normalize_financial_statement_type(&raw_statement_type)
        .ok_or_else(|| row.issue("statement_type", "吏?먰븯吏 ?딅뒗 ?щТ?쒗몴 援щ텇?낅땲??"))?;
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

    let standard_account_code =
        row.get_any(&["std_account_code", "standard_account_code", "standard_code"]);
    let standard_account_name =
        row.get_any(&["std_account_name", "standard_account_name", "standard_name"]);
    let key = (statement_type.clone(), account_code.clone());
    let (resolved_code, resolved_name, learned_mapping, auto_mapped) =
        if let Some(code) = standard_account_code.filter(|value| !value.trim().is_empty()) {
            let code = code.trim().to_ascii_uppercase();
            let master_name = standard_accounts.get(&code).ok_or_else(|| {
                row.issue(
                    "standard_account_code",
                    format!("invalid standard account code: {code}"),
                )
            })?;
            let name = standard_account_name
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| master_name.clone());
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
    let has_mapping = resolved_code.is_some();

    Ok(FinancialParseResult {
        row: FinancialRow {
            row_no: row.row_no,
            statement_type,
            account_code,
            account_name,
            standard_account_code: resolved_code,
            standard_account_name: resolved_name,
            is_auto_mapped: auto_mapped,
            map_confidence: if has_mapping { Some(1.0) } else { None },
            debit,
            credit,
        },
        learned_mapping,
        auto_mapped,
    })
}

fn normalize_financial_statement_type(value: &str) -> Option<String> {
    let normalized = value.trim().to_ascii_uppercase().replace([' ', '-'], "_");
    match normalized.as_str() {
        "BS" | "BALANCE_SHEET" | "STD_BS" => Some("BS".to_string()),
        "IS" | "PL" | "P_L" | "P&L" | "PROFIT_LOSS" | "INCOME_STATEMENT" | "STD_IS" => {
            Some("IS".to_string())
        }
        "CF" | "CASH_FLOW" | "STD_CF" => Some("CF".to_string()),
        "EQUITY" | "CE" | "CAPITAL_CHANGES" => Some("EQUITY".to_string()),
        _ => None,
    }
}

fn parse_asset_row(row: &TabularRow) -> Result<AssetImportRow, ImportIssue> {
    let asset_code = row.required("asset_code")?;
    let asset_name = row.required("asset_name")?;
    let asset_category = row
        .get_any(&["asset_category", "category"])
        .map(|value| normalize_asset_category(Some(&value)))
        .unwrap_or_else(|| normalize_asset_category(None));
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
    let depr_method = normalize_depr_method(
        row.get_any(&["depr_method", "depreciation_method"])
            .as_deref(),
    )
    .map_err(|error| row.issue("depr_method", error.to_string()))?;
    Ok(AssetImportRow {
        is_business_vehicle: is_business_vehicle(&asset_category, &asset_name),
        asset_code,
        asset_name,
        asset_category,
        acquisition_date,
        acquisition_cost,
        useful_life_years,
        depr_method,
        residual_value: row
            .amount_or_zero(&["residual_value", "salvage_value"])
            .map(|value| value.max(0))?,
        accumulated_depr_prior: row
            .amount_or_zero(&["accumulated_depr_prior", "prior_accumulated_depr"])
            .map(|value| value.max(0))?,
        acct_depr_current: row
            .amount_or_zero(&[
                "acct_depr_current",
                "book_depr_current",
                "depreciation_book",
            ])
            .map(|value| value.max(0))?,
    })
}

fn parse_transaction_row(row: &TabularRow) -> Result<TransactionImportRow, ImportIssue> {
    let tx_date = row.date("tx_date")?;
    let partner_name = row.required("partner_name")?;
    let category = canonical_transaction_category(&row.required("category")?);
    let allowed = [
        "DONATION",
        "ENTERTAINMENT",
        "INTEREST",
        "RECEIVABLE",
        "OTHER",
    ];
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

fn canonical_transaction_category(value: &str) -> String {
    let normalized = value.trim().to_ascii_uppercase().replace([' ', '-'], "_");
    match normalized.as_str() {
        "DONATION" | "DONATIONS" | "GIFT" | "CONTRIBUTION" => "DONATION".to_string(),
        "ENTERTAINMENT" | "ENTERTAIN" | "CORP_CARD" | "NO_RECEIPT" | "CONDOLENCE" => {
            "ENTERTAINMENT".to_string()
        }
        "INTEREST" | "INTEREST_EXP" | "INTEREST_EXPENSE" | "LOAN_INTEREST" => {
            "INTEREST".to_string()
        }
        "RECEIVABLE" | "RECEIVABLES" | "AR" | "BAD_DEBT" | "BAD_DEBT_RESERVE"
        | "BAD_DEBT_ALLOWANCE" => "RECEIVABLE".to_string(),
        _ => normalized,
    }
}

fn is_business_vehicle(asset_category: &str, asset_name: &str) -> bool {
    let text = format!(
        "{} {}",
        asset_category.to_ascii_uppercase(),
        asset_name.to_ascii_uppercase()
    );
    text.contains("VEHICLE") || text.contains("CAR") || text.contains("AUTO")
}

fn is_intangible_category(asset_category: &str) -> bool {
    let normalized = asset_category.trim().to_ascii_uppercase();
    ["INTANGIBLE", "SOFTWARE", "LICENSE", "PATENT", "GOODWILL"]
        .iter()
        .any(|keyword| normalized.contains(keyword))
}

fn normalize_asset_category(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("GENERAL")
        .to_ascii_uppercase()
        .replace([' ', '-'], "_")
}

fn normalize_depr_method(value: Option<&str>) -> Result<String> {
    let normalized = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("SL")
        .to_ascii_uppercase()
        .replace([' ', '-'], "_");
    match normalized.as_str() {
        "SL" | "STRAIGHT_LINE" | "STRAIGHTLINE" | "정액" | "정액법" => Ok("SL".to_string()),
        "DB" | "DECLINING_BALANCE" | "DECLININGBALANCE" | "정률" | "정률법" => {
            Ok("DB".to_string())
        }
        _ => Err(anyhow!("depr_method must be SL or DB")),
    }
}

fn normalize_required_text(value: &str, field_name: &str) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        anyhow::bail!("{field_name} is required");
    }
    Ok(normalized.to_string())
}

fn validate_positive_i32(value: i32, field_name: &str) -> Result<()> {
    if value <= 0 {
        anyhow::bail!("{field_name} must be positive");
    }
    Ok(())
}

fn validate_nonnegative_i64(value: i64, field_name: &str) -> Result<()> {
    if value < 0 {
        anyhow::bail!("{field_name} must be non-negative");
    }
    Ok(())
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
