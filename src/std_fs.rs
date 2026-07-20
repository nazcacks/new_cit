use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::Cursor,
};

use anyhow::{anyhow, Context, Result};
use calamine::{Reader, Xlsx};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    db::quote_ident,
    domain::{
        BulkStdFsMappingRequest, BusinessYear, CarryForwardStdFsMappingRequest,
        CloneStdFsVersionRequest, CreateStdFsItemRequest, CreateStdFsVersionRequest,
        StdFsAggregateResult, StdFsConfirmResult, StdFsImportIssue, StdFsImportReport,
        StdFsIntegrityIssue, StdFsIntegrityResult, StdFsItem, StdFsItemDiff, StdFsItemVersion,
        StdFsMappingCarryForwardResult, StdFsMappingRow, StdFsMappingSaveResult, StdFsStatement,
        StdFsStatementLine, StdFsValidationIssue, StdFsValidationResult, StdFsVersionDiff,
        TenantRef, UpdateStdFsItemRequest, UpdateStdFsMappingRequest, UpdateStdFsVersionRequest,
        UpdateStdFsVersionStatusRequest,
    },
    tax, tenant,
};

const ALLOWED_VERSION_STATUSES: &[&str] = &["DRAFT", "REVIEWED", "ACTIVE", "RETIRED"];
const ALLOWED_STMT_TYPES: &[&str] = &["STD_BS", "STD_IS", "STD_COST", "STD_RE"];
const ALLOWED_NORMAL_BALANCES: &[&str] = &["DEBIT", "CREDIT"];
const IMPORT_HEADERS: &[&str] = &[
    "stmt_type",
    "item_code",
    "item_name",
    "parent_code",
    "level",
    "account_class",
    "normal_balance",
    "is_subtotal",
    "is_required",
    "agg_formula",
    "xml_field_id",
    "sort_order",
    "is_active",
];

#[derive(Debug, Clone)]
struct ImportTabularRow {
    row_no: i32,
    values: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct ImportItemRow {
    row_no: i32,
    stmt_type: String,
    item_code: String,
    item_name: String,
    parent_code: Option<String>,
    level: i32,
    account_class: Option<String>,
    normal_balance: Option<String>,
    is_subtotal: bool,
    is_required: bool,
    agg_formula: Option<String>,
    xml_field_id: Option<String>,
    sort_order: Option<i32>,
    is_active: bool,
    raw_row: Value,
}

#[derive(Debug, Clone)]
struct WorkspaceMappingContext {
    business_year: BusinessYear,
    version_id: Uuid,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct SourceStdFsMapping {
    account_code: String,
    account_name: Option<String>,
    std_fs_item_code: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct DirectStdFsAggregateRow {
    item_code: String,
    amount: i64,
    source_line_ids: Value,
}

#[derive(Debug, Clone)]
struct StdFsAggregateState {
    item: StdFsItem,
    direct_amount: i64,
    amount: i64,
    direct_source_line_ids: Vec<i64>,
    source_line_ids: Vec<i64>,
}

#[derive(Debug, Clone, Default, sqlx::FromRow)]
struct FinancialStatementTotals {
    bs_debit_total: i64,
    bs_credit_total: i64,
    is_debit_total: i64,
    is_credit_total: i64,
}

pub async fn list_workspace_mappings(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<StdFsMappingRow>> {
    let context = resolve_mapping_context(pool, tenant, by_id).await?;
    list_workspace_mappings_with_context(pool, tenant, &context).await
}

pub async fn save_workspace_mapping(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    account_code: &str,
    request: UpdateStdFsMappingRequest,
    user_id: i64,
) -> Result<StdFsMappingSaveResult> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "std-fs mapping").await?;
    let context = resolve_mapping_context(pool, tenant, by_id).await?;
    let account_code = normalize_account_code(account_code)?;
    let item_code = normalize_optional_item_code(request.std_fs_item_code.as_deref())?;

    let mut tx = pool
        .begin()
        .await
        .context("failed to begin std-fs mapping transaction")?;
    let (updated_count, cleared_count) = save_mapping_in_tx(
        pool,
        &mut tx,
        tenant,
        &context,
        &account_code,
        request.account_name.as_deref(),
        item_code.as_deref(),
        user_id,
    )
    .await?;
    tx.commit()
        .await
        .context("failed to commit std-fs mapping transaction")?;

    let mappings = list_workspace_mappings_with_context(pool, tenant, &context).await?;
    Ok(StdFsMappingSaveResult {
        updated_count,
        cleared_count,
        mappings,
    })
}

pub async fn bulk_save_workspace_mappings(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: BulkStdFsMappingRequest,
    user_id: i64,
) -> Result<StdFsMappingSaveResult> {
    if request.mappings.is_empty() {
        anyhow::bail!("mappings are required");
    }
    tenant::ensure_business_year_editable(pool, tenant, by_id, "std-fs mapping").await?;
    let context = resolve_mapping_context(pool, tenant, by_id).await?;

    let mut tx = pool
        .begin()
        .await
        .context("failed to begin std-fs mapping transaction")?;
    let mut updated_count = 0;
    let mut cleared_count = 0;
    for input in &request.mappings {
        let account_code = normalize_account_code(&input.account_code)?;
        let item_code = normalize_optional_item_code(input.std_fs_item_code.as_deref())?;
        let (updated, cleared) = save_mapping_in_tx(
            pool,
            &mut tx,
            tenant,
            &context,
            &account_code,
            input.account_name.as_deref(),
            item_code.as_deref(),
            user_id,
        )
        .await?;
        updated_count += updated;
        cleared_count += cleared;
    }
    tx.commit()
        .await
        .context("failed to commit std-fs mapping transaction")?;

    let mappings = list_workspace_mappings_with_context(pool, tenant, &context).await?;
    Ok(StdFsMappingSaveResult {
        updated_count,
        cleared_count,
        mappings,
    })
}

pub async fn carry_forward_workspace_mappings(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: CarryForwardStdFsMappingRequest,
    user_id: i64,
) -> Result<StdFsMappingCarryForwardResult> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "std-fs mapping").await?;
    let target = resolve_mapping_context(pool, tenant, by_id).await?;
    let source_by_id = match request.source_by_id {
        Some(source_by_id) => source_by_id,
        None => previous_business_year_id(pool, tenant, &target.business_year)
            .await?
            .ok_or_else(|| anyhow!("source business year not found"))?,
    };
    if source_by_id == by_id {
        anyhow::bail!("invalid source_by_id: source and target business years must differ");
    }

    let source_by = tenant::get_business_year(pool, tenant, source_by_id).await?;
    if source_by.customer_id != target.business_year.customer_id {
        anyhow::bail!("invalid source_by_id: source business year belongs to a different customer");
    }
    let source = resolve_mapping_context(pool, tenant, source_by_id).await?;
    let source_mappings = load_source_std_fs_mappings(pool, tenant, &source).await?;
    let target_leaf_codes = load_leaf_item_codes(pool, target.version_id).await?;

    let mut tx = pool
        .begin()
        .await
        .context("failed to begin std-fs carry-forward transaction")?;
    let mut copied_count = 0;
    let mut skipped_count = 0;
    for mapping in source_mappings {
        if !target_leaf_codes.contains(&mapping.std_fs_item_code) {
            skipped_count += 1;
            continue;
        }
        upsert_std_fs_mapping_in_tx(
            &mut tx,
            tenant,
            &target,
            &mapping.account_code,
            mapping.account_name.as_deref(),
            &mapping.std_fs_item_code,
            true,
            user_id,
        )
        .await?;
        apply_std_fs_mapping_to_lines_in_tx(
            &mut tx,
            tenant,
            target.business_year.by_id,
            &mapping.account_code,
            &mapping.std_fs_item_code,
        )
        .await?;
        copied_count += 1;
    }
    tx.commit()
        .await
        .context("failed to commit std-fs carry-forward transaction")?;

    let mappings = list_workspace_mappings_with_context(pool, tenant, &target).await?;
    Ok(StdFsMappingCarryForwardResult {
        source_by_id,
        copied_count,
        skipped_count,
        mappings,
    })
}

pub async fn aggregate_workspace_statements(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<StdFsAggregateResult> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "std-fs aggregation").await?;
    let context = resolve_mapping_context(pool, tenant, by_id).await?;
    let statements = aggregate_statement_lines(pool, tenant, &context).await?;
    let validation = validate_statement_lines(pool, tenant, &context, &statements).await?;
    Ok(StdFsAggregateResult {
        by_id,
        version_id: context.version_id,
        statements,
        validation,
    })
}

pub async fn list_workspace_statements(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    stmt_type: Option<&str>,
) -> Result<Vec<StdFsStatementLine>> {
    let stmt_type = normalize_statement_filter(stmt_type)?;
    let context = resolve_mapping_context(pool, tenant, by_id).await?;
    let confirmed =
        load_confirmed_statement_lines(pool, tenant, &context, stmt_type.as_deref()).await?;
    if !confirmed.is_empty() {
        return Ok(confirmed);
    }
    let mut statements = aggregate_statement_lines(pool, tenant, &context).await?;
    if let Some(stmt_type) = stmt_type {
        statements.retain(|line| line.stmt_type == stmt_type);
    }
    Ok(statements)
}

pub async fn validate_workspace_statements(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<StdFsValidationResult> {
    let context = resolve_mapping_context(pool, tenant, by_id).await?;
    let statements = aggregate_statement_lines(pool, tenant, &context).await?;
    validate_statement_lines(pool, tenant, &context, &statements).await
}

pub async fn confirm_workspace_statements(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<StdFsConfirmResult> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "std-fs confirmation").await?;
    let context = resolve_mapping_context(pool, tenant, by_id).await?;
    let statements = aggregate_statement_lines(pool, tenant, &context).await?;
    let mut validation = validate_statement_lines(pool, tenant, &context, &statements).await?;
    if validation.error_count > 0 {
        anyhow::bail!(
            "standard financial statement validation blocked confirmation: {} error(s)",
            validation.error_count
        );
    }
    validation.confirmed = true;

    let schema = quote_ident(&tenant.schema_name)?;
    let mut tx = pool
        .begin()
        .await
        .context("failed to begin std-fs confirmation transaction")?;

    let delete_superseded_sql = format!(
        r#"
        DELETE FROM {schema}.std_fs_statements
        WHERE business_year_id = $1
          AND version_id = $2
          AND status = 'SUPERSEDED'
        "#
    );
    sqlx::query(&delete_superseded_sql)
        .bind(by_id)
        .bind(context.version_id)
        .execute(&mut *tx)
        .await
        .context("failed to clear superseded std-fs statements")?;

    let supersede_sql = format!(
        r#"
        UPDATE {schema}.std_fs_statements
        SET status = 'SUPERSEDED'
        WHERE business_year_id = $1
          AND version_id = $2
          AND status = 'CONFIRMED'
        "#
    );
    sqlx::query(&supersede_sql)
        .bind(by_id)
        .bind(context.version_id)
        .execute(&mut *tx)
        .await
        .context("failed to supersede prior std-fs statements")?;

    let total_check = json!({
        "validation": &validation,
        "totals": validation.totals.clone(),
    });
    let insert_sql = format!(
        r#"
        INSERT INTO {schema}.std_fs_statements (
            tenant_id, business_year_id, version_id, stmt_type, status, item_code,
            amount, source_line_ids, total_check, confirmed_at
        )
        VALUES ($1, $2, $3, $4, 'CONFIRMED', $5, $6, $7, $8, NOW())
        RETURNING id, tenant_id, business_year_id, version_id, stmt_type, status,
                  item_code, amount, source_line_ids, total_check, confirmed_at, created_at
        "#
    );
    let mut confirmed = Vec::with_capacity(statements.len());
    for line in &statements {
        let row = sqlx::query_as::<_, StdFsStatement>(&insert_sql)
            .bind(tenant.tenant_id)
            .bind(by_id)
            .bind(context.version_id)
            .bind(&line.stmt_type)
            .bind(&line.item_code)
            .bind(line.amount)
            .bind(line.source_line_ids.clone())
            .bind(total_check.clone())
            .fetch_one(&mut *tx)
            .await
            .context("failed to insert confirmed std-fs statement")?;
        confirmed.push(row);
    }

    tx.commit()
        .await
        .context("failed to commit std-fs confirmation")?;

    Ok(StdFsConfirmResult {
        by_id,
        version_id: context.version_id,
        confirmed_count: confirmed.len(),
        statements: confirmed,
        validation,
    })
}

async fn aggregate_statement_lines(
    pool: &PgPool,
    tenant: &TenantRef,
    context: &WorkspaceMappingContext,
) -> Result<Vec<StdFsStatementLine>> {
    let items = list_items(pool, context.version_id, None, false).await?;
    let mut states = BTreeMap::<String, StdFsAggregateState>::new();
    let mut children_by_parent = HashMap::<String, Vec<String>>::new();
    for item in items
        .into_iter()
        .filter(|item| matches!(item.stmt_type.as_str(), "STD_BS" | "STD_IS"))
    {
        if let Some(parent_code) = item.parent_code.as_deref() {
            children_by_parent
                .entry(parent_code.to_string())
                .or_default()
                .push(item.item_code.clone());
        }
        states.insert(
            item.item_code.clone(),
            StdFsAggregateState {
                item,
                direct_amount: 0,
                amount: 0,
                direct_source_line_ids: Vec::new(),
                source_line_ids: Vec::new(),
            },
        );
    }

    for row in load_direct_aggregate_rows(pool, tenant, context).await? {
        let Some(state) = states.get_mut(&row.item_code) else {
            continue;
        };
        state.direct_amount += row.amount;
        state.amount += row.amount;
        let ids = json_i64_array(&row.source_line_ids);
        extend_unique_sorted(&mut state.direct_source_line_ids, ids.clone());
        extend_unique_sorted(&mut state.source_line_ids, ids);
    }

    let mut ordered_codes = states
        .values()
        .map(|state| {
            (
                state.item.level,
                state.item.sort_order.unwrap_or(i32::MAX),
                state.item.item_code.clone(),
            )
        })
        .collect::<Vec<_>>();
    ordered_codes.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
    });

    for (_, _, item_code) in ordered_codes {
        let Some(state) = states.get(&item_code) else {
            continue;
        };
        if !state.item.is_subtotal {
            continue;
        }
        let formula = state
            .item
            .agg_formula
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let direct_amount = state.direct_amount;
        let direct_source_line_ids = state.direct_source_line_ids.clone();

        let (calculated_amount, calculated_source_line_ids) = if let Some(formula) = formula {
            evaluate_agg_formula(&formula, &states)?
        } else if let Some(children) = children_by_parent.get(&item_code) {
            let mut amount = 0;
            let mut source_line_ids = Vec::new();
            for child_code in children {
                if let Some(child) = states.get(child_code) {
                    amount += child.amount;
                    extend_unique_sorted(&mut source_line_ids, child.source_line_ids.clone());
                }
            }
            (amount, source_line_ids)
        } else {
            (0, Vec::new())
        };

        let Some(state) = states.get_mut(&item_code) else {
            continue;
        };
        state.amount = direct_amount + calculated_amount;
        state.source_line_ids = direct_source_line_ids;
        extend_unique_sorted(&mut state.source_line_ids, calculated_source_line_ids);
    }

    let mut lines = states
        .values()
        .map(|state| StdFsStatementLine {
            by_id: context.business_year.by_id,
            version_id: context.version_id,
            stmt_type: state.item.stmt_type.clone(),
            item_code: state.item.item_code.clone(),
            item_name: state.item.item_name.clone(),
            parent_code: state.item.parent_code.clone(),
            level: state.item.level,
            account_class: state.item.account_class.clone(),
            normal_balance: state.item.normal_balance.clone(),
            is_subtotal: state.item.is_subtotal,
            is_required: state.item.is_required,
            sort_order: state.item.sort_order,
            amount: state.amount,
            source_line_ids: json!(state.source_line_ids),
            total_check: json!({}),
            confirmed: false,
            confirmed_at: None,
        })
        .collect::<Vec<_>>();
    sort_statement_lines(&mut lines);
    Ok(lines)
}

async fn load_direct_aggregate_rows(
    pool: &PgPool,
    tenant: &TenantRef,
    context: &WorkspaceMappingContext,
) -> Result<Vec<DirectStdFsAggregateRow>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT l.std_fs_item_code AS item_code,
               COALESCE(SUM(l.amount), 0)::BIGINT AS amount,
               COALESCE(jsonb_agg(l.line_id ORDER BY l.line_id), '[]'::jsonb) AS source_line_ids
        FROM {schema}.financial_statements f
        JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
        JOIN public.std_fs_items i
          ON i.version_id = $2
         AND i.item_code = l.std_fs_item_code
         AND i.is_active = TRUE
        WHERE f.by_id = $1
          AND f.statement_type IN ('BS', 'IS', 'STD_BS', 'STD_IS')
          AND l.std_fs_item_code IS NOT NULL
          AND TRIM(l.std_fs_item_code) <> ''
        GROUP BY l.std_fs_item_code
        "#
    );
    sqlx::query_as::<_, DirectStdFsAggregateRow>(&sql)
        .bind(context.business_year.by_id)
        .bind(context.version_id)
        .fetch_all(pool)
        .await
        .context("failed to aggregate std-fs direct lines")
}

async fn validate_statement_lines(
    pool: &PgPool,
    tenant: &TenantRef,
    context: &WorkspaceMappingContext,
    lines: &[StdFsStatementLine],
) -> Result<StdFsValidationResult> {
    let totals = load_financial_statement_totals(pool, tenant, context.business_year.by_id).await?;
    let unmapped_count = load_unmapped_std_fs_line_count(pool, tenant, context).await?;
    let confirmed = confirmed_statement_count(
        pool,
        tenant,
        context.business_year.by_id,
        context.version_id,
    )
    .await?
        > 0;

    let std_bs_assets = line_amount_by_code(lines, "1000");
    let std_bs_liabilities = line_amount_by_code(lines, "2000");
    let std_bs_equity = line_amount_by_code(lines, "3000");
    let std_bs_liabilities_equity = std_bs_liabilities + std_bs_equity;
    let source_bs_assets = totals.bs_debit_total;
    let source_is_profit_loss = totals.is_credit_total - totals.is_debit_total;
    let std_is_profit_loss = standard_income_profit_loss(lines);

    let mut issues = vec![
        std_validation_issue(
            "CHK_STDBS_BALANCE",
            "ERROR",
            "STD_BS assets must equal liabilities plus equity",
            std_bs_liabilities_equity,
            std_bs_assets,
            json!({
                "asset_item_code": "1000",
                "liability_item_code": "2000",
                "equity_item_code": "3000",
            }),
        ),
        std_validation_issue(
            "CHK_STDBS_VS_FS",
            "ERROR",
            "STD_BS asset total must equal source BS asset total",
            source_bs_assets,
            std_bs_assets,
            json!({
                "source": "financial_statements",
                "statement_type": "BS",
                "debit_credit": "DEBIT",
            }),
        ),
        std_validation_issue(
            "CHK_STDIS_VS_FS",
            "ERROR",
            "STD_IS profit/loss must equal source IS profit/loss",
            source_is_profit_loss,
            std_is_profit_loss,
            json!({
                "source": "financial_statements",
                "statement_type": "IS",
                "formula": "credits - debits",
            }),
        ),
        std_validation_issue(
            "CHK_STDFS_UNMAPPED",
            "ERROR",
            "All source FS lines must be mapped to an active leaf standard FS item",
            0,
            unmapped_count,
            json!({
                "line_scope": ["BS", "IS"],
                "invalid_if": ["missing", "inactive", "subtotal", "statement_type_mismatch"],
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
    let totals_json = json!({
        "std_bs": {
            "assets": std_bs_assets,
            "liabilities": std_bs_liabilities,
            "equity": std_bs_equity,
            "liabilities_plus_equity": std_bs_liabilities_equity,
        },
        "source_fs": {
            "bs_debit_total": totals.bs_debit_total,
            "bs_credit_total": totals.bs_credit_total,
            "is_debit_total": totals.is_debit_total,
            "is_credit_total": totals.is_credit_total,
            "is_profit_loss": source_is_profit_loss,
        },
        "std_is": {
            "profit_loss": std_is_profit_loss,
        },
        "unmapped_count": unmapped_count,
    });

    for issue in &mut issues {
        issue.metadata["totals"] = totals_json.clone();
    }

    Ok(StdFsValidationResult {
        by_id: context.business_year.by_id,
        version_id: context.version_id,
        valid: error_count == 0,
        error_count,
        warn_count,
        unmapped_count,
        confirmed,
        totals: totals_json,
        issues,
    })
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
                   WHEN f.statement_type IN ('BS', 'STD_BS') AND l.debit_credit = 'CREDIT'
                   THEN l.amount ELSE 0 END), 0)::BIGINT AS bs_credit_total,
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
    sqlx::query_as::<_, FinancialStatementTotals>(&sql)
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("failed to load financial statement totals")
}

async fn load_unmapped_std_fs_line_count(
    pool: &PgPool,
    tenant: &TenantRef,
    context: &WorkspaceMappingContext,
) -> Result<i64> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM {schema}.financial_statements f
        JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
        LEFT JOIN public.std_fs_items i
          ON i.version_id = $2
         AND i.item_code = l.std_fs_item_code
         AND i.is_active = TRUE
        WHERE f.by_id = $1
          AND f.statement_type IN ('BS', 'IS', 'STD_BS', 'STD_IS')
          AND (
                l.std_fs_item_code IS NULL
             OR TRIM(l.std_fs_item_code) = ''
             OR i.id IS NULL
             OR i.is_subtotal = TRUE
             OR i.stmt_type <> CASE f.statement_type
                    WHEN 'BS' THEN 'STD_BS'
                    WHEN 'IS' THEN 'STD_IS'
                    ELSE f.statement_type
                END
          )
        "#
    );
    sqlx::query_scalar::<_, i64>(&sql)
        .bind(context.business_year.by_id)
        .bind(context.version_id)
        .fetch_one(pool)
        .await
        .context("failed to count unmapped std-fs lines")
}

async fn confirmed_statement_count(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    version_id: Uuid,
) -> Result<i64> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM {schema}.std_fs_statements
        WHERE business_year_id = $1
          AND version_id = $2
          AND status = 'CONFIRMED'
        "#
    );
    sqlx::query_scalar::<_, i64>(&sql)
        .bind(by_id)
        .bind(version_id)
        .fetch_one(pool)
        .await
        .context("failed to count confirmed std-fs statements")
}

async fn load_confirmed_statement_lines(
    pool: &PgPool,
    tenant: &TenantRef,
    context: &WorkspaceMappingContext,
    stmt_type: Option<&str>,
) -> Result<Vec<StdFsStatementLine>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT s.stmt_type,
               s.item_code,
               i.item_name,
               i.parent_code,
               i.level,
               i.account_class,
               i.normal_balance,
               i.is_subtotal,
               i.is_required,
               i.sort_order,
               s.amount,
               s.source_line_ids,
               s.total_check,
               s.confirmed_at
        FROM {schema}.std_fs_statements s
        JOIN public.std_fs_items i
          ON i.version_id = s.version_id
         AND i.item_code = s.item_code
        WHERE s.business_year_id = $1
          AND s.version_id = $2
          AND s.status = 'CONFIRMED'
          AND ($3::TEXT IS NULL OR s.stmt_type = $3)
        ORDER BY s.stmt_type, i.sort_order NULLS LAST, s.item_code
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(context.business_year.by_id)
        .bind(context.version_id)
        .bind(stmt_type)
        .fetch_all(pool)
        .await
        .context("failed to load confirmed std-fs statements")?;
    let mut lines = Vec::with_capacity(rows.len());
    for row in rows {
        lines.push(StdFsStatementLine {
            by_id: context.business_year.by_id,
            version_id: context.version_id,
            stmt_type: row.try_get("stmt_type")?,
            item_code: row.try_get("item_code")?,
            item_name: row.try_get("item_name")?,
            parent_code: row.try_get("parent_code")?,
            level: row.try_get("level")?,
            account_class: row.try_get("account_class")?,
            normal_balance: row.try_get("normal_balance")?,
            is_subtotal: row.try_get("is_subtotal")?,
            is_required: row.try_get("is_required")?,
            sort_order: row.try_get("sort_order")?,
            amount: row.try_get("amount")?,
            source_line_ids: row.try_get("source_line_ids")?,
            total_check: row.try_get("total_check")?,
            confirmed: true,
            confirmed_at: row.try_get::<Option<DateTime<Utc>>, _>("confirmed_at")?,
        });
    }
    sort_statement_lines(&mut lines);
    Ok(lines)
}

fn evaluate_agg_formula(
    formula: &str,
    states: &BTreeMap<String, StdFsAggregateState>,
) -> Result<(i64, Vec<i64>)> {
    let normalized = formula.replace('+', " + ").replace('-', " - ");
    let mut sign = 1_i64;
    let mut amount = 0_i64;
    let mut source_line_ids = Vec::new();
    let mut saw_reference = false;
    for token in normalized.split_whitespace() {
        match token {
            "+" => sign = 1,
            "-" => sign = -1,
            item_code => {
                let state = states.get(item_code).ok_or_else(|| {
                    anyhow!("invalid agg_formula reference: item_code {item_code} not found")
                })?;
                amount += sign * state.amount;
                extend_unique_sorted(&mut source_line_ids, state.source_line_ids.clone());
                sign = 1;
                saw_reference = true;
            }
        }
    }
    if !saw_reference {
        anyhow::bail!("invalid agg_formula: empty formula");
    }
    Ok((amount, source_line_ids))
}

fn normalize_statement_filter(stmt_type: Option<&str>) -> Result<Option<String>> {
    let Some(stmt_type) = stmt_type else {
        return Ok(None);
    };
    let stmt_type = normalize_stmt_type(stmt_type)?;
    if !matches!(stmt_type.as_str(), "STD_BS" | "STD_IS") {
        anyhow::bail!("invalid stmt_type: only STD_BS or STD_IS is supported");
    }
    Ok(Some(stmt_type))
}

fn line_amount_by_code(lines: &[StdFsStatementLine], item_code: &str) -> i64 {
    lines
        .iter()
        .find(|line| line.item_code == item_code)
        .map(|line| line.amount)
        .unwrap_or(0)
}

fn standard_income_profit_loss(lines: &[StdFsStatementLine]) -> i64 {
    lines
        .iter()
        .filter(|line| line.stmt_type == "STD_IS" && !line.is_subtotal)
        .map(|line| {
            let account_class = line.account_class.as_deref().unwrap_or_default();
            match account_class {
                "REVENUE" | "GAIN" => line.amount,
                "EXPENSE" | "LOSS" => -line.amount,
                _ => match line.normal_balance.as_deref() {
                    Some("CREDIT") => line.amount,
                    Some("DEBIT") => -line.amount,
                    _ => 0,
                },
            }
        })
        .sum()
}

fn std_validation_issue(
    rule_code: &str,
    severity: &str,
    message: &str,
    expected: i64,
    actual: i64,
    metadata: Value,
) -> StdFsValidationIssue {
    let difference = actual - expected;
    StdFsValidationIssue {
        rule_code: rule_code.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
        passed: difference == 0,
        expected,
        actual,
        difference,
        metadata,
    }
}

fn json_i64_array(value: &Value) -> Vec<i64> {
    value
        .as_array()
        .map(|values| values.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default()
}

fn extend_unique_sorted(target: &mut Vec<i64>, values: Vec<i64>) {
    target.extend(values);
    target.sort_unstable();
    target.dedup();
}

fn sort_statement_lines(lines: &mut [StdFsStatementLine]) {
    lines.sort_by(|left, right| {
        left.stmt_type
            .cmp(&right.stmt_type)
            .then_with(|| {
                left.sort_order
                    .unwrap_or(i32::MAX)
                    .cmp(&right.sort_order.unwrap_or(i32::MAX))
            })
            .then_with(|| left.item_code.cmp(&right.item_code))
    });
}

async fn list_workspace_mappings_with_context(
    pool: &PgPool,
    tenant: &TenantRef,
    context: &WorkspaceMappingContext,
) -> Result<Vec<StdFsMappingRow>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        WITH account_lines AS (
            SELECT f.statement_type,
                   l.account_code,
                   MAX(l.account_name) AS account_name,
                   COALESCE(SUM(CASE WHEN l.debit_credit = 'DEBIT' THEN l.amount ELSE 0 END), 0)::BIGINT AS debit_total,
                   COALESCE(SUM(CASE WHEN l.debit_credit = 'CREDIT' THEN l.amount ELSE 0 END), 0)::BIGINT AS credit_total,
                   COALESCE(SUM(l.amount), 0)::BIGINT AS amount,
                   CASE
                       WHEN COALESCE(SUM(CASE WHEN l.debit_credit = 'DEBIT' THEN l.amount ELSE -l.amount END), 0) >= 0
                       THEN 'DEBIT'
                       ELSE 'CREDIT'
                   END AS debit_credit,
                   MAX(l.std_fs_item_code) FILTER (WHERE l.std_fs_item_code IS NOT NULL) AS line_std_fs_item_code
            FROM {schema}.financial_statements f
            JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
            WHERE f.by_id = $1
            GROUP BY f.statement_type, l.account_code
        )
        SELECT $1::BIGINT AS by_id,
               $2::BIGINT AS customer_id,
               $3::UUID AS version_id,
               a.statement_type,
               a.account_code,
               a.account_name,
               a.debit_total,
               a.credit_total,
               a.amount,
               a.debit_credit,
               COALESCE(m.std_fs_item_code, a.line_std_fs_item_code) AS std_fs_item_code,
               i.item_name AS std_fs_item_name,
               i.is_subtotal AS mapped_is_subtotal,
               m.id AS mapping_id,
               COALESCE(m.is_auto_mapped, FALSE) AS is_auto_mapped,
               m.usage_count,
               m.updated_at AS mapped_at
        FROM account_lines a
        LEFT JOIN {schema}.std_fs_mappings m
               ON m.customer_id = $2
              AND m.version_id = $3
              AND m.account_code = a.account_code
        LEFT JOIN public.std_fs_items i
               ON i.version_id = $3
              AND i.item_code = COALESCE(m.std_fs_item_code, a.line_std_fs_item_code)
        ORDER BY a.statement_type, a.account_code
        "#
    );
    sqlx::query_as::<_, StdFsMappingRow>(&sql)
        .bind(context.business_year.by_id)
        .bind(context.business_year.customer_id)
        .bind(context.version_id)
        .fetch_all(pool)
        .await
        .context("failed to list std-fs mappings")
}

async fn save_mapping_in_tx(
    pool: &PgPool,
    tx: &mut Transaction<'_, Postgres>,
    tenant: &TenantRef,
    context: &WorkspaceMappingContext,
    account_code: &str,
    account_name: Option<&str>,
    std_fs_item_code: Option<&str>,
    user_id: i64,
) -> Result<(usize, usize)> {
    let current_account_name =
        current_account_name(pool, tenant, context.business_year.by_id, account_code)
            .await?
            .ok_or_else(|| {
                anyhow!(
            "invalid account_code: {account_code} not found in business year financial statements"
        )
            })?;
    if let Some(std_fs_item_code) = std_fs_item_code {
        validate_leaf_std_fs_item(pool, context.version_id, std_fs_item_code).await?;
        let account_name =
            normalize_optional_text(account_name, "account_name")?.unwrap_or(current_account_name);
        upsert_std_fs_mapping_in_tx(
            tx,
            tenant,
            context,
            account_code,
            Some(&account_name),
            std_fs_item_code,
            false,
            user_id,
        )
        .await?;
        apply_std_fs_mapping_to_lines_in_tx(
            tx,
            tenant,
            context.business_year.by_id,
            account_code,
            std_fs_item_code,
        )
        .await?;
        Ok((1, 0))
    } else {
        clear_std_fs_mapping_in_tx(tx, tenant, context, account_code).await?;
        Ok((0, 1))
    }
}

async fn resolve_mapping_context(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<WorkspaceMappingContext> {
    let business_year = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = tax::ensure_law_snapshot(pool, tenant, by_id).await?;
    let version_id = snapshot
        .std_fs_version_id
        .ok_or_else(|| anyhow!("standard financial statement version not found"))?;
    Ok(WorkspaceMappingContext {
        business_year,
        version_id,
    })
}

async fn current_account_name(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    account_code: &str,
) -> Result<Option<String>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT MAX(l.account_name) AS account_name
        FROM {schema}.financial_statements f
        JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
        WHERE f.by_id = $1
          AND l.account_code = $2
        "#
    );
    sqlx::query_scalar::<_, Option<String>>(&sql)
        .bind(by_id)
        .bind(account_code)
        .fetch_one(pool)
        .await
        .context("failed to resolve financial statement account")
}

async fn validate_leaf_std_fs_item(pool: &PgPool, version_id: Uuid, item_code: &str) -> Result<()> {
    let row = sqlx::query(
        r#"
        SELECT is_subtotal
        FROM public.std_fs_items
        WHERE version_id = $1
          AND item_code = $2
          AND is_active = TRUE
        "#,
    )
    .bind(version_id)
    .bind(item_code)
    .fetch_optional(pool)
    .await
    .context("failed to validate std-fs item")?
    .ok_or_else(|| anyhow!("invalid std_fs_item_code: active item not found"))?;
    if row.get::<bool, _>("is_subtotal") {
        anyhow::bail!("invalid std_fs_item_code: leaf item is required");
    }
    Ok(())
}

async fn load_leaf_item_codes(pool: &PgPool, version_id: Uuid) -> Result<HashSet<String>> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT item_code
        FROM public.std_fs_items
        WHERE version_id = $1
          AND is_active = TRUE
          AND is_subtotal = FALSE
        "#,
    )
    .bind(version_id)
    .fetch_all(pool)
    .await
    .context("failed to load std-fs leaf item codes")?;
    Ok(rows.into_iter().collect())
}

async fn upsert_std_fs_mapping_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    tenant: &TenantRef,
    context: &WorkspaceMappingContext,
    account_code: &str,
    account_name: Option<&str>,
    std_fs_item_code: &str,
    is_auto_mapped: bool,
    user_id: i64,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.std_fs_mappings (
            tenant_id, customer_id, version_id, account_code, account_name,
            std_fs_item_code, is_auto_mapped, last_used_at, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), $8)
        ON CONFLICT (customer_id, version_id, account_code)
        DO UPDATE SET
            account_name = EXCLUDED.account_name,
            std_fs_item_code = EXCLUDED.std_fs_item_code,
            is_auto_mapped = EXCLUDED.is_auto_mapped,
            usage_count = {schema}.std_fs_mappings.usage_count + 1,
            last_used_at = NOW(),
            created_by = COALESCE({schema}.std_fs_mappings.created_by, EXCLUDED.created_by),
            updated_at = NOW()
        "#
    );
    sqlx::query(&sql)
        .bind(tenant.tenant_id)
        .bind(context.business_year.customer_id)
        .bind(context.version_id)
        .bind(account_code)
        .bind(account_name)
        .bind(std_fs_item_code)
        .bind(is_auto_mapped)
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .context("failed to save std-fs mapping")?;
    Ok(())
}

async fn apply_std_fs_mapping_to_lines_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    tenant: &TenantRef,
    by_id: i64,
    account_code: &str,
    std_fs_item_code: &str,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.fs_lines l
        SET std_fs_item_code = $1
        FROM {schema}.financial_statements f
        WHERE f.fs_id = l.fs_id
          AND f.by_id = $2
          AND l.account_code = $3
        "#
    );
    sqlx::query(&sql)
        .bind(std_fs_item_code)
        .bind(by_id)
        .bind(account_code)
        .execute(&mut **tx)
        .await
        .context("failed to apply std-fs mapping to financial statement lines")?;
    Ok(())
}

async fn clear_std_fs_mapping_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    tenant: &TenantRef,
    context: &WorkspaceMappingContext,
    account_code: &str,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let delete_sql = format!(
        r#"
        DELETE FROM {schema}.std_fs_mappings
        WHERE customer_id = $1
          AND version_id = $2
          AND account_code = $3
        "#
    );
    sqlx::query(&delete_sql)
        .bind(context.business_year.customer_id)
        .bind(context.version_id)
        .bind(account_code)
        .execute(&mut **tx)
        .await
        .context("failed to clear std-fs mapping")?;

    let update_sql = format!(
        r#"
        UPDATE {schema}.fs_lines l
        SET std_fs_item_code = NULL
        FROM {schema}.financial_statements f
        WHERE f.fs_id = l.fs_id
          AND f.by_id = $1
          AND l.account_code = $2
        "#
    );
    sqlx::query(&update_sql)
        .bind(context.business_year.by_id)
        .bind(account_code)
        .execute(&mut **tx)
        .await
        .context("failed to clear std-fs mapping from financial statement lines")?;
    Ok(())
}

async fn previous_business_year_id(
    pool: &PgPool,
    tenant: &TenantRef,
    target: &BusinessYear,
) -> Result<Option<i64>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT by_id
        FROM {schema}.business_years
        WHERE customer_id = $1
          AND by_id <> $2
          AND end_date < $3
        ORDER BY end_date DESC, by_id DESC
        LIMIT 1
        "#
    );
    sqlx::query_scalar::<_, i64>(&sql)
        .bind(target.customer_id)
        .bind(target.by_id)
        .bind(target.start_date)
        .fetch_optional(pool)
        .await
        .context("failed to find previous business year")
}

async fn load_source_std_fs_mappings(
    pool: &PgPool,
    tenant: &TenantRef,
    context: &WorkspaceMappingContext,
) -> Result<Vec<SourceStdFsMapping>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        WITH candidates AS (
            SELECT m.account_code,
                   m.account_name,
                   m.std_fs_item_code,
                   1 AS priority,
                   m.updated_at AS touched_at
            FROM {schema}.std_fs_mappings m
            WHERE m.customer_id = $1
              AND m.version_id = $2
            UNION ALL
            SELECT l.account_code,
                   MAX(l.account_name) AS account_name,
                   MAX(l.std_fs_item_code) AS std_fs_item_code,
                   2 AS priority,
                   MAX(f.created_at) AS touched_at
            FROM {schema}.financial_statements f
            JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
            WHERE f.by_id = $3
              AND l.std_fs_item_code IS NOT NULL
              AND TRIM(l.std_fs_item_code) <> ''
            GROUP BY l.account_code
        )
        SELECT DISTINCT ON (account_code)
               account_code, account_name, std_fs_item_code
        FROM candidates
        WHERE std_fs_item_code IS NOT NULL
          AND TRIM(std_fs_item_code) <> ''
        ORDER BY account_code, priority, touched_at DESC NULLS LAST
        "#
    );
    sqlx::query_as::<_, SourceStdFsMapping>(&sql)
        .bind(context.business_year.customer_id)
        .bind(context.version_id)
        .bind(context.business_year.by_id)
        .fetch_all(pool)
        .await
        .context("failed to load source std-fs mappings")
}

fn normalize_account_code(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("account_code is required");
    }
    if value.len() > 50 {
        anyhow::bail!("invalid account_code: maximum length is 50");
    }
    Ok(value.to_string())
}

fn normalize_optional_item_code(value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 10 {
        anyhow::bail!("invalid std_fs_item_code: maximum length is 10");
    }
    Ok(Some(value.to_ascii_uppercase()))
}

fn normalize_optional_text(value: Option<&str>, field_name: &str) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 200 {
        anyhow::bail!("invalid {field_name}: maximum length is 200");
    }
    Ok(Some(value.to_string()))
}

pub async fn list_versions(
    pool: &PgPool,
    status: Option<&str>,
    industry_type: Option<&str>,
    corp_type: Option<&str>,
) -> Result<Vec<StdFsItemVersion>> {
    let status = status.map(normalize_version_status).transpose()?;
    let industry_type = industry_type
        .map(|value| normalize_required_upper(value, "industry_type"))
        .transpose()?;
    let corp_type = corp_type
        .map(|value| normalize_required_upper(value, "corp_type"))
        .transpose()?;
    sqlx::query_as::<_, StdFsItemVersion>(
        r#"
        SELECT id, version_code, industry_type, corp_type, effective_from, effective_to,
               nts_doc_ref, status, xml_schema_ver, created_by, reviewed_by,
               created_at, activated_at
        FROM std_fs_item_versions
        WHERE ($1::TEXT IS NULL OR status = $1)
          AND ($2::TEXT IS NULL OR industry_type = $2)
          AND ($3::TEXT IS NULL OR corp_type = $3)
        ORDER BY effective_from DESC, version_code
        "#,
    )
    .bind(status)
    .bind(industry_type)
    .bind(corp_type)
    .fetch_all(pool)
    .await
    .context("failed to list standard financial statement versions")
}

pub async fn get_version(pool: &PgPool, version_id: Uuid) -> Result<StdFsItemVersion> {
    sqlx::query_as::<_, StdFsItemVersion>(
        r#"
        SELECT id, version_code, industry_type, corp_type, effective_from, effective_to,
               nts_doc_ref, status, xml_schema_ver, created_by, reviewed_by,
               created_at, activated_at
        FROM std_fs_item_versions
        WHERE id = $1
        "#,
    )
    .bind(version_id)
    .fetch_one(pool)
    .await
    .context("standard financial statement version not found")
}

pub async fn create_version(
    pool: &PgPool,
    request: CreateStdFsVersionRequest,
    created_by: i64,
) -> Result<StdFsItemVersion> {
    let version_code = normalize_required_upper(&request.version_code, "version_code")?;
    let industry_type = normalize_required_upper(&request.industry_type, "industry_type")?;
    let corp_type = request
        .corp_type
        .as_deref()
        .map(|value| normalize_required_upper(value, "corp_type"))
        .transpose()?
        .unwrap_or_else(|| "DOMESTIC".to_string());
    let status = normalize_version_status(request.status.as_deref().unwrap_or("DRAFT"))?;
    if status == "ACTIVE" {
        return Err(anyhow!(
            "invalid status: create version as DRAFT or REVIEWED first"
        ));
    }
    validate_effective_range(request.effective_from, request.effective_to)?;

    sqlx::query_as::<_, StdFsItemVersion>(
        r#"
        INSERT INTO std_fs_item_versions (
            version_code, industry_type, corp_type, effective_from, effective_to,
            nts_doc_ref, status, xml_schema_ver, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, version_code, industry_type, corp_type, effective_from, effective_to,
                  nts_doc_ref, status, xml_schema_ver, created_by, reviewed_by,
                  created_at, activated_at
        "#,
    )
    .bind(version_code)
    .bind(industry_type)
    .bind(corp_type)
    .bind(request.effective_from)
    .bind(request.effective_to)
    .bind(trim_optional(request.nts_doc_ref))
    .bind(status)
    .bind(trim_optional(request.xml_schema_ver))
    .bind(created_by)
    .fetch_one(pool)
    .await
    .context("failed to create standard financial statement version")
}

pub async fn update_version(
    pool: &PgPool,
    version_id: Uuid,
    request: UpdateStdFsVersionRequest,
) -> Result<StdFsItemVersion> {
    ensure_version_editable(pool, version_id).await?;
    if let Some(effective_from) = request.effective_from {
        validate_effective_range(effective_from, request.effective_to)?;
    }
    let version_code = request
        .version_code
        .as_deref()
        .map(|value| normalize_required_upper(value, "version_code"))
        .transpose()?;
    let industry_type = request
        .industry_type
        .as_deref()
        .map(|value| normalize_required_upper(value, "industry_type"))
        .transpose()?;
    let corp_type = request
        .corp_type
        .as_deref()
        .map(|value| normalize_required_upper(value, "corp_type"))
        .transpose()?;

    sqlx::query_as::<_, StdFsItemVersion>(
        r#"
        UPDATE std_fs_item_versions
        SET version_code = COALESCE($2, version_code),
            industry_type = COALESCE($3, industry_type),
            corp_type = COALESCE($4, corp_type),
            effective_from = COALESCE($5, effective_from),
            effective_to = COALESCE($6, effective_to),
            nts_doc_ref = COALESCE($7, nts_doc_ref),
            xml_schema_ver = COALESCE($8, xml_schema_ver)
        WHERE id = $1
        RETURNING id, version_code, industry_type, corp_type, effective_from, effective_to,
                  nts_doc_ref, status, xml_schema_ver, created_by, reviewed_by,
                  created_at, activated_at
        "#,
    )
    .bind(version_id)
    .bind(version_code)
    .bind(industry_type)
    .bind(corp_type)
    .bind(request.effective_from)
    .bind(request.effective_to)
    .bind(trim_optional(request.nts_doc_ref))
    .bind(trim_optional(request.xml_schema_ver))
    .fetch_one(pool)
    .await
    .context("failed to update standard financial statement version")
}

pub async fn delete_version(pool: &PgPool, version_id: Uuid) -> Result<()> {
    ensure_version_editable(pool, version_id).await?;
    sqlx::query("DELETE FROM std_fs_item_versions WHERE id = $1")
        .bind(version_id)
        .execute(pool)
        .await
        .context("failed to delete standard financial statement version")?;
    Ok(())
}

pub async fn clone_version(
    pool: &PgPool,
    source_version_id: Uuid,
    request: CloneStdFsVersionRequest,
    created_by: i64,
) -> Result<StdFsItemVersion> {
    let source = get_version(pool, source_version_id).await?;
    let version_code = normalize_required_upper(&request.version_code, "version_code")?;
    let industry_type = request
        .industry_type
        .as_deref()
        .map(|value| normalize_required_upper(value, "industry_type"))
        .transpose()?
        .unwrap_or(source.industry_type);
    let corp_type = request
        .corp_type
        .as_deref()
        .map(|value| normalize_required_upper(value, "corp_type"))
        .transpose()?
        .unwrap_or(source.corp_type);
    let effective_from = request.effective_from.unwrap_or(source.effective_from);
    let effective_to = request.effective_to.or(source.effective_to);
    validate_effective_range(effective_from, effective_to)?;

    let mut tx = pool
        .begin()
        .await
        .context("failed to begin standard financial statement clone")?;
    let cloned = sqlx::query_as::<_, StdFsItemVersion>(
        r#"
        INSERT INTO std_fs_item_versions (
            version_code, industry_type, corp_type, effective_from, effective_to,
            nts_doc_ref, status, xml_schema_ver, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'DRAFT', $7, $8)
        RETURNING id, version_code, industry_type, corp_type, effective_from, effective_to,
                  nts_doc_ref, status, xml_schema_ver, created_by, reviewed_by,
                  created_at, activated_at
        "#,
    )
    .bind(version_code)
    .bind(industry_type)
    .bind(corp_type)
    .bind(effective_from)
    .bind(effective_to)
    .bind(trim_optional(request.nts_doc_ref).or(source.nts_doc_ref))
    .bind(trim_optional(request.xml_schema_ver).or(source.xml_schema_ver))
    .bind(created_by)
    .fetch_one(&mut *tx)
    .await
    .context("failed to clone standard financial statement version")?;

    sqlx::query(
        r#"
        INSERT INTO std_fs_items (
            version_id, stmt_type, item_code, item_name, parent_code, level,
            account_class, normal_balance, is_subtotal, is_required, agg_formula,
            xml_field_id, sort_order, is_active
        )
        SELECT $2, stmt_type, item_code, item_name, parent_code, level,
               account_class, normal_balance, is_subtotal, is_required, agg_formula,
               xml_field_id, sort_order, is_active
        FROM std_fs_items
        WHERE version_id = $1
        ORDER BY stmt_type, sort_order, item_code
        "#,
    )
    .bind(source_version_id)
    .bind(cloned.id)
    .execute(&mut *tx)
    .await
    .context("failed to clone standard financial statement items")?;

    tx.commit()
        .await
        .context("failed to commit standard financial statement clone")?;
    Ok(cloned)
}

pub async fn update_version_status(
    pool: &PgPool,
    version_id: Uuid,
    request: UpdateStdFsVersionStatusRequest,
    actor_user_id: i64,
) -> Result<StdFsItemVersion> {
    let current = get_version(pool, version_id).await?;
    let status = normalize_version_status(&request.status)?;
    validate_status_transition(&current.status, &status)?;
    if status == "ACTIVE" {
        let integrity = check_integrity(pool, version_id).await?;
        if !integrity.valid {
            return Err(anyhow!(
                "invalid integrity: standard financial statement version has {} error(s)",
                integrity.error_count
            ));
        }
    }

    sqlx::query_as::<_, StdFsItemVersion>(
        r#"
        UPDATE std_fs_item_versions
        SET status = $2,
            reviewed_by = CASE
                WHEN $2 IN ('REVIEWED', 'ACTIVE') THEN COALESCE(reviewed_by, $3)
                ELSE reviewed_by
            END,
            activated_at = CASE
                WHEN $2 = 'ACTIVE' THEN COALESCE(activated_at, NOW())
                WHEN $2 = 'DRAFT' THEN NULL
                ELSE activated_at
            END
        WHERE id = $1
        RETURNING id, version_code, industry_type, corp_type, effective_from, effective_to,
                  nts_doc_ref, status, xml_schema_ver, created_by, reviewed_by,
                  created_at, activated_at
        "#,
    )
    .bind(version_id)
    .bind(status)
    .bind(actor_user_id)
    .fetch_one(pool)
    .await
    .context("failed to update standard financial statement version status")
}

pub async fn list_items(
    pool: &PgPool,
    version_id: Uuid,
    stmt_type: Option<&str>,
    include_inactive: bool,
) -> Result<Vec<StdFsItem>> {
    get_version(pool, version_id).await?;
    let stmt_type = stmt_type.map(normalize_stmt_type).transpose()?;
    sqlx::query_as::<_, StdFsItem>(
        r#"
        SELECT id, version_id, stmt_type, item_code, item_name, parent_code, level,
               account_class, normal_balance, is_subtotal, is_required, agg_formula,
               xml_field_id, sort_order, is_active
        FROM std_fs_items
        WHERE version_id = $1
          AND ($2::TEXT IS NULL OR stmt_type = $2)
          AND ($3::BOOL OR is_active = TRUE)
        ORDER BY stmt_type, sort_order NULLS LAST, item_code
        "#,
    )
    .bind(version_id)
    .bind(stmt_type)
    .bind(include_inactive)
    .fetch_all(pool)
    .await
    .context("failed to list standard financial statement items")
}

pub async fn get_item(pool: &PgPool, item_id: Uuid) -> Result<StdFsItem> {
    sqlx::query_as::<_, StdFsItem>(
        r#"
        SELECT id, version_id, stmt_type, item_code, item_name, parent_code, level,
               account_class, normal_balance, is_subtotal, is_required, agg_formula,
               xml_field_id, sort_order, is_active
        FROM std_fs_items
        WHERE id = $1
        "#,
    )
    .bind(item_id)
    .fetch_one(pool)
    .await
    .context("standard financial statement item not found")
}

pub async fn create_item(
    pool: &PgPool,
    version_id: Uuid,
    request: CreateStdFsItemRequest,
) -> Result<StdFsItem> {
    ensure_version_editable(pool, version_id).await?;
    let stmt_type = normalize_stmt_type(&request.stmt_type)?;
    let item_code = normalize_required_upper(&request.item_code, "item_code")?;
    let item_name = normalize_required(&request.item_name, "item_name")?;
    let parent_code = request
        .parent_code
        .as_deref()
        .map(|value| normalize_optional_upper(value, "parent_code"))
        .transpose()?
        .flatten();
    let account_class = request
        .account_class
        .as_deref()
        .map(|value| normalize_optional_upper(value, "account_class"))
        .transpose()?
        .flatten();
    let normal_balance = request
        .normal_balance
        .as_deref()
        .map(normalize_normal_balance)
        .transpose()?;
    let level = request.level.unwrap_or(1);
    if level <= 0 {
        return Err(anyhow!("invalid level"));
    }

    sqlx::query_as::<_, StdFsItem>(
        r#"
        INSERT INTO std_fs_items (
            version_id, stmt_type, item_code, item_name, parent_code, level,
            account_class, normal_balance, is_subtotal, is_required, agg_formula,
            xml_field_id, sort_order, is_active
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, COALESCE($9, FALSE), COALESCE($10, FALSE), $11,
            $12, $13, COALESCE($14, TRUE)
        )
        RETURNING id, version_id, stmt_type, item_code, item_name, parent_code, level,
                  account_class, normal_balance, is_subtotal, is_required, agg_formula,
                  xml_field_id, sort_order, is_active
        "#,
    )
    .bind(version_id)
    .bind(stmt_type)
    .bind(item_code)
    .bind(item_name)
    .bind(parent_code)
    .bind(level)
    .bind(account_class)
    .bind(normal_balance)
    .bind(request.is_subtotal)
    .bind(request.is_required)
    .bind(trim_optional(request.agg_formula))
    .bind(trim_optional(request.xml_field_id))
    .bind(request.sort_order)
    .bind(request.is_active)
    .fetch_one(pool)
    .await
    .context("failed to create standard financial statement item")
}

pub async fn update_item(
    pool: &PgPool,
    item_id: Uuid,
    request: UpdateStdFsItemRequest,
) -> Result<StdFsItem> {
    let current = get_item(pool, item_id).await?;
    ensure_version_editable(pool, current.version_id).await?;
    let stmt_type = request
        .stmt_type
        .as_deref()
        .map(normalize_stmt_type)
        .transpose()?;
    let item_code = request
        .item_code
        .as_deref()
        .map(|value| normalize_required_upper(value, "item_code"))
        .transpose()?;
    let item_name = request
        .item_name
        .as_deref()
        .map(|value| normalize_required(value, "item_name"))
        .transpose()?;
    let parent_code = request
        .parent_code
        .as_deref()
        .map(|value| normalize_nullable_upper(value, "parent_code"))
        .transpose()?;
    let account_class = request
        .account_class
        .as_deref()
        .map(|value| normalize_nullable_upper(value, "account_class"))
        .transpose()?;
    let normal_balance = request
        .normal_balance
        .as_deref()
        .map(normalize_nullable_normal_balance)
        .transpose()?;
    if request.level.is_some_and(|level| level <= 0) {
        return Err(anyhow!("invalid level"));
    }

    sqlx::query_as::<_, StdFsItem>(
        r#"
        UPDATE std_fs_items
        SET stmt_type = COALESCE($2, stmt_type),
            item_code = COALESCE($3, item_code),
            item_name = COALESCE($4, item_name),
            parent_code = CASE WHEN $5::TEXT IS NULL THEN parent_code ELSE NULLIF($5, '') END,
            level = COALESCE($6, level),
            account_class = CASE WHEN $7::TEXT IS NULL THEN account_class ELSE NULLIF($7, '') END,
            normal_balance = CASE WHEN $8::TEXT IS NULL THEN normal_balance ELSE NULLIF($8, '') END,
            is_subtotal = COALESCE($9, is_subtotal),
            is_required = COALESCE($10, is_required),
            agg_formula = CASE WHEN $11::TEXT IS NULL THEN agg_formula ELSE NULLIF($11, '') END,
            xml_field_id = CASE WHEN $12::TEXT IS NULL THEN xml_field_id ELSE NULLIF($12, '') END,
            sort_order = COALESCE($13, sort_order),
            is_active = COALESCE($14, is_active)
        WHERE id = $1
        RETURNING id, version_id, stmt_type, item_code, item_name, parent_code, level,
                  account_class, normal_balance, is_subtotal, is_required, agg_formula,
                  xml_field_id, sort_order, is_active
        "#,
    )
    .bind(item_id)
    .bind(stmt_type)
    .bind(item_code)
    .bind(item_name)
    .bind(parent_code)
    .bind(request.level)
    .bind(account_class)
    .bind(normal_balance)
    .bind(request.is_subtotal)
    .bind(request.is_required)
    .bind(trim_nullable(request.agg_formula))
    .bind(trim_nullable(request.xml_field_id))
    .bind(request.sort_order)
    .bind(request.is_active)
    .fetch_one(pool)
    .await
    .context("failed to update standard financial statement item")
}

pub async fn delete_item(pool: &PgPool, item_id: Uuid) -> Result<()> {
    let item = get_item(pool, item_id).await?;
    ensure_version_editable(pool, item.version_id).await?;
    sqlx::query("DELETE FROM std_fs_items WHERE id = $1")
        .bind(item_id)
        .execute(pool)
        .await
        .context("failed to delete standard financial statement item")?;
    Ok(())
}

pub fn import_template_csv() -> String {
    format!(
        "{}\nSTD_BS,1000,Assets,,1,ASSET,DEBIT,true,true,1010+1020,BS_ASSET_TOTAL,100,true\nSTD_BS,1010,Cash and cash equivalents,1000,2,ASSET,DEBIT,false,false,,BS_CASH,110,true\n",
        IMPORT_HEADERS.join(",")
    )
}

pub async fn import_items(
    pool: &PgPool,
    version_id: Uuid,
    file_name: Option<String>,
    bytes: &[u8],
) -> Result<StdFsImportReport> {
    ensure_version_editable(pool, version_id).await?;
    let parsed = parse_import_tabular(bytes, file_name.as_deref())?;
    let total_rows = parsed.rows.len();
    let (rows, mut issues) = parse_import_item_rows(&parsed.rows);
    issues.extend(parsed.issues);

    let existing = list_items(pool, version_id, None, true).await?;
    issues.extend(validate_import_preconditions(version_id, &existing, &rows));
    let final_items = final_items_after_import(version_id, &existing, &rows);
    let integrity = integrity_for_items(version_id, &final_items);
    let imported_row_by_code = rows
        .iter()
        .map(|row| (row.item_code.clone(), row.row_no))
        .collect::<HashMap<_, _>>();
    for integrity_issue in &integrity.issues {
        issues.push(import_issue(
            imported_row_by_code
                .get(integrity_issue.item_code.as_deref().unwrap_or_default())
                .copied()
                .unwrap_or(0),
            &integrity_issue.severity,
            &integrity_issue.code,
            integrity_issue.item_code.as_deref().map(|_| "item_code"),
            integrity_issue.item_code.clone(),
            &integrity_issue.message,
            json!({}),
        ));
    }

    let has_error = issues.iter().any(|issue| issue.severity == "ERROR");
    if has_error {
        return Ok(import_report(
            version_id,
            "VALIDATION_FAILED",
            total_rows,
            valid_row_count(total_rows, &issues),
            0,
            0,
            0,
            issues,
            Some(integrity),
        ));
    }

    let existing_by_key = existing
        .iter()
        .map(|item| ((item.stmt_type.clone(), item.item_code.clone()), item))
        .collect::<HashMap<_, _>>();
    let mut inserted_count = 0;
    let mut updated_count = 0;
    let mut unchanged_count = 0;
    for row in &rows {
        match existing_by_key.get(&(row.stmt_type.clone(), row.item_code.clone())) {
            Some(existing) if import_row_matches_item(row, existing) => unchanged_count += 1,
            Some(_) => updated_count += 1,
            None => inserted_count += 1,
        }
    }

    let mut tx = pool
        .begin()
        .await
        .context("failed to begin standard financial statement import")?;
    for row in &rows {
        sqlx::query(
            r#"
            INSERT INTO std_fs_items (
                version_id, stmt_type, item_code, item_name, parent_code, level,
                account_class, normal_balance, is_subtotal, is_required, agg_formula,
                xml_field_id, sort_order, is_active
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (version_id, stmt_type, item_code)
            DO UPDATE SET
                item_name = EXCLUDED.item_name,
                parent_code = EXCLUDED.parent_code,
                level = EXCLUDED.level,
                account_class = EXCLUDED.account_class,
                normal_balance = EXCLUDED.normal_balance,
                is_subtotal = EXCLUDED.is_subtotal,
                is_required = EXCLUDED.is_required,
                agg_formula = EXCLUDED.agg_formula,
                xml_field_id = EXCLUDED.xml_field_id,
                sort_order = EXCLUDED.sort_order,
                is_active = EXCLUDED.is_active
            "#,
        )
        .bind(version_id)
        .bind(&row.stmt_type)
        .bind(&row.item_code)
        .bind(&row.item_name)
        .bind(&row.parent_code)
        .bind(row.level)
        .bind(&row.account_class)
        .bind(&row.normal_balance)
        .bind(row.is_subtotal)
        .bind(row.is_required)
        .bind(&row.agg_formula)
        .bind(&row.xml_field_id)
        .bind(row.sort_order)
        .bind(row.is_active)
        .execute(&mut *tx)
        .await
        .context("failed to upsert standard financial statement import row")?;
    }
    tx.commit()
        .await
        .context("failed to commit standard financial statement import")?;

    let integrity = check_integrity(pool, version_id).await?;
    Ok(import_report(
        version_id,
        "IMPORTED",
        total_rows,
        rows.len(),
        inserted_count,
        updated_count,
        unchanged_count,
        issues,
        Some(integrity),
    ))
}

pub async fn check_integrity(pool: &PgPool, version_id: Uuid) -> Result<StdFsIntegrityResult> {
    get_version(pool, version_id).await?;
    let items = list_items(pool, version_id, None, true).await?;
    Ok(integrity_for_items(version_id, &items))
}

fn integrity_for_items(version_id: Uuid, items: &[StdFsItem]) -> StdFsIntegrityResult {
    let mut issues = Vec::new();
    if items.is_empty() {
        issues.push(issue(
            "ERROR",
            "VERSION_EMPTY",
            None,
            "standard financial statement version has no items",
        ));
    }

    let mut seen_codes = HashSet::new();
    let mut seen_stmt_codes = HashSet::new();
    let mut by_stmt_code: HashMap<(String, String), &StdFsItem> = HashMap::new();
    let mut children_by_stmt_parent: HashMap<(String, String), usize> = HashMap::new();
    let mut stmt_types = HashSet::new();
    for item in items {
        stmt_types.insert(item.stmt_type.clone());
        if !seen_codes.insert(item.item_code.clone()) {
            issues.push(issue(
                "ERROR",
                "ITEM_CODE_DUPLICATE",
                Some(item.item_code.clone()),
                "item_code is duplicated within version",
            ));
        }
        if !seen_stmt_codes.insert((item.stmt_type.clone(), item.item_code.clone())) {
            issues.push(issue(
                "ERROR",
                "STATEMENT_ITEM_CODE_DUPLICATE",
                Some(item.item_code.clone()),
                "item_code is duplicated within statement type",
            ));
        }
        by_stmt_code.insert((item.stmt_type.clone(), item.item_code.clone()), item);
        if let Some(parent_code) = item.parent_code.as_ref() {
            children_by_stmt_parent
                .entry((item.stmt_type.clone(), parent_code.clone()))
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
    }

    for item in items {
        if let Some(parent_code) = item.parent_code.as_ref() {
            match by_stmt_code.get(&(item.stmt_type.clone(), parent_code.clone())) {
                Some(parent) if item.level <= parent.level => issues.push(issue(
                    "ERROR",
                    "PARENT_LEVEL_INVALID",
                    Some(item.item_code.clone()),
                    "parent level must be less than child level",
                )),
                Some(_) => {}
                None => issues.push(issue(
                    "ERROR",
                    "PARENT_MISSING",
                    Some(item.item_code.clone()),
                    "parent_code does not exist in the same statement type",
                )),
            }
        }

        let child_count = children_by_stmt_parent
            .get(&(item.stmt_type.clone(), item.item_code.clone()))
            .copied()
            .unwrap_or(0);
        if item.is_subtotal {
            if let Some(formula) = item
                .agg_formula
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                for ref_code in formula_refs(formula) {
                    if ref_code == item.item_code {
                        issues.push(issue(
                            "ERROR",
                            "FORMULA_SELF_REFERENCE",
                            Some(item.item_code.clone()),
                            "subtotal formula references itself",
                        ));
                    } else if !by_stmt_code
                        .contains_key(&(item.stmt_type.clone(), ref_code.clone()))
                    {
                        issues.push(issue(
                            "ERROR",
                            "FORMULA_REF_MISSING",
                            Some(item.item_code.clone()),
                            &format!("subtotal formula references missing item_code {ref_code}"),
                        ));
                    }
                }
            } else if child_count == 0 {
                issues.push(issue(
                    "ERROR",
                    "SUBTOTAL_SOURCE_MISSING",
                    Some(item.item_code.clone()),
                    "subtotal item must have a formula or children",
                ));
            }
        } else if child_count > 0 {
            issues.push(issue(
                "WARN",
                "LEAF_HAS_CHILDREN",
                Some(item.item_code.clone()),
                "non-subtotal item has children",
            ));
        }

        if item.is_required
            && item.is_active
            && item.xml_field_id.as_deref().unwrap_or("").trim().is_empty()
        {
            issues.push(issue(
                "WARN",
                "REQUIRED_XML_FIELD_MISSING",
                Some(item.item_code.clone()),
                "required active item has no xml_field_id",
            ));
        }
    }

    for stmt_type in stmt_types {
        let has_required = items
            .iter()
            .any(|item| item.stmt_type == stmt_type && item.is_active && item.is_required);
        if !has_required {
            issues.push(issue(
                "ERROR",
                "REQUIRED_ITEM_MISSING",
                None,
                &format!("{stmt_type} has no active required item"),
            ));
        }
    }

    let error_count = issues
        .iter()
        .filter(|issue| issue.severity == "ERROR")
        .count();
    let warn_count = issues
        .iter()
        .filter(|issue| issue.severity == "WARN")
        .count();
    StdFsIntegrityResult {
        version_id,
        valid: error_count == 0,
        error_count,
        warn_count,
        issues,
    }
}

pub async fn diff_versions(
    pool: &PgPool,
    from_version_id: Uuid,
    to_version_id: Uuid,
) -> Result<StdFsVersionDiff> {
    get_version(pool, from_version_id).await?;
    get_version(pool, to_version_id).await?;
    let from_items = list_items(pool, from_version_id, None, true).await?;
    let to_items = list_items(pool, to_version_id, None, true).await?;
    let from_by_code = from_items
        .into_iter()
        .map(|item| (item.item_code.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let to_by_code = to_items
        .into_iter()
        .map(|item| (item.item_code.clone(), item))
        .collect::<BTreeMap<_, _>>();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    for (code, to_item) in &to_by_code {
        match from_by_code.get(code) {
            Some(from_item) => {
                let changed_fields = changed_fields(from_item, to_item);
                if !changed_fields.is_empty() {
                    changed.push(StdFsItemDiff {
                        item_code: code.clone(),
                        from: Some(from_item.clone()),
                        to: Some(to_item.clone()),
                        changed_fields,
                    });
                }
            }
            None => added.push(StdFsItemDiff {
                item_code: code.clone(),
                from: None,
                to: Some(to_item.clone()),
                changed_fields: Vec::new(),
            }),
        }
    }
    for (code, from_item) in &from_by_code {
        if !to_by_code.contains_key(code) {
            removed.push(StdFsItemDiff {
                item_code: code.clone(),
                from: Some(from_item.clone()),
                to: None,
                changed_fields: Vec::new(),
            });
        }
    }

    Ok(StdFsVersionDiff {
        from_version_id,
        to_version_id,
        summary: json!({
            "added_count": added.len(),
            "removed_count": removed.len(),
            "changed_count": changed.len()
        }),
        added,
        removed,
        changed,
    })
}

#[derive(Debug)]
struct ParsedImportTabular {
    rows: Vec<ImportTabularRow>,
    issues: Vec<StdFsImportIssue>,
}

fn parse_import_tabular(bytes: &[u8], file_name: Option<&str>) -> Result<ParsedImportTabular> {
    if file_name
        .unwrap_or_default()
        .to_ascii_lowercase()
        .ends_with(".xlsx")
    {
        parse_import_xlsx(bytes)
    } else {
        parse_import_csv(bytes)
    }
}

fn parse_import_csv(bytes: &[u8]) -> Result<ParsedImportTabular> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(bytes);
    let raw_headers = reader
        .headers()
        .context("failed to read standard financial statement import headers")?
        .iter()
        .map(canonical_import_header)
        .collect::<Vec<_>>();
    let issues = validate_import_headers(&raw_headers);
    let rows = reader
        .records()
        .enumerate()
        .map(|(index, record)| {
            let record =
                record.context("failed to read standard financial statement import record")?;
            let values = raw_headers
                .iter()
                .enumerate()
                .map(|(header_index, header)| {
                    (
                        header.clone(),
                        record
                            .get(header_index)
                            .unwrap_or_default()
                            .trim()
                            .to_string(),
                    )
                })
                .collect::<HashMap<_, _>>();
            Ok((index, values))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|(_, values)| values.values().any(|value| !value.is_empty()))
        .map(|(index, values)| ImportTabularRow {
            row_no: index as i32 + 2,
            values,
        })
        .collect::<Vec<_>>();
    Ok(ParsedImportTabular { rows, issues })
}

fn parse_import_xlsx(bytes: &[u8]) -> Result<ParsedImportTabular> {
    let cursor = Cursor::new(bytes.to_vec());
    let mut workbook =
        Xlsx::new(cursor).context("failed to open standard financial statement xlsx workbook")?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| anyhow!("xlsx workbook has no worksheet"))?
        .context("failed to read standard financial statement xlsx worksheet")?;
    let mut rows = range.rows();
    let raw_headers = rows
        .next()
        .ok_or_else(|| anyhow!("xlsx worksheet has no header row"))?
        .iter()
        .map(|cell| canonical_import_header(&cell.to_string()))
        .collect::<Vec<_>>();
    let issues = validate_import_headers(&raw_headers);
    let parsed_rows = rows
        .enumerate()
        .map(|(index, row)| {
            let values = raw_headers
                .iter()
                .enumerate()
                .map(|(header_index, header)| {
                    (
                        header.clone(),
                        row.get(header_index)
                            .map(|cell| cell.to_string().trim().to_string())
                            .unwrap_or_default(),
                    )
                })
                .collect::<HashMap<_, _>>();
            (index, values)
        })
        .filter(|(_, values)| values.values().any(|value| !value.is_empty()))
        .map(|(index, values)| ImportTabularRow {
            row_no: index as i32 + 2,
            values,
        })
        .collect::<Vec<_>>();
    Ok(ParsedImportTabular {
        rows: parsed_rows,
        issues,
    })
}

fn parse_import_item_rows(
    rows: &[ImportTabularRow],
) -> (Vec<ImportItemRow>, Vec<StdFsImportIssue>) {
    let mut parsed = Vec::new();
    let mut issues = Vec::new();
    for row in rows {
        let raw_row = json!(row.values);
        let stmt_type = match row.required("stmt_type") {
            Some(value) => match normalize_stmt_type(&value) {
                Ok(value) => value,
                Err(_) => {
                    issues.push(row.issue(
                        "ERROR",
                        "INVALID_STMT_TYPE",
                        "stmt_type",
                        None,
                        "stmt_type must be one of STD_BS, STD_IS, STD_COST, STD_RE",
                    ));
                    continue;
                }
            },
            None => {
                issues.push(row.issue(
                    "ERROR",
                    "REQUIRED_FIELD_MISSING",
                    "stmt_type",
                    None,
                    "stmt_type is required",
                ));
                continue;
            }
        };
        let item_code = match row.required("item_code") {
            Some(value) => match normalize_required_upper(&value, "item_code") {
                Ok(value) => value,
                Err(_) => {
                    issues.push(row.issue(
                        "ERROR",
                        "REQUIRED_FIELD_MISSING",
                        "item_code",
                        None,
                        "item_code is required",
                    ));
                    continue;
                }
            },
            None => {
                issues.push(row.issue(
                    "ERROR",
                    "REQUIRED_FIELD_MISSING",
                    "item_code",
                    None,
                    "item_code is required",
                ));
                continue;
            }
        };
        let item_name = match row.required("item_name") {
            Some(value) => match normalize_required(&value, "item_name") {
                Ok(value) => value,
                Err(_) => {
                    issues.push(row.issue(
                        "ERROR",
                        "REQUIRED_FIELD_MISSING",
                        "item_name",
                        Some(item_code),
                        "item_name is required",
                    ));
                    continue;
                }
            },
            None => {
                issues.push(row.issue(
                    "ERROR",
                    "REQUIRED_FIELD_MISSING",
                    "item_name",
                    Some(item_code),
                    "item_name is required",
                ));
                continue;
            }
        };
        let level = match row
            .optional("level")
            .unwrap_or_else(|| "1".to_string())
            .parse::<i32>()
        {
            Ok(level) if level > 0 => level,
            _ => {
                issues.push(row.issue(
                    "ERROR",
                    "INVALID_LEVEL",
                    "level",
                    Some(item_code),
                    "level must be a positive integer",
                ));
                continue;
            }
        };
        let parent_code = match row
            .optional("parent_code")
            .map(|value| normalize_optional_upper(&value, "parent_code"))
            .transpose()
        {
            Ok(value) => value.flatten(),
            Err(_) => {
                issues.push(row.issue(
                    "ERROR",
                    "INVALID_PARENT_CODE",
                    "parent_code",
                    Some(item_code),
                    "parent_code is invalid",
                ));
                continue;
            }
        };
        let account_class = match row
            .optional("account_class")
            .map(|value| normalize_optional_upper(&value, "account_class"))
            .transpose()
        {
            Ok(value) => value.flatten(),
            Err(_) => {
                issues.push(row.issue(
                    "ERROR",
                    "INVALID_ACCOUNT_CLASS",
                    "account_class",
                    Some(item_code),
                    "account_class is invalid",
                ));
                continue;
            }
        };
        let normal_balance = match row
            .optional("normal_balance")
            .map(|value| normalize_normal_balance(&value))
            .transpose()
        {
            Ok(value) => value,
            Err(_) => {
                issues.push(row.issue(
                    "ERROR",
                    "INVALID_NORMAL_BALANCE",
                    "normal_balance",
                    Some(item_code),
                    "normal_balance must be DEBIT or CREDIT",
                ));
                continue;
            }
        };
        let is_subtotal = match parse_import_bool(row.optional("is_subtotal").as_deref(), false) {
            Ok(value) => value,
            Err(message) => {
                issues.push(row.issue(
                    "ERROR",
                    "INVALID_BOOLEAN",
                    "is_subtotal",
                    Some(item_code),
                    message.to_string(),
                ));
                continue;
            }
        };
        let is_required = match parse_import_bool(row.optional("is_required").as_deref(), false) {
            Ok(value) => value,
            Err(message) => {
                issues.push(row.issue(
                    "ERROR",
                    "INVALID_BOOLEAN",
                    "is_required",
                    Some(item_code),
                    message.to_string(),
                ));
                continue;
            }
        };
        let is_active = match parse_import_bool(row.optional("is_active").as_deref(), true) {
            Ok(value) => value,
            Err(message) => {
                issues.push(row.issue(
                    "ERROR",
                    "INVALID_BOOLEAN",
                    "is_active",
                    Some(item_code),
                    message.to_string(),
                ));
                continue;
            }
        };
        let sort_order = match row.optional("sort_order") {
            Some(value) => match value.parse::<i32>() {
                Ok(value) => Some(value),
                Err(_) => {
                    issues.push(row.issue(
                        "ERROR",
                        "INVALID_SORT_ORDER",
                        "sort_order",
                        Some(item_code),
                        "sort_order must be an integer",
                    ));
                    continue;
                }
            },
            None => None,
        };
        parsed.push(ImportItemRow {
            row_no: row.row_no,
            stmt_type,
            item_code,
            item_name,
            parent_code,
            level,
            account_class,
            normal_balance,
            is_subtotal,
            is_required,
            agg_formula: trim_optional(row.optional("agg_formula")),
            xml_field_id: trim_optional(row.optional("xml_field_id")),
            sort_order,
            is_active,
            raw_row,
        });
    }
    (parsed, issues)
}

fn validate_import_headers(headers: &[String]) -> Vec<StdFsImportIssue> {
    let mut issues = Vec::new();
    let mut seen = HashSet::new();
    for header in headers {
        if !seen.insert(header.clone()) {
            issues.push(import_issue(
                1,
                "ERROR",
                "HEADER_DUPLICATE",
                Some(header.as_str()),
                None,
                &format!("duplicate import header: {header}"),
                json!({ "header": header }),
            ));
        }
        if !IMPORT_HEADERS.contains(&header.as_str()) {
            issues.push(import_issue(
                1,
                "ERROR",
                "HEADER_UNKNOWN",
                Some(header.as_str()),
                None,
                &format!("unknown import header: {header}"),
                json!({ "header": header }),
            ));
        }
    }
    for &required in IMPORT_HEADERS {
        if !seen.contains(required) {
            issues.push(import_issue(
                1,
                "ERROR",
                "HEADER_MISSING",
                Some(required),
                None,
                &format!("missing required import header: {required}"),
                json!({ "header": required }),
            ));
        }
    }
    issues
}

fn validate_import_preconditions(
    version_id: Uuid,
    existing: &[StdFsItem],
    rows: &[ImportItemRow],
) -> Vec<StdFsImportIssue> {
    let mut issues = Vec::new();
    let mut seen_item_code: HashMap<String, i32> = HashMap::new();
    let mut seen_stmt_code: HashMap<(String, String), i32> = HashMap::new();
    let existing_by_code = existing
        .iter()
        .map(|item| (item.item_code.clone(), item))
        .collect::<HashMap<_, _>>();
    for row in rows {
        if let Some(first_row) = seen_item_code.insert(row.item_code.clone(), row.row_no) {
            issues.push(import_issue(
                row.row_no,
                "ERROR",
                "ITEM_CODE_DUPLICATE",
                Some("item_code"),
                Some(row.item_code.clone()),
                &format!(
                    "item_code {} is duplicated in import rows {} and {}",
                    row.item_code, first_row, row.row_no
                ),
                row.raw_row.clone(),
            ));
        }
        let stmt_key = (row.stmt_type.clone(), row.item_code.clone());
        if let Some(first_row) = seen_stmt_code.insert(stmt_key, row.row_no) {
            issues.push(import_issue(
                row.row_no,
                "ERROR",
                "STATEMENT_ITEM_CODE_DUPLICATE",
                Some("item_code"),
                Some(row.item_code.clone()),
                &format!(
                    "stmt_type+item_code {}:{} is duplicated in import rows {} and {}",
                    row.stmt_type, row.item_code, first_row, row.row_no
                ),
                row.raw_row.clone(),
            ));
        }
        if let Some(existing) = existing_by_code.get(&row.item_code) {
            if existing.stmt_type != row.stmt_type {
                issues.push(import_issue(
                    row.row_no,
                    "ERROR",
                    "ITEM_CODE_STATEMENT_CONFLICT",
                    Some("item_code"),
                    Some(row.item_code.clone()),
                    &format!(
                        "item_code {} already exists in {} for version {}",
                        row.item_code, existing.stmt_type, version_id
                    ),
                    row.raw_row.clone(),
                ));
            }
        }
    }

    let final_items = final_items_after_import(version_id, existing, rows);
    let by_stmt_code = final_items
        .iter()
        .map(|item| ((item.stmt_type.clone(), item.item_code.clone()), item))
        .collect::<HashMap<_, _>>();
    let mut children_by_stmt_parent: HashMap<(String, String), Vec<&StdFsItem>> = HashMap::new();
    for item in &final_items {
        if let Some(parent_code) = item.parent_code.as_ref() {
            children_by_stmt_parent
                .entry((item.stmt_type.clone(), parent_code.clone()))
                .or_default()
                .push(item);
        }
    }
    let imported_row_by_code = rows
        .iter()
        .map(|row| (row.item_code.clone(), row))
        .collect::<HashMap<_, _>>();
    for ((stmt_type, parent_code), children) in children_by_stmt_parent {
        if let Some(parent) = by_stmt_code.get(&(stmt_type, parent_code.clone())) {
            if !parent.is_subtotal {
                let row = imported_row_by_code.get(&parent.item_code);
                issues.push(import_issue(
                    row.map(|row| row.row_no).unwrap_or(0),
                    "ERROR",
                    "PARENT_NOT_SUBTOTAL",
                    Some("is_subtotal"),
                    Some(parent.item_code.clone()),
                    "an item with children must be marked as subtotal",
                    row.map(|row| row.raw_row.clone())
                        .unwrap_or_else(|| json!({})),
                ));
            }
        }
        for child in children {
            if child.parent_code.as_deref() == Some(&child.item_code) {
                let row = imported_row_by_code.get(&child.item_code);
                issues.push(import_issue(
                    row.map(|row| row.row_no).unwrap_or(0),
                    "ERROR",
                    "PARENT_SELF_REFERENCE",
                    Some("parent_code"),
                    Some(child.item_code.clone()),
                    "parent_code cannot reference the same item_code",
                    row.map(|row| row.raw_row.clone())
                        .unwrap_or_else(|| json!({})),
                ));
            }
        }
    }
    issues
}

fn final_items_after_import(
    version_id: Uuid,
    existing: &[StdFsItem],
    rows: &[ImportItemRow],
) -> Vec<StdFsItem> {
    let mut final_items = existing
        .iter()
        .map(|item| {
            (
                (item.stmt_type.clone(), item.item_code.clone()),
                item.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for row in rows {
        let key = (row.stmt_type.clone(), row.item_code.clone());
        let item_id = final_items
            .get(&key)
            .map(|item| item.id)
            .unwrap_or_else(Uuid::nil);
        final_items.insert(
            key,
            StdFsItem {
                id: item_id,
                version_id,
                stmt_type: row.stmt_type.clone(),
                item_code: row.item_code.clone(),
                item_name: row.item_name.clone(),
                parent_code: row.parent_code.clone(),
                level: row.level,
                account_class: row.account_class.clone(),
                normal_balance: row.normal_balance.clone(),
                is_subtotal: row.is_subtotal,
                is_required: row.is_required,
                agg_formula: row.agg_formula.clone(),
                xml_field_id: row.xml_field_id.clone(),
                sort_order: row.sort_order,
                is_active: row.is_active,
            },
        );
    }
    final_items.into_values().collect()
}

fn import_row_matches_item(row: &ImportItemRow, item: &StdFsItem) -> bool {
    row.item_name == item.item_name
        && row.parent_code == item.parent_code
        && row.level == item.level
        && row.account_class == item.account_class
        && row.normal_balance == item.normal_balance
        && row.is_subtotal == item.is_subtotal
        && row.is_required == item.is_required
        && row.agg_formula == item.agg_formula
        && row.xml_field_id == item.xml_field_id
        && row.sort_order == item.sort_order
        && row.is_active == item.is_active
}

fn import_report(
    version_id: Uuid,
    status: &str,
    total_rows: usize,
    valid_rows: usize,
    inserted_count: usize,
    updated_count: usize,
    unchanged_count: usize,
    issues: Vec<StdFsImportIssue>,
    integrity: Option<StdFsIntegrityResult>,
) -> StdFsImportReport {
    let error_count = issues
        .iter()
        .filter(|issue| issue.severity == "ERROR")
        .count();
    let warn_count = issues
        .iter()
        .filter(|issue| issue.severity == "WARN")
        .count();
    StdFsImportReport {
        version_id,
        status: status.to_string(),
        total_rows,
        valid_rows,
        inserted_count,
        updated_count,
        unchanged_count,
        error_count,
        warn_count,
        issues,
        integrity,
    }
}

fn valid_row_count(total_rows: usize, issues: &[StdFsImportIssue]) -> usize {
    let invalid_rows = issues
        .iter()
        .filter(|issue| issue.row_no > 1 && issue.severity == "ERROR")
        .map(|issue| issue.row_no)
        .collect::<HashSet<_>>()
        .len();
    total_rows.saturating_sub(invalid_rows)
}

fn import_issue(
    row_no: i32,
    severity: &str,
    code: &str,
    field_name: Option<&str>,
    item_code: Option<String>,
    message: &str,
    raw_row: Value,
) -> StdFsImportIssue {
    StdFsImportIssue {
        row_no,
        severity: severity.to_string(),
        code: code.to_string(),
        field_name: field_name.map(ToString::to_string),
        item_code,
        message: message.to_string(),
        raw_row,
    }
}

fn canonical_import_header(header: &str) -> String {
    let normalized = header
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '.'], "_");
    match normalized.as_str() {
        "statement_type" | "statement" | "fs_type" | "재무제표구분" | "표준재무제표구분" => {
            "stmt_type"
        }
        "code" | "item_cd" | "항목코드" | "표준계정과목코드" => "item_code",
        "name" | "item_nm" | "항목명" | "계정과목명" => "item_name",
        "parent" | "parent_item_code" | "상위코드" | "상위항목코드" => "parent_code",
        "lv" | "depth" | "레벨" | "수준" => "level",
        "class" | "account_cls" | "계정분류" | "분류" => "account_class",
        "balance" | "normal_bal" | "차대구분" | "정상잔액" => "normal_balance",
        "subtotal" | "is_total" | "합계여부" | "합계행여부" => "is_subtotal",
        "required" | "필수여부" | "필수항목여부" => "is_required",
        "formula" | "aggregation_formula" | "산식" | "합계산식" => "agg_formula",
        "xml_field" | "xml_id" | "xml필드id" | "xml필드" => "xml_field_id",
        "sort" | "display_order" | "정렬순서" | "표시순서" => "sort_order",
        "active" | "enabled" | "사용여부" | "활성여부" => "is_active",
        value => value,
    }
    .to_string()
}

fn parse_import_bool(value: Option<&str>, default_value: bool) -> Result<bool> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(default_value);
    };
    match value.to_ascii_uppercase().as_str() {
        "TRUE" | "T" | "Y" | "YES" | "1" | "O" | "예" | "사용" | "활성" => Ok(true),
        "FALSE" | "F" | "N" | "NO" | "0" | "X" | "아니오" | "미사용" | "비활성" => {
            Ok(false)
        }
        _ => Err(anyhow!("boolean value must be true/false, y/n, or 1/0")),
    }
}

impl ImportTabularRow {
    fn optional(&self, name: &str) -> Option<String> {
        self.values.get(name).and_then(|value| {
            let value = value.trim();
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
    }

    fn required(&self, name: &str) -> Option<String> {
        self.optional(name)
    }

    fn issue(
        &self,
        severity: &str,
        code: &str,
        field_name: &str,
        item_code: Option<String>,
        message: impl Into<String>,
    ) -> StdFsImportIssue {
        import_issue(
            self.row_no,
            severity,
            code,
            Some(field_name),
            item_code,
            &message.into(),
            json!(self.values),
        )
    }
}

async fn ensure_version_editable(pool: &PgPool, version_id: Uuid) -> Result<()> {
    let version = get_version(pool, version_id).await?;
    if version.status != "DRAFT" {
        return Err(anyhow!(
            "version status blocked editing: only DRAFT versions can be edited"
        ));
    }
    Ok(())
}

fn validate_status_transition(current: &str, next: &str) -> Result<()> {
    if current == next {
        return Ok(());
    }
    let allowed = matches!(
        (current, next),
        ("DRAFT", "REVIEWED")
            | ("DRAFT", "RETIRED")
            | ("REVIEWED", "DRAFT")
            | ("REVIEWED", "ACTIVE")
            | ("REVIEWED", "RETIRED")
            | ("ACTIVE", "RETIRED")
    );
    if allowed {
        Ok(())
    } else {
        Err(anyhow!(
            "invalid status transition: {current} -> {next} is not allowed"
        ))
    }
}

fn changed_fields(from: &StdFsItem, to: &StdFsItem) -> Vec<String> {
    let mut fields = Vec::new();
    if from.stmt_type != to.stmt_type {
        fields.push("stmt_type".to_string());
    }
    if from.item_name != to.item_name {
        fields.push("item_name".to_string());
    }
    if from.parent_code != to.parent_code {
        fields.push("parent_code".to_string());
    }
    if from.level != to.level {
        fields.push("level".to_string());
    }
    if from.account_class != to.account_class {
        fields.push("account_class".to_string());
    }
    if from.normal_balance != to.normal_balance {
        fields.push("normal_balance".to_string());
    }
    if from.is_subtotal != to.is_subtotal {
        fields.push("is_subtotal".to_string());
    }
    if from.is_required != to.is_required {
        fields.push("is_required".to_string());
    }
    if from.agg_formula != to.agg_formula {
        fields.push("agg_formula".to_string());
    }
    if from.xml_field_id != to.xml_field_id {
        fields.push("xml_field_id".to_string());
    }
    if from.sort_order != to.sort_order {
        fields.push("sort_order".to_string());
    }
    if from.is_active != to.is_active {
        fields.push("is_active".to_string());
    }
    fields
}

fn issue(
    severity: &str,
    code: &str,
    item_code: Option<String>,
    message: &str,
) -> StdFsIntegrityIssue {
    StdFsIntegrityIssue {
        severity: severity.to_string(),
        code: code.to_string(),
        item_code,
        message: message.to_string(),
    }
}

fn formula_refs(formula: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut current = String::new();
    for ch in formula.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch.to_ascii_uppercase());
        } else if !current.is_empty() {
            refs.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        refs.push(current);
    }
    refs.sort();
    refs.dedup();
    refs
}

fn validate_effective_range(
    effective_from: chrono::NaiveDate,
    effective_to: Option<chrono::NaiveDate>,
) -> Result<()> {
    if effective_to.is_some_and(|date| effective_from > date) {
        return Err(anyhow!("invalid effective date range"));
    }
    Ok(())
}

fn normalize_version_status(status: &str) -> Result<String> {
    normalize_choice(status, ALLOWED_VERSION_STATUSES, "status")
}

fn normalize_stmt_type(stmt_type: &str) -> Result<String> {
    normalize_choice(stmt_type, ALLOWED_STMT_TYPES, "stmt_type")
}

fn normalize_normal_balance(value: &str) -> Result<String> {
    normalize_choice(value, ALLOWED_NORMAL_BALANCES, "normal_balance")
}

fn normalize_nullable_normal_balance(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        Ok(String::new())
    } else {
        normalize_normal_balance(value)
    }
}

fn normalize_choice(value: &str, allowed: &[&str], field: &str) -> Result<String> {
    let normalized = normalize_required_upper(value, field)?;
    if !allowed.contains(&normalized.as_str()) {
        return Err(anyhow!("invalid {field}"));
    }
    Ok(normalized)
}

fn normalize_required(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("invalid {field}"));
    }
    Ok(value.to_string())
}

fn normalize_required_upper(value: &str, field: &str) -> Result<String> {
    Ok(normalize_required(value, field)?.to_ascii_uppercase())
}

fn normalize_optional_upper(value: &str, field: &str) -> Result<Option<String>> {
    let value = value.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(normalize_required_upper(value, field)?))
    }
}

fn normalize_nullable_upper(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        Ok(String::new())
    } else {
        normalize_required_upper(value, field)
    }
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn trim_nullable(value: Option<String>) -> Option<String> {
    value.map(|value| value.trim().to_string())
}
