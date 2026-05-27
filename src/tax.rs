use std::io::{Cursor, Write};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use crate::{
    db::quote_ident,
    domain::{
        AdjustmentItem, AssetBasedAdjustmentRequest, AssetBasedAdjustmentResult,
        CalculateAdjustmentRequest, CalculationResult, CapitalChange, CapitalChangeInput,
        ConsolidatedEntityInput, ConsolidationEliminationInput, CreateAdjustmentAttachmentRequest,
        CreateIncomeAdjustmentRequest, CreateLawAmendmentRequest, CreateTaxLawRequest,
        CreateTaxLimitRequest, CreateTaxRateRequest, CreateVehicleUsageLogRequest,
        DonationCarryforward, EvaluationAdjustmentRequest, EvaluationAdjustmentResult,
        ForeignIncomeInput, FormAttachmentSummary, FormData, FormDataHistory, FormOutputFile,
        FormPreviewField, FormPreviewResult, FormValidationIssue, IncomeAdjustmentItemInput,
        IncomeAdjustmentResult, LawAmendmentHistory, LawSnapshot, LawVersioningImpactRequest,
        LossCarryforwardInput, LossCarryforwardRecord, PenaltyTaxInput, ReserveRecord,
        RevenueBreakdownInput, SpecialTaxAdjustmentRequest, SpecialTaxAdjustmentResult,
        TaxAdjustment, TaxAmountAdjustmentRequest, TaxAmountAdjustmentResult, TaxCreditInput,
        TaxLawVersion, TaxLimit, TaxRate, TenantRef, TransactionBasedAdjustmentRequest,
        TransactionBasedAdjustmentResult, UpdateFormDataRequest, UpdateTaxLawStatusRequest,
        ValuationPositionInput, VehicleUsageLog,
    },
    tenant,
};

pub async fn create_tax_law(pool: &PgPool, request: CreateTaxLawRequest) -> Result<TaxLawVersion> {
    sqlx::query_as::<_, TaxLawVersion>(
        r#"
        INSERT INTO tax_law_versions (
            version_code, law_name, effective_from, effective_to, status, metadata
        )
        VALUES ($1, $2, $3, $4, 'DRAFT', COALESCE($5, '{}'::jsonb))
        RETURNING law_version_id, version_code, law_name, effective_from, effective_to,
                  status, metadata, created_at
        "#,
    )
    .bind(request.version_code)
    .bind(request.law_name)
    .bind(request.effective_from)
    .bind(request.effective_to)
    .bind(request.metadata)
    .fetch_one(pool)
    .await
    .context("failed to create tax law version")
}

pub async fn list_tax_laws(pool: &PgPool) -> Result<Vec<TaxLawVersion>> {
    sqlx::query_as::<_, TaxLawVersion>(
        r#"
        SELECT law_version_id, version_code, law_name, effective_from, effective_to,
               status, metadata, created_at
        FROM tax_law_versions
        ORDER BY effective_from DESC, version_code
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list tax law versions")
}

pub async fn create_tax_rate(pool: &PgPool, request: CreateTaxRateRequest) -> Result<TaxRate> {
    sqlx::query_as::<_, TaxRate>(
        r#"
        INSERT INTO tax_rates (
            law_version_id, item_code, taxable_from, taxable_to, base_tax,
            rate_bps, progressive_deduction, effective_from, effective_to, metadata
        )
        VALUES ($1, $2, $3, $4, COALESCE($5, 0), $6, COALESCE($7, 0), $8, $9, COALESCE($10, '{}'::jsonb))
        RETURNING tax_rate_id, law_version_id, item_code, taxable_from, taxable_to,
                  base_tax, rate_bps, progressive_deduction, effective_from, effective_to, metadata
        "#,
    )
    .bind(request.law_version_id)
    .bind(request.item_code)
    .bind(request.taxable_from)
    .bind(request.taxable_to)
    .bind(request.base_tax)
    .bind(request.rate_bps)
    .bind(request.progressive_deduction)
    .bind(request.effective_from)
    .bind(request.effective_to)
    .bind(request.metadata)
    .fetch_one(pool)
    .await
        .context("failed to create tax rate")
}

pub async fn list_tax_rates(pool: &PgPool, law_version_id: Option<i64>) -> Result<Vec<TaxRate>> {
    sqlx::query_as::<_, TaxRate>(
        r#"
        SELECT tax_rate_id, law_version_id, item_code, taxable_from, taxable_to,
               base_tax, rate_bps, progressive_deduction, effective_from, effective_to, metadata
        FROM tax_rates
        WHERE ($1::BIGINT IS NULL OR law_version_id = $1)
        ORDER BY law_version_id DESC, item_code, taxable_from
        "#,
    )
    .bind(law_version_id)
    .fetch_all(pool)
    .await
    .context("failed to list tax rates")
}

pub async fn create_tax_limit(pool: &PgPool, request: CreateTaxLimitRequest) -> Result<TaxLimit> {
    if request.item_code.trim().is_empty() {
        return Err(anyhow!("invalid tax limit item code"));
    }
    if request.amount < 0 {
        return Err(anyhow!("invalid tax limit amount"));
    }

    sqlx::query_as::<_, TaxLimit>(
        r#"
        INSERT INTO tax_limits (
            law_version_id, item_code, amount, effective_from, effective_to, metadata
        )
        VALUES ($1, $2, $3, $4, $5, COALESCE($6, '{}'::jsonb))
        RETURNING tax_limit_id, law_version_id, item_code, amount,
                  effective_from, effective_to, metadata
        "#,
    )
    .bind(request.law_version_id)
    .bind(request.item_code.trim().to_ascii_uppercase())
    .bind(request.amount)
    .bind(request.effective_from)
    .bind(request.effective_to)
    .bind(request.metadata)
    .fetch_one(pool)
    .await
    .context("failed to create tax limit")
}

pub async fn list_tax_limits(
    pool: &PgPool,
    law_version_id: Option<i64>,
    category: Option<&str>,
) -> Result<Vec<TaxLimit>> {
    sqlx::query_as::<_, TaxLimit>(
        r#"
        SELECT tax_limit_id, law_version_id, item_code, amount,
               effective_from, effective_to, metadata
        FROM tax_limits
        WHERE ($1::BIGINT IS NULL OR law_version_id = $1)
          AND (
              $2::TEXT IS NULL
              OR item_code = UPPER($2)
              OR metadata->>'category' = $2
              OR metadata->>'group' = $2
          )
        ORDER BY law_version_id DESC, item_code, effective_from DESC, tax_limit_id DESC
        "#,
    )
    .bind(law_version_id)
    .bind(category)
    .fetch_all(pool)
    .await
    .context("failed to list tax limits")
}

pub async fn update_tax_law_status(
    pool: &PgPool,
    law_version_id: i64,
    request: UpdateTaxLawStatusRequest,
) -> Result<TaxLawVersion> {
    let status = request.status.trim().to_ascii_uppercase();
    let allowed = ["DRAFT", "REVIEWED", "APPROVED", "ACTIVE", "RETIRED"];
    if !allowed.contains(&status.as_str()) {
        return Err(anyhow!("invalid tax law status"));
    }
    let current_status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM tax_law_versions WHERE law_version_id = $1",
    )
    .bind(law_version_id)
    .fetch_one(pool)
    .await
    .context("tax law version not found")?;
    validate_law_status_transition(&current_status, &status)?;

    let law = sqlx::query_as::<_, TaxLawVersion>(
        r#"
        UPDATE tax_law_versions
        SET status = $1
        WHERE law_version_id = $2
        RETURNING law_version_id, version_code, law_name, effective_from, effective_to,
                  status, metadata, created_at
        "#,
    )
    .bind(&status)
    .bind(law_version_id)
    .fetch_one(pool)
    .await
    .context("failed to update tax law status")?;

    if let Some(summary) = request
        .change_summary
        .filter(|value| !value.trim().is_empty())
    {
        create_law_amendment(
            pool,
            CreateLawAmendmentRequest {
                law_version_id,
                change_summary: summary,
                approved_by: request.approved_by,
            },
        )
        .await?;
    }

    Ok(law)
}

fn validate_law_status_transition(current: &str, next: &str) -> Result<()> {
    if current == next {
        return Ok(());
    }
    let allowed = match current {
        "DRAFT" => matches!(next, "REVIEWED"),
        "REVIEWED" => matches!(next, "ACTIVE" | "DRAFT"),
        "APPROVED" => matches!(next, "ACTIVE" | "RETIRED"),
        "ACTIVE" => matches!(next, "RETIRED"),
        "RETIRED" => false,
        _ => false,
    };
    if allowed {
        Ok(())
    } else {
        Err(anyhow!(
            "invalid tax law status transition: {current} -> {next}"
        ))
    }
}

pub async fn create_law_amendment(
    pool: &PgPool,
    request: CreateLawAmendmentRequest,
) -> Result<LawAmendmentHistory> {
    if request.change_summary.trim().is_empty() {
        return Err(anyhow!("invalid amendment summary"));
    }

    sqlx::query_as::<_, LawAmendmentHistory>(
        r#"
        INSERT INTO law_amendment_history (law_version_id, change_summary, approved_by)
        VALUES ($1, $2, COALESCE($3, 'system'))
        RETURNING amendment_id, law_version_id, change_summary, approved_by, approved_at
        "#,
    )
    .bind(request.law_version_id)
    .bind(request.change_summary.trim())
    .bind(request.approved_by)
    .fetch_one(pool)
    .await
    .context("failed to create law amendment history")
}

pub async fn list_law_amendments(
    pool: &PgPool,
    law_version_id: Option<i64>,
) -> Result<Vec<LawAmendmentHistory>> {
    sqlx::query_as::<_, LawAmendmentHistory>(
        r#"
        SELECT amendment_id, law_version_id, change_summary, approved_by, approved_at
        FROM law_amendment_history
        WHERE ($1::BIGINT IS NULL OR law_version_id = $1)
        ORDER BY approved_at DESC, amendment_id DESC
        "#,
    )
    .bind(law_version_id)
    .fetch_all(pool)
    .await
    .context("failed to list law amendment history")
}

pub async fn law_versioning_summary(pool: &PgPool) -> Result<Value> {
    let laws = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tax_law_versions")
        .fetch_one(pool)
        .await?;
    let rates = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tax_rates")
        .fetch_one(pool)
        .await?;
    let limits = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tax_limits")
        .fetch_one(pool)
        .await?;
    let amendments = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM law_amendment_history")
        .fetch_one(pool)
        .await?;
    let latest_law = sqlx::query_as::<_, TaxLawVersion>(
        r#"
        SELECT law_version_id, version_code, law_name, effective_from, effective_to,
               status, metadata, created_at
        FROM tax_law_versions
        ORDER BY effective_from DESC, law_version_id DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?;
    let status_counts = sqlx::query(
        r#"
        SELECT status, COUNT(*) AS count
        FROM tax_law_versions
        GROUP BY status
        ORDER BY status
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        json!({
            "status": row.get::<String, _>("status"),
            "count": row.get::<i64, _>("count")
        })
    })
    .collect::<Vec<_>>();

    Ok(json!({
        "laws": laws,
        "rates": rates,
        "limits": limits,
        "amendments": amendments,
        "latest_law": latest_law,
        "status_counts": status_counts
    }))
}

pub async fn simulate_law_impact(
    pool: &PgPool,
    request: LawVersioningImpactRequest,
) -> Result<Value> {
    let law = sqlx::query_as::<_, TaxLawVersion>(
        r#"
        SELECT law_version_id, version_code, law_name, effective_from, effective_to,
               status, metadata, created_at
        FROM tax_law_versions
        WHERE law_version_id = $1
        "#,
    )
    .bind(request.law_version_id)
    .fetch_one(pool)
    .await
    .context("failed to load tax law version for impact simulation")?;

    let rate_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tax_rates WHERE law_version_id = $1")
            .bind(law.law_version_id)
            .fetch_one(pool)
            .await?;
    let limit_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tax_limits WHERE law_version_id = $1")
            .bind(law.law_version_id)
            .fetch_one(pool)
            .await?;

    let tenants = sqlx::query(
        r#"
        SELECT tenant_code, schema_name
        FROM tenants
        WHERE status = 'ACTIVE'
        ORDER BY tenant_code
        "#,
    )
    .fetch_all(pool)
    .await?;

    let include_locked = request.include_locked.unwrap_or(false);
    let mut total_business_years = 0_i64;
    let mut locked_snapshots = 0_i64;
    let mut tenant_impacts = Vec::new();

    for tenant_row in tenants {
        let tenant_code = tenant_row.get::<String, _>("tenant_code");
        let schema_name = tenant_row.get::<String, _>("schema_name");
        let table_ref = format!("{schema_name}.business_years");
        let has_business_years =
            sqlx::query_scalar::<_, Option<String>>("SELECT to_regclass($1)::TEXT")
                .bind(&table_ref)
                .fetch_one(pool)
                .await?
                .is_some();
        if !has_business_years {
            tenant_impacts.push(json!({
                "tenant_code": tenant_code,
                "schema_name": schema_name,
                "business_years": 0,
                "locked_snapshots": 0,
                "schema_ready": false
            }));
            continue;
        }
        let schema = quote_ident(&schema_name)?;
        let lock_filter = if include_locked {
            ""
        } else {
            "AND COALESCE(s.locked, FALSE) = FALSE AND b.locked_at IS NULL"
        };
        let sql = format!(
            r#"
            SELECT
                COUNT(*) AS business_years,
                COUNT(*) FILTER (WHERE COALESCE(s.locked, FALSE) = TRUE OR b.locked_at IS NOT NULL) AS locked_snapshots
            FROM {schema}.business_years b
            LEFT JOIN {schema}.by_law_snapshot s ON s.by_id = b.by_id
            WHERE b.end_date >= $1
              AND ($2::DATE IS NULL OR b.start_date <= $2)
              {lock_filter}
            "#
        );
        let row = sqlx::query(&sql)
            .bind(law.effective_from)
            .bind(law.effective_to)
            .fetch_one(pool)
            .await?;
        let business_years = row.get::<i64, _>("business_years");
        let tenant_locked = row.get::<i64, _>("locked_snapshots");
        total_business_years += business_years;
        locked_snapshots += tenant_locked;
        tenant_impacts.push(json!({
            "tenant_code": tenant_code,
            "schema_name": schema_name,
            "business_years": business_years,
            "locked_snapshots": tenant_locked,
            "schema_ready": true
        }));
    }

    Ok(json!({
        "law": law,
        "include_locked": include_locked,
        "summary": {
            "business_years": total_business_years,
            "locked_snapshots": locked_snapshots,
            "rate_rows": rate_count,
            "limit_rows": limit_count,
            "estimated_recalculation_targets": total_business_years
        },
        "tenant_impacts": tenant_impacts
    }))
}

pub async fn ensure_law_snapshot(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<LawSnapshot> {
    if let Ok(snapshot) = get_law_snapshot(pool, tenant, by_id).await {
        return Ok(snapshot);
    }

    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let law = sqlx::query_as::<_, TaxLawVersion>(
        r#"
        SELECT law_version_id, version_code, law_name, effective_from, effective_to,
               status, metadata, created_at
        FROM tax_law_versions
        WHERE status IN ('APPROVED', 'ACTIVE')
          AND effective_from <= $1
          AND (effective_to IS NULL OR effective_to >= $2)
        ORDER BY
          EXISTS (
              SELECT 1
              FROM tax_rates r
              WHERE r.law_version_id = tax_law_versions.law_version_id
                AND r.item_code = 'CORPORATE_TAX'
          ) DESC,
          effective_from DESC,
          law_version_id DESC
        LIMIT 1
        "#,
    )
    .bind(by.end_date)
    .bind(by.start_date)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("no approved tax law version applies to business year {by_id}"))?;

    let rate_ids = sqlx::query(
        r#"
        SELECT tax_rate_id
        FROM tax_rates
        WHERE law_version_id = $1
          AND effective_from <= $2
          AND (effective_to IS NULL OR effective_to >= $3)
        ORDER BY item_code, taxable_from
        "#,
    )
    .bind(law.law_version_id)
    .bind(by.end_date)
    .bind(by.start_date)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.get::<i64, _>("tax_rate_id"))
    .collect::<Vec<_>>();

    let limit_ids = sqlx::query(
        r#"
        SELECT tax_limit_id
        FROM tax_limits
        WHERE law_version_id = $1
          AND effective_from <= $2
          AND (effective_to IS NULL OR effective_to >= $3)
        ORDER BY item_code, tax_limit_id
        "#,
    )
    .bind(law.law_version_id)
    .bind(by.end_date)
    .bind(by.start_date)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.get::<i64, _>("tax_limit_id"))
    .collect::<Vec<_>>();

    let forms = sqlx::query(
        r#"
        SELECT DISTINCT ON (form_code) form_version_id, form_code, version_no
        FROM form_versions
        WHERE status IN ('APPROVED', 'ACTIVE')
          AND effective_from <= $1
          AND (effective_to IS NULL OR effective_to >= $2)
        ORDER BY form_code, effective_from DESC, form_version_id DESC
        "#,
    )
    .bind(by.end_date)
    .bind(by.start_date)
    .fetch_all(pool)
    .await?;
    let form_versions = forms
        .into_iter()
        .map(|row| {
            json!({
                "form_version_id": row.get::<i64, _>("form_version_id"),
                "form_code": row.get::<String, _>("form_code"),
                "version_no": row.get::<String, _>("version_no")
            })
        })
        .collect::<Vec<_>>();

    let efile_masters = sqlx::query(
        r#"
        SELECT efile_master_id, master_code, version_no
        FROM efile_masters
        WHERE status IN ('APPROVED', 'ACTIVE')
          AND effective_from <= $1
          AND (effective_to IS NULL OR effective_to >= $2)
        ORDER BY effective_from DESC, efile_master_id DESC
        "#,
    )
    .bind(by.end_date)
    .bind(by.start_date)
    .fetch_all(pool)
    .await?;
    let efile_master_ids = efile_masters
        .into_iter()
        .map(|row| {
            json!({
                "efile_master_id": row.get::<i64, _>("efile_master_id"),
                "master_code": row.get::<String, _>("master_code"),
                "version_no": row.get::<String, _>("version_no")
            })
        })
        .collect::<Vec<_>>();

    let snapshot_data = json!({
        "business_year": {
            "by_id": by.by_id,
            "year_label": by.year_label,
            "start_date": by.start_date.to_string(),
            "end_date": by.end_date.to_string()
        },
        "law": {
            "law_version_id": law.law_version_id,
            "version_code": law.version_code,
            "law_name": law.law_name
        },
        "rate_ids": rate_ids,
        "limit_ids": limit_ids,
        "form_versions": form_versions,
        "efile_masters": efile_master_ids
    });

    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.by_law_snapshot (
            by_id, law_version_id, rate_version_ids, form_version_ids, efile_master_ids, snapshot_data
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (by_id)
        DO UPDATE SET snapshot_data = EXCLUDED.snapshot_data
        RETURNING snapshot_id, by_id, law_version_id, rate_version_ids,
                  form_version_ids, efile_master_ids, snapshot_data, locked, created_at
        "#
    );

    sqlx::query_as::<_, LawSnapshot>(&sql)
        .bind(by_id)
        .bind(law.law_version_id)
        .bind(json!(rate_ids))
        .bind(json!(form_versions))
        .bind(json!(efile_master_ids))
        .bind(snapshot_data)
        .fetch_one(pool)
        .await
        .context("failed to create law snapshot")
}

pub async fn clone_law_snapshot(
    pool: &PgPool,
    tenant: &TenantRef,
    source_by_id: i64,
    target_by_id: i64,
) -> Result<LawSnapshot> {
    let source = ensure_law_snapshot(pool, tenant, source_by_id).await?;
    let target = tenant::get_business_year(pool, tenant, target_by_id).await?;
    let mut snapshot_data = source.snapshot_data.clone();
    if let Some(object) = snapshot_data.as_object_mut() {
        object.insert(
            "business_year".to_string(),
            json!({
                "by_id": target.by_id,
                "year_label": target.year_label,
                "start_date": target.start_date.to_string(),
                "end_date": target.end_date.to_string(),
                "carry_forward_from_by_id": source_by_id
            }),
        );
        object.insert(
            "carry_forward".to_string(),
            json!({
                "source_by_id": source_by_id,
                "copied_at": Utc::now().to_rfc3339()
            }),
        );
    }

    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.by_law_snapshot (
            by_id, law_version_id, rate_version_ids, form_version_ids, efile_master_ids, snapshot_data
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (by_id)
        DO UPDATE SET
            law_version_id = EXCLUDED.law_version_id,
            rate_version_ids = EXCLUDED.rate_version_ids,
            form_version_ids = EXCLUDED.form_version_ids,
            efile_master_ids = EXCLUDED.efile_master_ids,
            snapshot_data = EXCLUDED.snapshot_data
        RETURNING snapshot_id, by_id, law_version_id, rate_version_ids,
                  form_version_ids, efile_master_ids, snapshot_data, locked, created_at
        "#
    );

    sqlx::query_as::<_, LawSnapshot>(&sql)
        .bind(target_by_id)
        .bind(source.law_version_id)
        .bind(source.rate_version_ids)
        .bind(source.form_version_ids)
        .bind(source.efile_master_ids)
        .bind(snapshot_data)
        .fetch_one(pool)
        .await
        .context("failed to clone law snapshot")
}

pub async fn get_law_snapshot(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<LawSnapshot> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT snapshot_id, by_id, law_version_id, rate_version_ids, form_version_ids,
               efile_master_ids, snapshot_data, locked, created_at
        FROM {schema}.by_law_snapshot
        WHERE by_id = $1
        "#
    );

    sqlx::query_as::<_, LawSnapshot>(&sql)
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("law snapshot not found")
}

pub async fn lock_law_snapshot(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<LawSnapshot> {
    let snapshot = ensure_law_snapshot(pool, tenant, by_id).await?;
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.by_law_snapshot
        SET locked = TRUE
        WHERE snapshot_id = $1
        RETURNING snapshot_id, by_id, law_version_id, rate_version_ids,
                  form_version_ids, efile_master_ids, snapshot_data, locked, created_at
        "#
    );
    sqlx::query_as::<_, LawSnapshot>(&sql)
        .bind(snapshot.snapshot_id)
        .fetch_one(pool)
        .await
        .context("failed to lock law snapshot")
}

#[derive(Debug, Clone)]
struct PreparedIncomeItem {
    section: String,
    item_code: String,
    item_name: String,
    amount: i64,
    direction: String,
    disposition: String,
    law_ref: Option<String>,
    metadata: Value,
}

pub async fn calculate_income_adjustment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: CreateIncomeAdjustmentRequest,
) -> Result<IncomeAdjustmentResult> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "adjustment").await?;
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = ensure_law_snapshot(pool, tenant, by_id).await?;
    let accounting_income = match request.accounting_income {
        Some(value) => value,
        None => resolve_accounting_income(pool, tenant, by_id).await?,
    };
    let items = request
        .items
        .into_iter()
        .map(prepare_income_item)
        .collect::<Result<Vec<_>>>()?;

    let gross_income_inclusion = sum_section(&items, "GROSS_INCLUSION");
    let gross_income_exclusion = sum_section(&items, "GROSS_EXCLUSION");
    let loss_inclusion = sum_section(&items, "LOSS_INCLUSION");
    let loss_disallowance = sum_section(&items, "LOSS_DISALLOWANCE");
    let addbacks = gross_income_inclusion + loss_disallowance;
    let deductions = gross_income_exclusion + loss_inclusion;
    let taxable_income = (accounting_income + addbacks - deductions).max(0);
    let law_banner = json!({
        "snapshot_id": snapshot.snapshot_id,
        "locked": snapshot.locked,
        "law": snapshot.snapshot_data.get("law").cloned().unwrap_or_else(|| json!({}))
    });
    let metadata = json!({
        "module": "B1",
        "accounting_income_source": if request.accounting_income.is_some() { "REQUEST" } else { "FINANCIAL_STATEMENT" },
        "sections": {
            "gross_income_inclusion": gross_income_inclusion,
            "gross_income_exclusion": gross_income_exclusion,
            "loss_inclusion": loss_inclusion,
            "loss_disallowance": loss_disallowance
        },
        "law_banner": law_banner
    });

    clear_income_adjustment(pool, tenant, by_id).await?;
    let accounting_adjustment_id = insert_tax_adjustment(
        pool,
        tenant,
        by_id,
        NewAdjustment {
            category: "B1_INCOME",
            code: "B1_ACCOUNTING_INCOME",
            amount: accounting_income,
            direction: "INFO",
            description: "결산서상 당기순이익",
            snapshot_id: snapshot.snapshot_id,
            metadata: metadata.clone(),
        },
    )
    .await?;
    let addback_adjustment_id = insert_tax_adjustment(
        pool,
        tenant,
        by_id,
        NewAdjustment {
            category: "B1_INCOME",
            code: "B1_ADDBACKS",
            amount: addbacks,
            direction: "ADD",
            description: "익금산입/손금불산입 합계",
            snapshot_id: snapshot.snapshot_id,
            metadata: metadata.clone(),
        },
    )
    .await?;
    let deduction_adjustment_id = insert_tax_adjustment(
        pool,
        tenant,
        by_id,
        NewAdjustment {
            category: "B1_INCOME",
            code: "B1_DEDUCTIONS",
            amount: deductions,
            direction: "DEDUCT",
            description: "익금불산입/손금산입 합계",
            snapshot_id: snapshot.snapshot_id,
            metadata: metadata.clone(),
        },
    )
    .await?;
    insert_tax_adjustment(
        pool,
        tenant,
        by_id,
        NewAdjustment {
            category: "B1_INCOME",
            code: "B1_TAXABLE_INCOME",
            amount: taxable_income,
            direction: "INFO",
            description: "차가감 소득금액",
            snapshot_id: snapshot.snapshot_id,
            metadata: metadata.clone(),
        },
    )
    .await?;

    let mut saved_items = Vec::new();
    let mut reserves_created = Vec::new();
    for item in &items {
        let parent_adjustment_id = if item.direction == "ADD" {
            addback_adjustment_id
        } else if item.direction == "DEDUCT" {
            deduction_adjustment_id
        } else {
            accounting_adjustment_id
        };
        let saved_item =
            insert_adjustment_item(pool, tenant, by_id, parent_adjustment_id, "B1", item).await?;
        if item.disposition == "RESERVE" {
            let reserve = insert_reserve(
                pool,
                tenant,
                by_id,
                parent_adjustment_id,
                "B1",
                item,
                by.year_label + 1,
            )
            .await?;
            reserves_created.push(reserve);
        }
        saved_items.push(saved_item);
    }

    Ok(IncomeAdjustmentResult {
        accounting_income,
        gross_income_inclusion,
        gross_income_exclusion,
        loss_inclusion,
        loss_disallowance,
        addbacks,
        deductions,
        taxable_income,
        snapshot_id: snapshot.snapshot_id,
        law_banner,
        items: saved_items,
        reserves_created,
    })
}

pub async fn calculate_asset_based_adjustment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: &str,
    request: AssetBasedAdjustmentRequest,
) -> Result<AssetBasedAdjustmentResult> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "adjustment").await?;
    let module_code = normalize_asset_module(module_code)?;
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = ensure_law_snapshot(pool, tenant, by_id).await?;
    clear_module_adjustment(pool, tenant, by_id, &module_code).await?;

    let law_banner = json!({
        "snapshot_id": snapshot.snapshot_id,
        "locked": snapshot.locked,
        "law": snapshot.snapshot_data.get("law").cloned().unwrap_or_else(|| json!({}))
    });
    let (items, details) = match module_code.as_str() {
        "B4" => depreciation_items(pool, tenant, by_id, snapshot.law_version_id).await?,
        "B5" => retirement_items(request),
        "B6" => bad_debt_items(request),
        "B10" => business_vehicle_items(pool, tenant, by_id, request).await?,
        _ => return Err(anyhow!("unsupported asset based adjustment module")),
    };
    let addbacks: i64 = items
        .iter()
        .filter(|item| item.direction == "ADD")
        .map(|item| item.amount)
        .sum();
    let deductions: i64 = items
        .iter()
        .filter(|item| item.direction == "DEDUCT")
        .map(|item| item.amount)
        .sum();
    let summary_id = insert_tax_adjustment(
        pool,
        tenant,
        by_id,
        NewAdjustment {
            category: "ASSET_BASED",
            code: match module_code.as_str() {
                "B4" => "B4_DEPRECIATION",
                "B5" => "B5_RETIREMENT_RESERVE",
                "B6" => "B6_BAD_DEBT_RESERVE",
                "B10" => "B10_BUSINESS_VEHICLE",
                _ => "ASSET_BASED",
            },
            amount: addbacks - deductions,
            direction: if addbacks >= deductions {
                "ADD"
            } else {
                "DEDUCT"
            },
            description: "자산 기반 세무조정",
            snapshot_id: snapshot.snapshot_id,
            metadata: json!({
                "module": module_code,
                "law_banner": law_banner,
                "details": details
            }),
        },
    )
    .await?;

    let mut saved_items = Vec::new();
    let mut reserves_created = Vec::new();
    for item in &items {
        let saved =
            insert_adjustment_item(pool, tenant, by_id, summary_id, &module_code, item).await?;
        if item.disposition == "RESERVE" && item.amount > 0 {
            reserves_created.push(
                insert_reserve(
                    pool,
                    tenant,
                    by_id,
                    summary_id,
                    &module_code,
                    item,
                    by.year_label + 1,
                )
                .await?,
            );
        }
        saved_items.push(saved);
    }

    Ok(AssetBasedAdjustmentResult {
        module_code,
        addbacks,
        deductions,
        snapshot_id: snapshot.snapshot_id,
        law_banner,
        items: saved_items,
        reserves_created,
        details,
    })
}

pub async fn calculate_transaction_based_adjustment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: &str,
    request: TransactionBasedAdjustmentRequest,
) -> Result<TransactionBasedAdjustmentResult> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "adjustment").await?;
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = ensure_law_snapshot(pool, tenant, by_id).await?;
    let module_code = normalize_transaction_module(module_code)?;
    let law_banner = json!({
        "snapshot_id": snapshot.snapshot_id,
        "locked": snapshot.locked,
        "law": snapshot.snapshot_data.get("law").cloned().unwrap_or_else(|| json!({}))
    });

    clear_transaction_adjustment(pool, tenant, by_id, &module_code).await?;
    let (items, details) = match module_code.as_str() {
        "B2" => {
            donation_adjustment_items(pool, tenant, &by, snapshot.law_version_id, request).await?
        }
        "B3" => {
            entertainment_adjustment_items(pool, tenant, by_id, snapshot.law_version_id, request)
                .await?
        }
        "B9" => {
            interest_adjustment_items(pool, tenant, by_id, snapshot.law_version_id, request).await?
        }
        _ => return Err(anyhow!("invalid transaction based adjustment module")),
    };
    let addbacks: i64 = items
        .iter()
        .filter(|item| item.direction == "ADD")
        .map(|item| item.amount)
        .sum();
    let deductions: i64 = items
        .iter()
        .filter(|item| item.direction == "DEDUCT")
        .map(|item| item.amount)
        .sum();
    let summary_id = insert_tax_adjustment(
        pool,
        tenant,
        by_id,
        NewAdjustment {
            category: "TRANSACTION_ADJUSTMENT",
            code: match module_code.as_str() {
                "B2" => "B2_DONATION_SUMMARY",
                "B3" => "B3_ENTERTAINMENT_SUMMARY",
                "B9" => "B9_INTEREST_SUMMARY",
                _ => "TRANSACTION_SUMMARY",
            },
            amount: addbacks - deductions,
            direction: "INFO",
            description: "Transaction based tax adjustment",
            snapshot_id: snapshot.snapshot_id,
            metadata: json!({
                "module": module_code,
                "law_banner": law_banner,
                "details": details
            }),
        },
    )
    .await?;

    let mut saved_items = Vec::new();
    let mut reserves_created = Vec::new();
    let mut donation_carryforwards = Vec::new();
    for item in &items {
        let saved =
            insert_adjustment_item(pool, tenant, by_id, summary_id, &module_code, item).await?;
        if item.disposition == "RESERVE" && item.amount > 0 {
            let carryforward_years = item
                .metadata
                .get("carryforward_years")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok())
                .unwrap_or(10);
            reserves_created.push(
                insert_reserve(
                    pool,
                    tenant,
                    by_id,
                    summary_id,
                    &module_code,
                    item,
                    by.year_label + carryforward_years,
                )
                .await?,
            );
        }
        if module_code == "B2" {
            if let Some(donation_type) = item
                .metadata
                .get("carryforward_donation_type")
                .and_then(Value::as_str)
            {
                donation_carryforwards.push(
                    insert_donation_carryforward(
                        pool,
                        tenant,
                        NewDonationCarryforward {
                            by_id,
                            source_year: by.year_label,
                            donation_type,
                            amount: item.amount,
                            expires_year: by.year_label
                                + item
                                    .metadata
                                    .get("carryforward_years")
                                    .and_then(Value::as_i64)
                                    .and_then(|value| i32::try_from(value).ok())
                                    .unwrap_or(10),
                            adjustment_item_id: Some(saved.adjustment_item_id),
                        },
                    )
                    .await?,
                );
            }
        }
        saved_items.push(saved);
    }
    if module_code == "B2" {
        donation_carryforwards.extend(list_donation_carryforwards(pool, tenant, by_id).await?);
        donation_carryforwards.sort_by_key(|row| row.carryforward_id);
        donation_carryforwards.dedup_by_key(|row| row.carryforward_id);
    }

    Ok(TransactionBasedAdjustmentResult {
        module_code,
        addbacks,
        deductions,
        snapshot_id: snapshot.snapshot_id,
        law_banner,
        items: saved_items,
        reserves_created,
        donation_carryforwards,
        details,
    })
}

pub async fn calculate_evaluation_adjustment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: &str,
    request: EvaluationAdjustmentRequest,
) -> Result<EvaluationAdjustmentResult> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "adjustment").await?;
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = ensure_law_snapshot(pool, tenant, by_id).await?;
    let module_code = normalize_evaluation_module(module_code)?;
    let law_banner = json!({
        "snapshot_id": snapshot.snapshot_id,
        "locked": snapshot.locked,
        "law": snapshot.snapshot_data.get("law").cloned().unwrap_or_else(|| json!({}))
    });

    clear_evaluation_adjustment(pool, tenant, by_id, &module_code).await?;
    let (items, details) = match module_code.as_str() {
        "B7" => {
            valuation_adjustment_items(
                pool,
                tenant,
                by_id,
                "B7",
                request.positions.unwrap_or_default(),
            )
            .await?
        }
        "B8" => {
            valuation_adjustment_items(
                pool,
                tenant,
                by_id,
                "B8",
                request.positions.unwrap_or_default(),
            )
            .await?
        }
        "B11" => {
            loss_carryforward_items(pool, tenant, &by, snapshot.law_version_id, request).await?
        }
        "B15" => capital_reserve_items(pool, tenant, by_id, request.capital_changes).await?,
        _ => return Err(anyhow!("invalid evaluation adjustment module")),
    };
    let addbacks = items
        .iter()
        .filter(|item| item.direction == "ADD")
        .map(|item| item.amount)
        .sum();
    let deductions = items
        .iter()
        .filter(|item| item.direction == "DEDUCT")
        .map(|item| item.amount)
        .sum();
    let summary_amount = if module_code == "B15" {
        details
            .get("reserve_total")
            .and_then(Value::as_i64)
            .unwrap_or(0)
    } else {
        addbacks - deductions
    };
    let summary_id = insert_tax_adjustment(
        pool,
        tenant,
        by_id,
        NewAdjustment {
            category: "EVALUATION_ADJUSTMENT",
            code: match module_code.as_str() {
                "B7" => "B7_FX_VALUATION_SUMMARY",
                "B8" => "B8_ASSET_VALUATION_SUMMARY",
                "B11" => "B11_LOSS_CARRYFORWARD_SUMMARY",
                "B15" => "B15_CAPITAL_RESERVE_SUMMARY",
                _ => "EVALUATION_SUMMARY",
            },
            amount: summary_amount,
            direction: "INFO",
            description: "Evaluation and carryforward adjustment",
            snapshot_id: snapshot.snapshot_id,
            metadata: json!({
                "module": module_code,
                "law_banner": law_banner,
                "details": details
            }),
        },
    )
    .await?;

    let mut saved_items = Vec::new();
    let mut reserves_created = Vec::new();
    for item in &items {
        let saved =
            insert_adjustment_item(pool, tenant, by_id, summary_id, &module_code, item).await?;
        if item.disposition == "RESERVE" && item.amount > 0 {
            reserves_created.push(
                insert_reserve(
                    pool,
                    tenant,
                    by_id,
                    summary_id,
                    &module_code,
                    item,
                    by.year_label + 1,
                )
                .await?,
            );
        }
        saved_items.push(saved);
    }

    Ok(EvaluationAdjustmentResult {
        module_code,
        addbacks,
        deductions,
        snapshot_id: snapshot.snapshot_id,
        law_banner,
        items: saved_items,
        reserves_created,
        details,
    })
}

pub async fn calculate_tax_amount_adjustment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: &str,
    request: TaxAmountAdjustmentRequest,
) -> Result<TaxAmountAdjustmentResult> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "adjustment").await?;
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = ensure_law_snapshot(pool, tenant, by_id).await?;
    let rates =
        load_applicable_rates(pool, snapshot.law_version_id, by.start_date, by.end_date).await?;
    let module_code = normalize_tax_amount_module(module_code)?;
    let law_banner = json!({
        "snapshot_id": snapshot.snapshot_id,
        "locked": snapshot.locked,
        "law": snapshot.snapshot_data.get("law").cloned().unwrap_or_else(|| json!({}))
    });
    clear_tax_amount_adjustment(pool, tenant, by_id, &module_code).await?;
    let tax_base = request.tax_base.unwrap_or_default().max(0);
    let calculated_tax = request
        .calculated_tax
        .unwrap_or_else(|| calculate_corporate_tax(tax_base, &rates))
        .max(0);
    let (items, details) = match module_code.as_str() {
        "B12" => {
            tax_credit_items(
                pool,
                tenant,
                by_id,
                snapshot.law_version_id,
                calculated_tax,
                request.credits.unwrap_or_default(),
            )
            .await?
        }
        "B13" => {
            minimum_tax_items(
                pool,
                tenant,
                &by,
                snapshot.law_version_id,
                tax_base,
                request
                    .regular_tax_after_credits
                    .unwrap_or(calculated_tax)
                    .max(0),
                request.minimum_tax_rate_bps,
            )
            .await?
        }
        "B14" => {
            penalty_tax_items(pool, tenant, by_id, request.penalties.unwrap_or_default()).await?
        }
        _ => return Err(anyhow!("invalid tax amount adjustment module")),
    };
    let addbacks: i64 = items
        .iter()
        .filter(|item| item.direction == "ADD")
        .map(|item| item.amount)
        .sum();
    let deductions: i64 = items
        .iter()
        .filter(|item| item.direction == "DEDUCT")
        .map(|item| item.amount)
        .sum();
    let determined_tax = (calculated_tax + addbacks - deductions).max(0);
    let summary_id = insert_tax_adjustment(
        pool,
        tenant,
        by_id,
        NewAdjustment {
            category: "TAX_AMOUNT_ADJUSTMENT",
            code: match module_code.as_str() {
                "B12" => "B12_TAX_CREDIT_SUMMARY",
                "B13" => "B13_MINIMUM_TAX_SUMMARY",
                "B14" => "B14_PENALTY_TAX_SUMMARY",
                _ => "TAX_AMOUNT_SUMMARY",
            },
            amount: determined_tax,
            direction: "INFO",
            description: "Tax amount adjustment",
            snapshot_id: snapshot.snapshot_id,
            metadata: json!({
                "module": module_code,
                "tax_base": tax_base,
                "calculated_tax": calculated_tax,
                "determined_tax": determined_tax,
                "law_banner": law_banner,
                "details": details
            }),
        },
    )
    .await?;
    let mut saved_items = Vec::new();
    for item in &items {
        saved_items.push(
            insert_adjustment_item(pool, tenant, by_id, summary_id, &module_code, item).await?,
        );
    }
    Ok(TaxAmountAdjustmentResult {
        module_code,
        addbacks,
        deductions,
        calculated_tax,
        determined_tax,
        snapshot_id: snapshot.snapshot_id,
        law_banner,
        items: saved_items,
        details,
    })
}

pub async fn calculate_special_tax_adjustment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: &str,
    request: SpecialTaxAdjustmentRequest,
) -> Result<SpecialTaxAdjustmentResult> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "adjustment").await?;
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = ensure_law_snapshot(pool, tenant, by_id).await?;
    let rates =
        load_applicable_rates(pool, snapshot.law_version_id, by.start_date, by.end_date).await?;
    let module_code = normalize_special_module(module_code)?;
    let law_banner = json!({
        "snapshot_id": snapshot.snapshot_id,
        "locked": snapshot.locked,
        "law": snapshot.snapshot_data.get("law").cloned().unwrap_or_else(|| json!({}))
    });
    clear_special_adjustment(pool, tenant, by_id, &module_code).await?;
    let (items, taxable_income, details) = match module_code.as_str() {
        "B16" => {
            foreign_corporation_items(
                pool,
                tenant,
                by_id,
                request.foreign_incomes.unwrap_or_default(),
            )
            .await?
        }
        "B17" => {
            consolidated_tax_items(
                pool,
                tenant,
                by_id,
                request.consolidated_entities.unwrap_or_default(),
                request.eliminations.unwrap_or_default(),
                &rates,
            )
            .await?
        }
        _ => return Err(anyhow!("invalid special tax adjustment module")),
    };
    let addbacks: i64 = items
        .iter()
        .filter(|item| item.direction == "ADD")
        .map(|item| item.amount)
        .sum();
    let deductions: i64 = items
        .iter()
        .filter(|item| item.direction == "DEDUCT")
        .map(|item| item.amount)
        .sum();
    let calculated_tax = calculate_corporate_tax(taxable_income, &rates);
    let summary_id = insert_tax_adjustment(
        pool,
        tenant,
        by_id,
        NewAdjustment {
            category: "SPECIAL_TAX_ADJUSTMENT",
            code: if module_code == "B16" {
                "B16_FOREIGN_CORPORATION_SUMMARY"
            } else {
                "B17_CONSOLIDATED_TAX_SUMMARY"
            },
            amount: taxable_income,
            direction: "INFO",
            description: "Special tax adjustment",
            snapshot_id: snapshot.snapshot_id,
            metadata: json!({
                "module": module_code,
                "taxable_income": taxable_income,
                "calculated_tax": calculated_tax,
                "law_banner": law_banner,
                "details": details
            }),
        },
    )
    .await?;
    let mut saved_items = Vec::new();
    for item in &items {
        saved_items.push(
            insert_adjustment_item(pool, tenant, by_id, summary_id, &module_code, item).await?,
        );
    }
    Ok(SpecialTaxAdjustmentResult {
        module_code,
        addbacks,
        deductions,
        taxable_income,
        calculated_tax,
        snapshot_id: snapshot.snapshot_id,
        law_banner,
        items: saved_items,
        details,
    })
}

pub async fn calculate_adjustments(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: CalculateAdjustmentRequest,
) -> Result<CalculationResult> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "adjustment").await?;
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = ensure_law_snapshot(pool, tenant, by_id).await?;
    let rates =
        load_applicable_rates(pool, snapshot.law_version_id, by.start_date, by.end_date).await?;

    let donations = request.donations.unwrap_or(0).max(0);
    let entertainment = request.entertainment_expense.unwrap_or(0).max(0);
    let revenue = request.gross_revenue.unwrap_or(0).max(0);
    let depreciation_book = request.depreciation_book.unwrap_or(0).max(0);
    let depreciation_limit = request.depreciation_tax_limit.unwrap_or(0).max(0);
    let carryforward_loss = request.carryforward_loss.unwrap_or(0).max(0);
    let requested_credits = request.tax_credits.unwrap_or(0).max(0);

    let donation_limit = ((request.accounting_income.max(0) as i128) * 1_000 / 10_000) as i64;
    let deductible_donations = donations.min(donation_limit);
    let non_deductible_donations = donations - deductible_donations;

    let entertainment_limit = 12_000_000 + ((revenue as i128) * 30 / 10_000) as i64;
    let non_deductible_entertainment = (entertainment - entertainment_limit).max(0);

    let depreciation_addback = (depreciation_book - depreciation_limit).max(0);
    let addbacks = non_deductible_donations + non_deductible_entertainment + depreciation_addback;

    let pre_loss_taxable = (request.accounting_income + addbacks - deductible_donations).max(0);
    let loss_deduction = carryforward_loss.min(pre_loss_taxable);
    let deductions = deductible_donations + loss_deduction;
    let taxable_income = (request.accounting_income + addbacks - deductions).max(0);

    let corporate_tax_before_credits = calculate_corporate_tax(taxable_income, &rates);
    let tax_credits = requested_credits.min(corporate_tax_before_credits);
    let corporate_tax = corporate_tax_before_credits - tax_credits;
    let local_income_tax = corporate_tax / 10;
    let total_tax_due = corporate_tax + local_income_tax;

    let details = json!({
        "donations": {
            "reported": donations,
            "limit": donation_limit,
            "deductible": deductible_donations,
            "non_deductible": non_deductible_donations
        },
        "entertainment_expense": {
            "reported": entertainment,
            "limit": entertainment_limit,
            "non_deductible": non_deductible_entertainment
        },
        "depreciation": {
            "book_amount": depreciation_book,
            "tax_limit": depreciation_limit,
            "addback": depreciation_addback
        },
        "carryforward_loss": {
            "available": carryforward_loss,
            "deducted": loss_deduction
        },
        "rates": rates
    });

    let result = CalculationResult {
        accounting_income: request.accounting_income,
        addbacks,
        deductions,
        taxable_income,
        corporate_tax,
        local_income_tax,
        tax_credits,
        total_tax_due,
        snapshot_id: snapshot.snapshot_id,
        details,
    };

    persist_adjustments(pool, tenant, by_id, &result).await?;
    Ok(result)
}

async fn load_applicable_rates(
    pool: &PgPool,
    law_version_id: i64,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
) -> Result<Vec<TaxRate>> {
    sqlx::query_as::<_, TaxRate>(
        r#"
        SELECT tax_rate_id, law_version_id, item_code, taxable_from, taxable_to,
               base_tax, rate_bps, progressive_deduction, effective_from, effective_to, metadata
        FROM tax_rates
        WHERE law_version_id = $1
          AND item_code = 'CORPORATE_TAX'
          AND effective_from <= $2
          AND (effective_to IS NULL OR effective_to >= $3)
        ORDER BY taxable_from
        "#,
    )
    .bind(law_version_id)
    .bind(end_date)
    .bind(start_date)
    .fetch_all(pool)
    .await
    .context("failed to load tax rates")
}

pub fn calculate_corporate_tax(taxable_income: i64, rates: &[TaxRate]) -> i64 {
    rates
        .iter()
        .find(|rate| {
            let under_limit = match rate.taxable_to {
                Some(limit) => taxable_income <= limit,
                None => true,
            };
            taxable_income >= rate.taxable_from && under_limit
        })
        .map(|rate| {
            let raw = taxable_income as i128 * i128::from(rate.rate_bps) / 10_000
                + i128::from(rate.base_tax)
                - i128::from(rate.progressive_deduction);
            raw.max(0) as i64
        })
        .unwrap_or(0)
}

struct NewAdjustment {
    category: &'static str,
    code: &'static str,
    amount: i64,
    direction: &'static str,
    description: &'static str,
    snapshot_id: i64,
    metadata: Value,
}

fn prepare_income_item(input: IncomeAdjustmentItemInput) -> Result<PreparedIncomeItem> {
    if input.amount < 0 {
        return Err(anyhow!("invalid income adjustment amount"));
    }
    let section = normalize_income_section(&input.section)?;
    let direction = income_section_direction(&section);
    let disposition =
        normalize_disposition(input.disposition.as_deref(), input.temporary, direction)?;
    let item_code = input.item_code.trim().to_ascii_uppercase();
    if item_code.is_empty() || input.item_name.trim().is_empty() {
        return Err(anyhow!("invalid income adjustment item"));
    }
    Ok(PreparedIncomeItem {
        section,
        item_code,
        item_name: input.item_name.trim().to_string(),
        amount: input.amount,
        direction: direction.to_string(),
        disposition,
        law_ref: input.law_ref.filter(|value| !value.trim().is_empty()),
        metadata: input.metadata.unwrap_or_else(|| json!({})),
    })
}

fn normalize_income_section(section: &str) -> Result<String> {
    let normalized = section.trim().to_ascii_uppercase().replace([' ', '-'], "_");
    match normalized.as_str() {
        "GROSS_INCLUSION" | "GROSS_INCOME_INCLUSION" | "ADD_GROSS" => {
            Ok("GROSS_INCLUSION".to_string())
        }
        "GROSS_EXCLUSION" | "GROSS_INCOME_EXCLUSION" | "DEDUCT_GROSS" => {
            Ok("GROSS_EXCLUSION".to_string())
        }
        "LOSS_INCLUSION" | "DEDUCTIBLE_INCLUSION" | "DEDUCT_LOSS" => {
            Ok("LOSS_INCLUSION".to_string())
        }
        "LOSS_DISALLOWANCE" | "NON_DEDUCTIBLE" | "ADD_LOSS" => Ok("LOSS_DISALLOWANCE".to_string()),
        _ => Err(anyhow!("invalid B-1 income adjustment section")),
    }
}

fn income_section_direction(section: &str) -> &'static str {
    match section {
        "GROSS_INCLUSION" | "LOSS_DISALLOWANCE" => "ADD",
        "GROSS_EXCLUSION" | "LOSS_INCLUSION" => "DEDUCT",
        _ => "INFO",
    }
}

fn normalize_disposition(
    disposition: Option<&str>,
    temporary: Option<bool>,
    direction: &str,
) -> Result<String> {
    if let Some(disposition) = disposition.filter(|value| !value.trim().is_empty()) {
        let normalized = disposition.trim().to_ascii_uppercase();
        let allowed = ["RESERVE", "OUTFLOW", "OTHER", "INTERNAL"];
        if allowed.contains(&normalized.as_str()) {
            return Ok(normalized);
        }
        return Err(anyhow!("invalid income adjustment disposition"));
    }
    if temporary.unwrap_or(false) {
        Ok("RESERVE".to_string())
    } else if direction == "ADD" {
        Ok("OUTFLOW".to_string())
    } else {
        Ok("OTHER".to_string())
    }
}

fn sum_section(items: &[PreparedIncomeItem], section: &str) -> i64 {
    items
        .iter()
        .filter(|item| item.section == section)
        .map(|item| item.amount)
        .sum()
}

fn normalize_asset_module(module_code: &str) -> Result<String> {
    let normalized = module_code
        .trim()
        .to_ascii_uppercase()
        .replace(['-', '_'], "");
    match normalized.as_str() {
        "B4" | "DEPRECIATION" => Ok("B4".to_string()),
        "B5" | "RETIREMENTRESERVE" => Ok("B5".to_string()),
        "B6" | "BADDEBTRESERVE" => Ok("B6".to_string()),
        "B10" | "BUSINESSVEHICLE" => Ok("B10".to_string()),
        _ => Err(anyhow!("invalid asset based adjustment module")),
    }
}

fn normalize_transaction_module(module_code: &str) -> Result<String> {
    let normalized = module_code
        .trim()
        .to_ascii_uppercase()
        .replace(['-', '_'], "");
    match normalized.as_str() {
        "B2" | "DONATION" | "DONATIONS" => Ok("B2".to_string()),
        "B3" | "ENTERTAINMENT" | "ENTERTAINMENTEXPENSE" => Ok("B3".to_string()),
        "B9" | "INTEREST" | "INTERESTEXPENSE" => Ok("B9".to_string()),
        _ => Err(anyhow!("invalid transaction based adjustment module")),
    }
}

fn normalize_evaluation_module(module_code: &str) -> Result<String> {
    let normalized = module_code
        .trim()
        .to_ascii_uppercase()
        .replace(['-', '_'], "");
    match normalized.as_str() {
        "B7" | "FOREIGNCURRENCY" | "FXVALUATION" => Ok("B7".to_string()),
        "B8" | "INVENTORY" | "SECURITIES" | "VALUATION" => Ok("B8".to_string()),
        "B11" | "LOSSCARRYFORWARD" | "CARRYFORWARDLOSS" => Ok("B11".to_string()),
        "B15" | "CAPITALRESERVE" | "RESERVESCHEDULE" => Ok("B15".to_string()),
        _ => Err(anyhow!("invalid evaluation adjustment module")),
    }
}

fn normalize_tax_amount_module(module_code: &str) -> Result<String> {
    let normalized = module_code
        .trim()
        .to_ascii_uppercase()
        .replace(['-', '_'], "");
    match normalized.as_str() {
        "B12" | "TAXCREDIT" | "CREDITS" => Ok("B12".to_string()),
        "B13" | "MINIMUMTAX" => Ok("B13".to_string()),
        "B14" | "PENALTYTAX" | "PENALTY" => Ok("B14".to_string()),
        _ => Err(anyhow!("invalid tax amount adjustment module")),
    }
}

fn normalize_special_module(module_code: &str) -> Result<String> {
    let normalized = module_code
        .trim()
        .to_ascii_uppercase()
        .replace(['-', '_'], "");
    match normalized.as_str() {
        "B16" | "FOREIGNCORP" | "FOREIGNCORPORATION" => Ok("B16".to_string()),
        "B17" | "CONSOLIDATED" | "CONSOLIDATEDTAX" => Ok("B17".to_string()),
        _ => Err(anyhow!("invalid special tax adjustment module")),
    }
}

fn normalize_any_adjustment_module(module_code: &str) -> Result<String> {
    normalize_asset_module(module_code)
        .or_else(|_| normalize_transaction_module(module_code))
        .or_else(|_| normalize_evaluation_module(module_code))
        .or_else(|_| normalize_tax_amount_module(module_code))
        .or_else(|_| normalize_special_module(module_code))
}

async fn depreciation_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    law_version_id: i64,
) -> Result<(Vec<PreparedIncomeItem>, Value)> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query(&format!(
        "DELETE FROM {schema}.depreciation WHERE by_id = $1"
    ))
    .bind(by_id)
    .execute(pool)
    .await?;
    let assets = sqlx::query(&format!(
        r#"
        SELECT asset_id, asset_code, asset_name, asset_category, is_business_vehicle,
               acquisition_cost, useful_life_years
        FROM {schema}.assets
        WHERE by_id = $1
        ORDER BY asset_code
        "#
    ))
    .bind(by_id)
    .fetch_all(pool)
    .await
    .context("failed to load assets for depreciation")?;
    let mut items = Vec::new();
    let mut details = Vec::new();
    for asset in assets {
        let asset_id = asset.get::<i64, _>("asset_id");
        let asset_code = asset.get::<String, _>("asset_code");
        let asset_name = asset.get::<String, _>("asset_name");
        let category = asset.get::<String, _>("asset_category");
        let cost = asset.get::<i64, _>("acquisition_cost").max(0);
        let book_life = asset.get::<i32, _>("useful_life_years").max(1);
        let tax_life = depreciation_tax_life(pool, law_version_id, &category).await?;
        let book_amount = cost / i64::from(book_life);
        let tax_limit = cost / i64::from(tax_life.max(1));
        let addback = (book_amount - tax_limit).max(0);
        insert_depreciation_row(
            pool,
            tenant,
            by_id,
            asset_id,
            book_amount,
            tax_limit,
            addback,
        )
        .await?;
        details.push(json!({
            "asset_code": asset_code,
            "book_life": book_life,
            "tax_life": tax_life,
            "book_amount": book_amount,
            "tax_limit": tax_limit,
            "addback": addback
        }));
        if addback > 0 {
            items.push(PreparedIncomeItem {
                section: "LOSS_DISALLOWANCE".to_string(),
                item_code: format!("B4_{asset_code}"),
                item_name: format!("{asset_name} depreciation limit excess"),
                amount: addback,
                direction: "ADD".to_string(),
                disposition: "RESERVE".to_string(),
                law_ref: Some("법인세법 시행령 감가상각 한도".to_string()),
                metadata: json!({"asset_id": asset_id, "category": category}),
            });
        }
    }
    Ok((items, json!({ "assets": details })))
}

fn retirement_items(request: AssetBasedAdjustmentRequest) -> (Vec<PreparedIncomeItem>, Value) {
    let book = request.book_reserve.unwrap_or(0).max(0);
    let estimated = request.estimated_liability.unwrap_or(0).max(0);
    let external = request.external_fund.unwrap_or(0).max(0);
    let tax_limit = (estimated - external).max(0);
    let addback = (book - tax_limit).max(0);
    let deduction = (tax_limit - book).max(0);
    let (amount, direction) = if addback > 0 {
        (addback, "ADD")
    } else {
        (deduction, "DEDUCT")
    };
    let items = if amount > 0 {
        vec![PreparedIncomeItem {
            section: if direction == "ADD" {
                "LOSS_DISALLOWANCE"
            } else {
                "LOSS_INCLUSION"
            }
            .to_string(),
            item_code: "B5_RETIREMENT_LIMIT".to_string(),
            item_name: "Retirement reserve limit adjustment".to_string(),
            amount,
            direction: direction.to_string(),
            disposition: "RESERVE".to_string(),
            law_ref: Some("퇴직급여충당금 한도".to_string()),
            metadata: json!({}),
        }]
    } else {
        Vec::new()
    };
    (
        items,
        json!({ "book_reserve": book, "estimated_liability": estimated, "external_fund": external, "tax_limit": tax_limit }),
    )
}

fn bad_debt_items(request: AssetBasedAdjustmentRequest) -> (Vec<PreparedIncomeItem>, Value) {
    let book = request.book_reserve.unwrap_or(0).max(0);
    let balance = request.receivable_balance.unwrap_or(0).max(0);
    let rate_bps = request.rate_bps.unwrap_or(100).max(0);
    let rate_limit = ((balance as i128) * i128::from(rate_bps) / 10_000) as i64;
    let tax_limit = rate_limit.max(request.actual_bad_debt.unwrap_or(0).max(0));
    let addback = (book - tax_limit).max(0);
    let items = if addback > 0 {
        vec![PreparedIncomeItem {
            section: "LOSS_DISALLOWANCE".to_string(),
            item_code: "B6_BAD_DEBT_LIMIT".to_string(),
            item_name: "Bad debt reserve limit excess".to_string(),
            amount: addback,
            direction: "ADD".to_string(),
            disposition: "RESERVE".to_string(),
            law_ref: Some("대손충당금 한도".to_string()),
            metadata: json!({}),
        }]
    } else {
        Vec::new()
    };
    (
        items,
        json!({ "book_reserve": book, "receivable_balance": balance, "rate_bps": rate_bps, "tax_limit": tax_limit }),
    )
}

async fn business_vehicle_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: AssetBasedAdjustmentRequest,
) -> Result<(Vec<PreparedIncomeItem>, Value)> {
    let schema = quote_ident(&tenant.schema_name)?;
    let assets = sqlx::query(&format!(
        r#"
        SELECT asset_id, asset_code, asset_name, acquisition_cost, useful_life_years
        FROM {schema}.assets
        WHERE by_id = $1 AND is_business_vehicle = TRUE
        ORDER BY asset_code
        "#
    ))
    .bind(by_id)
    .fetch_all(pool)
    .await
    .context("failed to load business vehicle assets")?;
    let mut items = Vec::new();
    let mut details = Vec::new();
    for asset in assets {
        let asset_id = asset.get::<i64, _>("asset_id");
        let asset_code = asset.get::<String, _>("asset_code");
        let asset_name = asset.get::<String, _>("asset_name");
        let cost = asset.get::<i64, _>("acquisition_cost").max(0);
        let book_life = asset.get::<i32, _>("useful_life_years").max(1);
        let use_bps = vehicle_business_use_bps(pool, tenant, by_id, asset_id)
            .await?
            .or(request.business_use_bps)
            .unwrap_or(10_000)
            .clamp(0, 10_000);
        let book_amount = cost / i64::from(book_life);
        let tax_basis = cost.min(80_000_000);
        let annual_limit = (tax_basis / 5).min(15_000_000);
        let tax_limit = ((annual_limit as i128) * i128::from(use_bps) / 10_000) as i64;
        let addback = (book_amount - tax_limit).max(0);
        details.push(json!({
            "asset_code": asset_code,
            "book_amount": book_amount,
            "tax_basis": tax_basis,
            "business_use_bps": use_bps,
            "tax_limit": tax_limit,
            "addback": addback
        }));
        if addback > 0 {
            items.push(PreparedIncomeItem {
                section: "LOSS_DISALLOWANCE".to_string(),
                item_code: format!("B10_{asset_code}"),
                item_name: format!("{asset_name} business vehicle limit excess"),
                amount: addback,
                direction: "ADD".to_string(),
                disposition: "RESERVE".to_string(),
                law_ref: Some("업무용승용차 한도".to_string()),
                metadata: json!({"asset_id": asset_id}),
            });
        }
    }
    Ok((items, json!({ "vehicles": details })))
}

#[derive(Debug, Clone)]
struct TransactionRow {
    description: String,
    amount: i64,
    evidence_type: String,
}

struct NewDonationCarryforward<'a> {
    by_id: i64,
    source_year: i32,
    donation_type: &'a str,
    amount: i64,
    expires_year: i32,
    adjustment_item_id: Option<i64>,
}

struct NewValuationPosition<'a> {
    by_id: i64,
    module_code: &'a str,
    input: &'a ValuationPositionInput,
    valuation_method: &'a str,
    tax_amount: i64,
    adjustment_amount: i64,
}

struct NewTaxCreditClaim<'a> {
    by_id: i64,
    credit_type: &'a str,
    base_amount: i64,
    rate_bps: i64,
    requested_amount: i64,
    allowed_amount: i64,
}

struct NewForeignIncomeItem<'a> {
    by_id: i64,
    income_type: &'a str,
    gross_amount: i64,
    attributable_expense: i64,
    pe_allocation_bps: i64,
    allocated_income: i64,
    withholding_tax: i64,
}

async fn donation_adjustment_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by: &crate::domain::BusinessYear,
    law_version_id: i64,
    request: TransactionBasedAdjustmentRequest,
) -> Result<(Vec<PreparedIncomeItem>, Value)> {
    mark_expired_donation_carryforwards(pool, tenant, by.customer_id, by.year_label).await?;
    let transactions = load_transaction_rows(pool, tenant, by.by_id, "DONATION").await?;
    let special_amount = transactions
        .iter()
        .filter(|row| classify_donation_type(&row.description) == "SPECIAL")
        .map(|row| row.amount)
        .sum::<i64>();
    let general_amount = transactions
        .iter()
        .filter(|row| classify_donation_type(&row.description) == "GENERAL")
        .map(|row| row.amount)
        .sum::<i64>();
    let base_income = match request
        .taxable_income_before_donation
        .or(request.accounting_income)
    {
        Some(value) => value.max(0),
        None => resolve_accounting_income(pool, tenant, by.by_id)
            .await?
            .max(0),
    };
    let special_bps =
        tax_limit_amount(pool, law_version_id, "DONATION_SPECIAL_LIMIT_BPS", 5_000).await?;
    let general_bps =
        tax_limit_amount(pool, law_version_id, "DONATION_GENERAL_LIMIT_BPS", 1_000).await?;
    let carryforward_years =
        tax_limit_amount(pool, law_version_id, "DONATION_CARRYFORWARD_YEARS", 10).await?;
    let special_limit = amount_by_bps(base_income, special_bps);
    let special_current_deductible = special_amount.min(special_limit);
    let (special_prior_used, special_allocations) = allocate_donation_carryforwards(
        pool,
        tenant,
        by.customer_id,
        by.year_label,
        "SPECIAL",
        special_limit - special_current_deductible,
    )
    .await?;
    let general_base = (base_income - special_current_deductible - special_prior_used).max(0);
    let general_limit = amount_by_bps(general_base, general_bps);
    let general_current_deductible = general_amount.min(general_limit);
    let (general_prior_used, general_allocations) = allocate_donation_carryforwards(
        pool,
        tenant,
        by.customer_id,
        by.year_label,
        "GENERAL",
        general_limit - general_current_deductible,
    )
    .await?;
    let special_excess = (special_amount - special_current_deductible).max(0);
    let general_excess = (general_amount - general_current_deductible).max(0);

    let mut items = Vec::new();
    if special_excess > 0 {
        items.push(PreparedIncomeItem {
            section: "LOSS_DISALLOWANCE".to_string(),
            item_code: "B2_SPECIAL_DONATION_EXCESS".to_string(),
            item_name: "Special donation limit excess".to_string(),
            amount: special_excess,
            direction: "ADD".to_string(),
            disposition: "RESERVE".to_string(),
            law_ref: Some("CIT donation limit".to_string()),
            metadata: json!({
                "carryforward_donation_type": "SPECIAL",
                "carryforward_years": carryforward_years
            }),
        });
    }
    if general_excess > 0 {
        items.push(PreparedIncomeItem {
            section: "LOSS_DISALLOWANCE".to_string(),
            item_code: "B2_GENERAL_DONATION_EXCESS".to_string(),
            item_name: "General donation limit excess".to_string(),
            amount: general_excess,
            direction: "ADD".to_string(),
            disposition: "RESERVE".to_string(),
            law_ref: Some("CIT donation limit".to_string()),
            metadata: json!({
                "carryforward_donation_type": "GENERAL",
                "carryforward_years": carryforward_years
            }),
        });
    }
    if special_prior_used + general_prior_used > 0 {
        items.push(PreparedIncomeItem {
            section: "LOSS_INCLUSION".to_string(),
            item_code: "B2_PRIOR_CARRYFORWARD_USED".to_string(),
            item_name: "Prior donation carryforward used".to_string(),
            amount: special_prior_used + general_prior_used,
            direction: "DEDUCT".to_string(),
            disposition: "OTHER".to_string(),
            law_ref: Some("CIT donation carryforward".to_string()),
            metadata: json!({
                "special_used": special_prior_used,
                "general_used": general_prior_used
            }),
        });
    }

    Ok((
        items,
        json!({
            "base_income": base_income,
            "transaction_count": transactions.len(),
            "special": {
                "reported": special_amount,
                "limit_bps": special_bps,
                "limit": special_limit,
                "current_deductible": special_current_deductible,
                "prior_used": special_prior_used,
                "excess": special_excess,
                "allocations": special_allocations
            },
            "general": {
                "reported": general_amount,
                "limit_bps": general_bps,
                "limit": general_limit,
                "current_deductible": general_current_deductible,
                "prior_used": general_prior_used,
                "excess": general_excess,
                "allocations": general_allocations
            },
            "carryforward_years": carryforward_years
        }),
    ))
}

async fn entertainment_adjustment_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    law_version_id: i64,
    request: TransactionBasedAdjustmentRequest,
) -> Result<(Vec<PreparedIncomeItem>, Value)> {
    let transactions = load_transaction_rows(pool, tenant, by_id, "ENTERTAINMENT").await?;
    let total = transactions.iter().map(|row| row.amount).sum::<i64>();
    let non_card = transactions
        .iter()
        .filter(|row| !is_card_evidence(&row.evidence_type))
        .map(|row| row.amount)
        .sum::<i64>();
    let card_eligible = (total - non_card).max(0);
    let (revenue, revenue_lines) = resolve_entertainment_revenue(
        pool,
        tenant,
        by_id,
        request.gross_revenue,
        request.revenue_breakdowns,
    )
    .await?;
    let base_limit =
        tax_limit_amount(pool, law_version_id, "ENTERTAINMENT_BASE_LIMIT", 12_000_000).await?;
    let revenue_rate_bps =
        tax_limit_amount(pool, law_version_id, "ENTERTAINMENT_REVENUE_RATE_BPS", 30).await?;
    let no_card_bps = tax_limit_amount(
        pool,
        law_version_id,
        "ENTERTAINMENT_NO_CARD_DISALLOW_BPS",
        10_000,
    )
    .await?;
    let revenue_limit = amount_by_bps(revenue, revenue_rate_bps);
    let tax_limit = base_limit + revenue_limit;
    let no_card_disallowed = amount_by_bps(non_card, no_card_bps);
    let limit_excess = (card_eligible - tax_limit).max(0);

    let mut items = Vec::new();
    if no_card_disallowed > 0 {
        items.push(PreparedIncomeItem {
            section: "LOSS_DISALLOWANCE".to_string(),
            item_code: "B3_NO_CARD_DISALLOWANCE".to_string(),
            item_name: "Entertainment expense without qualified evidence".to_string(),
            amount: no_card_disallowed,
            direction: "ADD".to_string(),
            disposition: "OUTFLOW".to_string(),
            law_ref: Some("CIT entertainment evidence rule".to_string()),
            metadata: json!({ "non_card_amount": non_card, "disallow_bps": no_card_bps }),
        });
    }
    if limit_excess > 0 {
        items.push(PreparedIncomeItem {
            section: "LOSS_DISALLOWANCE".to_string(),
            item_code: "B3_ENTERTAINMENT_LIMIT_EXCESS".to_string(),
            item_name: "Entertainment expense limit excess".to_string(),
            amount: limit_excess,
            direction: "ADD".to_string(),
            disposition: "OUTFLOW".to_string(),
            law_ref: Some("CIT entertainment limit".to_string()),
            metadata: json!({ "tax_limit": tax_limit }),
        });
    }

    Ok((
        items,
        json!({
            "transaction_count": transactions.len(),
            "reported": total,
            "card_eligible": card_eligible,
            "non_card": non_card,
            "gross_revenue": revenue,
            "revenue_breakdowns": revenue_lines,
            "base_limit": base_limit,
            "revenue_rate_bps": revenue_rate_bps,
            "revenue_limit": revenue_limit,
            "tax_limit": tax_limit,
            "no_card_disallowed": no_card_disallowed,
            "limit_excess": limit_excess
        }),
    ))
}

async fn interest_adjustment_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    law_version_id: i64,
    request: TransactionBasedAdjustmentRequest,
) -> Result<(Vec<PreparedIncomeItem>, Value)> {
    let transactions = load_transaction_rows(pool, tenant, by_id, "INTEREST").await?;
    let mut unknown_creditor = 0;
    let mut unknown_recipient = 0;
    let mut construction = 0;
    let mut non_business = 0;
    let mut general = 0;
    for row in &transactions {
        match classify_interest_type(&row.description).as_str() {
            "UNKNOWN_CREDITOR" => unknown_creditor += row.amount,
            "UNKNOWN_RECIPIENT" => unknown_recipient += row.amount,
            "CONSTRUCTION" => construction += row.amount,
            "NON_BUSINESS" => non_business += row.amount,
            _ => general += row.amount,
        }
    }
    let default_rate =
        tax_limit_amount(pool, law_version_id, "INTEREST_DEEMED_RATE_BPS", 460).await?;
    let rate_bps = request
        .weighted_average_interest_rate_bps
        .unwrap_or(i32::try_from(default_rate).unwrap_or(460))
        .max(0);
    let loan_balance = request.weighted_average_loan_balance.unwrap_or(0).max(0);
    let deemed_interest = amount_by_bps(loan_balance, i64::from(rate_bps));
    let manual = request.manual_interest_disallowance.unwrap_or(0).max(0);
    insert_loan_interest_fact(pool, tenant, by_id, loan_balance, rate_bps, deemed_interest).await?;

    let buckets = [
        (
            "B9_UNKNOWN_CREDITOR",
            "Interest paid to unidentified creditor",
            unknown_creditor,
            "UNKNOWN_CREDITOR",
        ),
        (
            "B9_UNKNOWN_RECIPIENT",
            "Interest paid to unidentified recipient",
            unknown_recipient,
            "UNKNOWN_RECIPIENT",
        ),
        (
            "B9_CONSTRUCTION_INTEREST",
            "Construction financing interest",
            construction,
            "CONSTRUCTION",
        ),
        (
            "B9_NON_BUSINESS_INTEREST",
            "Non-business asset related interest",
            non_business,
            "NON_BUSINESS",
        ),
        (
            "B9_DEEMED_LOAN_INTEREST",
            "Deemed interest from weighted loan balance",
            deemed_interest,
            "WEIGHTED_AVERAGE_LOAN",
        ),
        (
            "B9_MANUAL_DISALLOWANCE",
            "Manual interest disallowance",
            manual,
            "MANUAL",
        ),
    ];
    let items = buckets
        .into_iter()
        .filter(|(_, _, amount, _)| *amount > 0)
        .map(|(code, name, amount, interest_type)| PreparedIncomeItem {
            section: "LOSS_DISALLOWANCE".to_string(),
            item_code: code.to_string(),
            item_name: name.to_string(),
            amount,
            direction: "ADD".to_string(),
            disposition: "OUTFLOW".to_string(),
            law_ref: Some("CIT interest expense disallowance".to_string()),
            metadata: json!({ "interest_type": interest_type }),
        })
        .collect::<Vec<_>>();

    Ok((
        items,
        json!({
            "transaction_count": transactions.len(),
            "unknown_creditor": unknown_creditor,
            "unknown_recipient": unknown_recipient,
            "construction": construction,
            "non_business": non_business,
            "general": general,
            "weighted_average_loan_balance": loan_balance,
            "weighted_average_interest_rate_bps": rate_bps,
            "deemed_interest": deemed_interest,
            "manual_interest_disallowance": manual
        }),
    ))
}

async fn valuation_adjustment_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: &str,
    positions: Vec<ValuationPositionInput>,
) -> Result<(Vec<PreparedIncomeItem>, Value)> {
    let mut items = Vec::new();
    let mut details = Vec::new();
    for input in positions {
        let item_code = input.item_code.trim().to_ascii_uppercase();
        if item_code.is_empty() || input.item_name.trim().is_empty() {
            return Err(anyhow!("invalid valuation position"));
        }
        let monetary = input.monetary.unwrap_or(true);
        let method = input
            .valuation_method
            .clone()
            .unwrap_or_else(|| {
                if module_code == "B7" && monetary {
                    "CLOSING_RATE".to_string()
                } else {
                    "BOOK_METHOD".to_string()
                }
            })
            .trim()
            .to_ascii_uppercase();
        let tax_amount = input.tax_amount.unwrap_or_else(|| {
            let foreign_amount = input.foreign_amount.unwrap_or(0.0);
            let rate = if method.contains("CLOSING") {
                input.closing_rate.or(input.book_rate).unwrap_or(1.0)
            } else {
                input.book_rate.or(input.closing_rate).unwrap_or(1.0)
            };
            (foreign_amount * rate).round() as i64
        });
        let adjustment = input.book_amount - tax_amount;
        insert_valuation_position(
            pool,
            tenant,
            NewValuationPosition {
                by_id,
                module_code,
                input: &input,
                valuation_method: &method,
                tax_amount,
                adjustment_amount: adjustment,
            },
        )
        .await?;
        details.push(json!({
            "item_code": item_code,
            "item_name": input.item_name,
            "position_type": input.position_type.clone().unwrap_or_else(|| "GENERAL".to_string()),
            "monetary": monetary,
            "valuation_method": method,
            "book_amount": input.book_amount,
            "tax_amount": tax_amount,
            "adjustment_amount": adjustment
        }));
        if adjustment != 0 {
            let direction = if adjustment > 0 { "ADD" } else { "DEDUCT" };
            items.push(PreparedIncomeItem {
                section: if direction == "ADD" {
                    "LOSS_DISALLOWANCE"
                } else {
                    "LOSS_INCLUSION"
                }
                .to_string(),
                item_code: format!("{module_code}_{item_code}"),
                item_name: input.item_name,
                amount: adjustment.abs(),
                direction: direction.to_string(),
                disposition: "RESERVE".to_string(),
                law_ref: Some("CIT valuation rule".to_string()),
                metadata: json!({
                    "valuation_method": method,
                    "monetary": monetary,
                    "tax_amount": tax_amount
                }),
            });
        }
    }
    Ok((
        items,
        json!({
            "module": module_code,
            "position_count": details.len(),
            "positions": details
        }),
    ))
}

async fn loss_carryforward_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by: &crate::domain::BusinessYear,
    law_version_id: i64,
    request: EvaluationAdjustmentRequest,
) -> Result<(Vec<PreparedIncomeItem>, Value)> {
    if let Some(losses) = request.loss_carryforwards {
        upsert_loss_carryforwards(pool, tenant, by.customer_id, &losses, by.year_label).await?;
    }
    expire_loss_carryforwards(pool, tenant, by.customer_id, by.year_label).await?;
    let taxable_income = match request.taxable_income_before_loss {
        Some(value) => value.max(0),
        None => resolve_accounting_income(pool, tenant, by.by_id)
            .await?
            .max(0),
    };
    let is_sme = load_customer_is_sme(pool, tenant, by.customer_id).await?;
    let limit_code = if is_sme {
        "LOSS_DEDUCTION_LIMIT_BPS_SME"
    } else {
        "LOSS_DEDUCTION_LIMIT_BPS_GENERAL"
    };
    let limit_bps = tax_limit_amount(
        pool,
        law_version_id,
        limit_code,
        if is_sme { 10_000 } else { 8_000 },
    )
    .await?;
    let deduction_limit = amount_by_bps(taxable_income, limit_bps);
    let (deducted, allocations) =
        allocate_loss_carryforwards(pool, tenant, by.customer_id, by.year_label, deduction_limit)
            .await?;
    let current_losses = list_loss_carryforwards(pool, tenant, by.customer_id).await?;
    let expiration_alerts = current_losses
        .iter()
        .filter(|loss| loss.remaining_amount > 0 && loss.expires_year <= by.year_label + 1)
        .map(|loss| {
            json!({
                "origin_year": loss.origin_year,
                "remaining_amount": loss.remaining_amount,
                "expires_year": loss.expires_year
            })
        })
        .collect::<Vec<_>>();
    let items = if deducted > 0 {
        vec![PreparedIncomeItem {
            section: "LOSS_INCLUSION".to_string(),
            item_code: "B11_LOSS_CARRYFORWARD_DEDUCTION".to_string(),
            item_name: "Loss carryforward deduction".to_string(),
            amount: deducted,
            direction: "DEDUCT".to_string(),
            disposition: "OTHER".to_string(),
            law_ref: Some("CIT loss carryforward rule".to_string()),
            metadata: json!({ "allocations": allocations }),
        }]
    } else {
        Vec::new()
    };
    Ok((
        items,
        json!({
            "taxable_income_before_loss": taxable_income,
            "is_sme": is_sme,
            "limit_bps": limit_bps,
            "deduction_limit": deduction_limit,
            "deducted": deducted,
            "allocations": allocations,
            "losses": current_losses,
            "expiration_alerts": expiration_alerts
        }),
    ))
}

async fn capital_reserve_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    capital_changes: Option<Vec<CapitalChangeInput>>,
) -> Result<(Vec<PreparedIncomeItem>, Value)> {
    if let Some(changes) = capital_changes {
        replace_capital_changes(pool, tenant, by_id, &changes).await?;
    }
    let changes = list_capital_changes(pool, tenant, by_id).await?;
    let reserve_rows = aggregate_reserves(pool, tenant, by_id).await?;
    let reserve_total = reserve_rows
        .iter()
        .map(|row| row.get("amount").and_then(Value::as_i64).unwrap_or(0))
        .sum::<i64>();
    let mut items = reserve_rows
        .iter()
        .map(|row| PreparedIncomeItem {
            section: "CAPITAL_RESERVE_SCHEDULE".to_string(),
            item_code: format!(
                "B15_{}",
                row.get("reserve_code")
                    .and_then(Value::as_str)
                    .unwrap_or("RESERVE")
            ),
            item_name: format!(
                "{} reserve rollforward",
                row.get("source_module")
                    .and_then(Value::as_str)
                    .unwrap_or("MODULE")
            ),
            amount: row.get("amount").and_then(Value::as_i64).unwrap_or(0),
            direction: "INFO".to_string(),
            disposition: "INTERNAL".to_string(),
            law_ref: Some("Capital and reserve schedule".to_string()),
            metadata: row.clone(),
        })
        .collect::<Vec<_>>();
    for change in &changes {
        items.push(PreparedIncomeItem {
            section: "CAPITAL_CHANGE".to_string(),
            item_code: format!("B15_CAPITAL_{}", change.capital_change_id),
            item_name: change.change_type.clone(),
            amount: change.amount,
            direction: "INFO".to_string(),
            disposition: "INTERNAL".to_string(),
            law_ref: Some("Capital change".to_string()),
            metadata: json!({
                "change_date": change.change_date,
                "description": change.description
            }),
        });
    }
    Ok((
        items,
        json!({
            "reserve_total": reserve_total,
            "reserves": reserve_rows,
            "capital_changes": changes
        }),
    ))
}

async fn tax_credit_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    law_version_id: i64,
    calculated_tax: i64,
    credits: Vec<TaxCreditInput>,
) -> Result<(Vec<PreparedIncomeItem>, Value)> {
    let mut remaining_tax = calculated_tax.max(0);
    let mut items = Vec::new();
    let mut details = Vec::new();
    for credit in credits {
        let credit_type = credit.credit_type.trim().to_ascii_uppercase();
        if credit_type.is_empty() || credit.base_amount <= 0 {
            continue;
        }
        let rate_bps = match credit.rate_bps {
            Some(value) => value.max(0),
            None => credit_rate_bps(pool, law_version_id, &credit_type).await?,
        };
        let requested = credit
            .requested_amount
            .unwrap_or_else(|| amount_by_bps(credit.base_amount, rate_bps))
            .max(0);
        let allowed = requested.min(remaining_tax);
        remaining_tax -= allowed;
        insert_tax_credit_claim(
            pool,
            tenant,
            NewTaxCreditClaim {
                by_id,
                credit_type: &credit_type,
                base_amount: credit.base_amount,
                rate_bps,
                requested_amount: requested,
                allowed_amount: allowed,
            },
        )
        .await?;
        details.push(json!({
            "credit_type": credit_type,
            "base_amount": credit.base_amount,
            "rate_bps": rate_bps,
            "requested_amount": requested,
            "allowed_amount": allowed
        }));
        if allowed > 0 {
            items.push(PreparedIncomeItem {
                section: "TAX_CREDIT".to_string(),
                item_code: format!("B12_{credit_type}"),
                item_name: format!("{credit_type} tax credit"),
                amount: allowed,
                direction: "DEDUCT".to_string(),
                disposition: "OTHER".to_string(),
                law_ref: Some("CIT tax credit rule".to_string()),
                metadata: json!({ "requested_amount": requested, "rate_bps": rate_bps }),
            });
        }
    }
    Ok((
        items,
        json!({
            "calculated_tax": calculated_tax,
            "credits": details,
            "tax_after_credits": remaining_tax
        }),
    ))
}

async fn minimum_tax_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by: &crate::domain::BusinessYear,
    law_version_id: i64,
    tax_base: i64,
    regular_tax: i64,
    requested_rate_bps: Option<i64>,
) -> Result<(Vec<PreparedIncomeItem>, Value)> {
    let is_sme = load_customer_is_sme(pool, tenant, by.customer_id).await?;
    let limit_code = if is_sme {
        "MINIMUM_TAX_RATE_BPS_SME"
    } else {
        "MINIMUM_TAX_RATE_BPS_GENERAL"
    };
    let rate_bps = requested_rate_bps
        .unwrap_or(
            tax_limit_amount(
                pool,
                law_version_id,
                limit_code,
                if is_sme { 1_000 } else { 1_700 },
            )
            .await?,
        )
        .max(0);
    let minimum_tax = amount_by_bps(tax_base, rate_bps);
    let additional_tax = minimum_tax_extra_due(regular_tax, tax_base, rate_bps);
    insert_minimum_tax_result(
        pool,
        tenant,
        by.by_id,
        tax_base,
        regular_tax,
        minimum_tax,
        additional_tax,
    )
    .await?;
    let items = if additional_tax > 0 {
        vec![PreparedIncomeItem {
            section: "MINIMUM_TAX".to_string(),
            item_code: "B13_MINIMUM_TAX_ADDITIONAL".to_string(),
            item_name: "Minimum tax additional amount".to_string(),
            amount: additional_tax,
            direction: "ADD".to_string(),
            disposition: "OTHER".to_string(),
            law_ref: Some("CIT minimum tax rule".to_string()),
            metadata: json!({ "minimum_tax": minimum_tax, "regular_tax": regular_tax }),
        }]
    } else {
        Vec::new()
    };
    Ok((
        items,
        json!({
            "tax_base": tax_base,
            "regular_tax": regular_tax,
            "minimum_tax_rate_bps": rate_bps,
            "minimum_tax": minimum_tax,
            "additional_tax": additional_tax,
            "is_sme": is_sme
        }),
    ))
}

async fn penalty_tax_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    penalties: Vec<PenaltyTaxInput>,
) -> Result<(Vec<PreparedIncomeItem>, Value)> {
    let mut items = Vec::new();
    let mut details = Vec::new();
    for penalty in penalties {
        let penalty_type = penalty.penalty_type.trim().to_ascii_uppercase();
        if penalty_type.is_empty() || penalty.tax_base <= 0 || penalty.rate_bps <= 0 {
            continue;
        }
        let multiplier = i64::from(penalty.days_late.unwrap_or(1).max(1));
        let raw = amount_by_bps(penalty.tax_base, penalty.rate_bps * multiplier);
        let reduction_bps = penalty.reduction_bps.unwrap_or(0).clamp(0, 10_000);
        let amount = amount_by_bps(raw, 10_000 - reduction_bps);
        insert_penalty_tax_item(pool, tenant, by_id, &penalty_type, &penalty, amount).await?;
        details.push(json!({
            "penalty_type": penalty_type,
            "tax_base": penalty.tax_base,
            "rate_bps": penalty.rate_bps,
            "days_late": penalty.days_late,
            "reduction_bps": reduction_bps,
            "penalty_amount": amount
        }));
        if amount > 0 {
            items.push(PreparedIncomeItem {
                section: "PENALTY_TAX".to_string(),
                item_code: format!("B14_{penalty_type}"),
                item_name: format!("{penalty_type} penalty tax"),
                amount,
                direction: "ADD".to_string(),
                disposition: "OTHER".to_string(),
                law_ref: Some("Penalty tax rule".to_string()),
                metadata: json!({ "reduction_bps": reduction_bps }),
            });
        }
    }
    let total = items.iter().map(|item| item.amount).sum::<i64>();
    Ok((
        items,
        json!({ "penalties": details, "penalty_total": total }),
    ))
}

pub fn minimum_tax_extra_due(regular_tax: i64, tax_base: i64, minimum_tax_rate_bps: i64) -> i64 {
    (amount_by_bps(tax_base, minimum_tax_rate_bps) - regular_tax.max(0)).max(0)
}

async fn foreign_corporation_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    incomes: Vec<ForeignIncomeInput>,
) -> Result<(Vec<PreparedIncomeItem>, i64, Value)> {
    let mut details = Vec::new();
    let mut taxable_income = 0;
    let mut withholding_total = 0;
    for income in incomes {
        let income_type = normalize_foreign_income_type(&income.income_type)?;
        let gross = income.gross_amount.max(0);
        let expense = income.attributable_expense.unwrap_or(0).max(0);
        let pe_bps = income.pe_allocation_bps.unwrap_or(10_000).clamp(0, 10_000);
        let net = (gross - expense).max(0);
        let allocated = amount_by_bps(net, pe_bps);
        let withholding = income.withholding_tax.unwrap_or(0).max(0);
        insert_foreign_income_item(
            pool,
            tenant,
            NewForeignIncomeItem {
                by_id,
                income_type: &income_type,
                gross_amount: gross,
                attributable_expense: expense,
                pe_allocation_bps: pe_bps,
                allocated_income: allocated,
                withholding_tax: withholding,
            },
        )
        .await?;
        taxable_income += allocated;
        withholding_total += withholding;
        details.push(json!({
            "income_type": income_type,
            "gross_amount": gross,
            "attributable_expense": expense,
            "pe_allocation_bps": pe_bps,
            "allocated_income": allocated,
            "withholding_tax": withholding
        }));
    }
    let mut items = Vec::new();
    if taxable_income > 0 {
        items.push(PreparedIncomeItem {
            section: "FOREIGN_CORPORATION".to_string(),
            item_code: "B16_DOMESTIC_SOURCE_INCOME".to_string(),
            item_name: "Domestic source income allocated to PE".to_string(),
            amount: taxable_income,
            direction: "ADD".to_string(),
            disposition: "OTHER".to_string(),
            law_ref: Some("Foreign corporation domestic source income".to_string()),
            metadata: json!({ "income_count": details.len() }),
        });
    }
    Ok((
        items,
        taxable_income,
        json!({
            "foreign_mode": true,
            "domestic_source_income": taxable_income,
            "withholding_tax_total": withholding_total,
            "incomes": details
        }),
    ))
}

async fn consolidated_tax_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    entities: Vec<ConsolidatedEntityInput>,
    eliminations: Vec<ConsolidationEliminationInput>,
    rates: &[TaxRate],
) -> Result<(Vec<PreparedIncomeItem>, i64, Value)> {
    if entities.len() < 2 {
        return Err(anyhow!("at least two consolidated entities are required"));
    }
    if entities.iter().any(|entity| entity.ownership_bps != 10_000) {
        return Err(anyhow!("consolidated entities must be 100 percent owned"));
    }
    let entity_income = entities
        .iter()
        .map(|entity| entity.taxable_income.max(0))
        .sum::<i64>();
    let mut elimination_total = 0;
    let mut elimination_details = Vec::new();
    for elimination in &eliminations {
        let amount = elimination.amount.max(0);
        let direction = elimination.direction.trim().to_ascii_uppercase();
        if direction == "DEDUCT" {
            elimination_total += amount;
        } else if direction == "ADD" {
            elimination_total -= amount;
        }
        insert_consolidation_elimination(pool, tenant, by_id, elimination, &direction).await?;
        elimination_details.push(json!({
            "elimination_type": elimination.elimination_type,
            "direction": direction,
            "amount": amount,
            "description": elimination.description
        }));
    }
    let consolidated_tax_base = (entity_income - elimination_total).max(0);
    let consolidated_tax = calculate_corporate_tax(consolidated_tax_base, rates);
    let mut entity_details = Vec::new();
    for entity in &entities {
        let ratio_bps = if entity_income > 0 {
            ((entity.taxable_income.max(0) as i128) * 10_000 / i128::from(entity_income)) as i64
        } else {
            0
        };
        let allocated_tax = amount_by_bps(consolidated_tax, ratio_bps);
        insert_consolidated_entity(pool, tenant, by_id, entity, allocated_tax).await?;
        entity_details.push(json!({
            "entity_code": entity.entity_code,
            "entity_name": entity.entity_name,
            "ownership_bps": entity.ownership_bps,
            "taxable_income": entity.taxable_income,
            "allocated_tax": allocated_tax
        }));
    }
    let items = vec![PreparedIncomeItem {
        section: "CONSOLIDATED_TAX".to_string(),
        item_code: "B17_CONSOLIDATED_TAX_BASE".to_string(),
        item_name: "Consolidated tax base after eliminations".to_string(),
        amount: consolidated_tax_base,
        direction: "INFO".to_string(),
        disposition: "INTERNAL".to_string(),
        law_ref: Some("Consolidated tax rule".to_string()),
        metadata: json!({ "entity_count": entities.len(), "elimination_total": elimination_total }),
    }];
    Ok((
        items,
        consolidated_tax_base,
        json!({
            "entity_count": entities.len(),
            "entity_income": entity_income,
            "elimination_total": elimination_total,
            "consolidated_tax_base": consolidated_tax_base,
            "consolidated_tax": consolidated_tax,
            "entities": entity_details,
            "eliminations": elimination_details
        }),
    ))
}

async fn resolve_accounting_income(pool: &PgPool, tenant: &TenantRef, by_id: i64) -> Result<i64> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT COUNT(*) AS line_count,
               COALESCE(SUM(CASE WHEN l.debit_credit = 'CREDIT' THEN l.amount ELSE -l.amount END), 0)::BIGINT AS accounting_income
        FROM {schema}.financial_statements f
        JOIN {schema}.fs_lines l ON l.fs_id = f.fs_id
        WHERE f.by_id = $1
          AND (
              UPPER(COALESCE(l.standard_account_code, '')) IN ('NET_INCOME', 'STD_NET_INCOME', 'ACCOUNTING_INCOME')
              OR UPPER(l.account_code) IN ('NET_INCOME', 'ACCOUNTING_INCOME')
              OR UPPER(l.account_name) LIKE '%NET INCOME%'
              OR l.account_name LIKE '%당기순이익%'
          )
        "#
    );
    let row = sqlx::query(&sql)
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("failed to resolve accounting income")?;
    if row.get::<i64, _>("line_count") == 0 {
        return Err(anyhow!(
            "accounting_income is required when financial statements do not contain NET_INCOME"
        ));
    }
    Ok(row.get::<i64, _>("accounting_income"))
}

async fn clear_income_adjustment(pool: &PgPool, tenant: &TenantRef, by_id: i64) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    archive_adjustment_items(pool, tenant, by_id, "B1", "RECALCULATE_CLEAR").await?;
    let reserve_sql =
        format!("DELETE FROM {schema}.reserves WHERE by_id = $1 AND source_module = 'B1'");
    sqlx::query(&reserve_sql).bind(by_id).execute(pool).await?;
    let item_sql =
        format!("DELETE FROM {schema}.adjustment_items WHERE by_id = $1 AND source_module = 'B1'");
    sqlx::query(&item_sql).bind(by_id).execute(pool).await?;
    let adjustment_sql = format!(
        "DELETE FROM {schema}.tax_adjustments WHERE by_id = $1 AND adj_category = 'B1_INCOME'"
    );
    sqlx::query(&adjustment_sql)
        .bind(by_id)
        .execute(pool)
        .await
        .context("failed to clear B-1 adjustment")?;
    Ok(())
}

async fn clear_module_adjustment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: &str,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    archive_adjustment_items(pool, tenant, by_id, module_code, "RECALCULATE_CLEAR").await?;
    let reserve_sql =
        format!("DELETE FROM {schema}.reserves WHERE by_id = $1 AND source_module = $2");
    sqlx::query(&reserve_sql)
        .bind(by_id)
        .bind(module_code)
        .execute(pool)
        .await?;
    let item_sql =
        format!("DELETE FROM {schema}.adjustment_items WHERE by_id = $1 AND source_module = $2");
    sqlx::query(&item_sql)
        .bind(by_id)
        .bind(module_code)
        .execute(pool)
        .await?;
    let adjustment_sql = format!(
        "DELETE FROM {schema}.tax_adjustments WHERE by_id = $1 AND metadata->>'module' = $2"
    );
    sqlx::query(&adjustment_sql)
        .bind(by_id)
        .bind(module_code)
        .execute(pool)
        .await
        .context("failed to clear asset based adjustment")?;
    Ok(())
}

async fn clear_transaction_adjustment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: &str,
) -> Result<()> {
    clear_module_adjustment(pool, tenant, by_id, module_code).await?;
    if module_code == "B2" {
        let schema = quote_ident(&tenant.schema_name)?;
        let sql = format!("DELETE FROM {schema}.donation_carryforwards WHERE by_id = $1");
        sqlx::query(&sql).bind(by_id).execute(pool).await?;
    }
    Ok(())
}

async fn clear_evaluation_adjustment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: &str,
) -> Result<()> {
    clear_module_adjustment(pool, tenant, by_id, module_code).await?;
    if matches!(module_code, "B7" | "B8") {
        let schema = quote_ident(&tenant.schema_name)?;
        sqlx::query(&format!(
            "DELETE FROM {schema}.valuation_positions WHERE by_id = $1 AND module_code = $2"
        ))
        .bind(by_id)
        .bind(module_code)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn clear_tax_amount_adjustment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: &str,
) -> Result<()> {
    clear_module_adjustment(pool, tenant, by_id, module_code).await?;
    let schema = quote_ident(&tenant.schema_name)?;
    match module_code {
        "B12" => {
            sqlx::query(&format!(
                "DELETE FROM {schema}.tax_credit_claims WHERE by_id = $1"
            ))
            .bind(by_id)
            .execute(pool)
            .await?;
        }
        "B13" => {
            sqlx::query(&format!(
                "DELETE FROM {schema}.minimum_tax_results WHERE by_id = $1"
            ))
            .bind(by_id)
            .execute(pool)
            .await?;
        }
        "B14" => {
            sqlx::query(&format!(
                "DELETE FROM {schema}.penalty_tax_items WHERE by_id = $1"
            ))
            .bind(by_id)
            .execute(pool)
            .await?;
        }
        _ => {}
    }
    Ok(())
}

async fn clear_special_adjustment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: &str,
) -> Result<()> {
    clear_module_adjustment(pool, tenant, by_id, module_code).await?;
    let schema = quote_ident(&tenant.schema_name)?;
    match module_code {
        "B16" => {
            sqlx::query(&format!(
                "DELETE FROM {schema}.foreign_income_items WHERE by_id = $1"
            ))
            .bind(by_id)
            .execute(pool)
            .await?;
        }
        "B17" => {
            sqlx::query(&format!(
                "DELETE FROM {schema}.consolidated_entities WHERE by_id = $1"
            ))
            .bind(by_id)
            .execute(pool)
            .await?;
            sqlx::query(&format!(
                "DELETE FROM {schema}.consolidation_eliminations WHERE by_id = $1"
            ))
            .bind(by_id)
            .execute(pool)
            .await?;
        }
        _ => {}
    }
    Ok(())
}

fn amount_by_bps(amount: i64, bps: i64) -> i64 {
    ((amount.max(0) as i128) * i128::from(bps.max(0)) / 10_000) as i64
}

async fn tax_limit_amount(
    pool: &PgPool,
    law_version_id: i64,
    item_code: &str,
    default_amount: i64,
) -> Result<i64> {
    Ok(sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT amount
        FROM tax_limits
        WHERE law_version_id = $1 AND item_code = $2
        ORDER BY effective_from DESC, tax_limit_id DESC
        LIMIT 1
        "#,
    )
    .bind(law_version_id)
    .bind(item_code)
    .fetch_optional(pool)
    .await
    .context("failed to load tax limit")?
    .flatten()
    .unwrap_or(default_amount))
}

async fn load_transaction_rows(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    category: &str,
) -> Result<Vec<TransactionRow>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT COALESCE(description, '') AS description,
               GREATEST(amount, 0)::BIGINT AS amount,
               COALESCE(evidence_type, '') AS evidence_type
        FROM {schema}.transactions
        WHERE by_id = $1 AND UPPER(category) = $2
        ORDER BY tx_date, transaction_id
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(by_id)
        .bind(category)
        .fetch_all(pool)
        .await
        .context("failed to load transaction rows")?;
    Ok(rows
        .into_iter()
        .map(|row| TransactionRow {
            description: row.get("description"),
            amount: row.get("amount"),
            evidence_type: row.get("evidence_type"),
        })
        .collect())
}

fn classify_donation_type(description: &str) -> &'static str {
    let normalized = description.to_ascii_uppercase();
    if normalized.contains("SPECIAL")
        || description.contains("특례")
        || description.contains("법정")
        || normalized.contains("STATUTORY")
    {
        "SPECIAL"
    } else {
        "GENERAL"
    }
}

fn classify_interest_type(description: &str) -> String {
    let normalized = description
        .trim()
        .to_ascii_uppercase()
        .replace([' ', '-'], "_");
    if normalized.contains("UNKNOWN_CREDITOR") || description.contains("채권자불분명") {
        "UNKNOWN_CREDITOR".to_string()
    } else if normalized.contains("UNKNOWN_RECIPIENT") || description.contains("수령자불분명")
    {
        "UNKNOWN_RECIPIENT".to_string()
    } else if normalized.contains("CONSTRUCTION") || description.contains("건설자금") {
        "CONSTRUCTION".to_string()
    } else if normalized.contains("NON_BUSINESS") || description.contains("업무무관") {
        "NON_BUSINESS".to_string()
    } else {
        "GENERAL".to_string()
    }
}

fn is_card_evidence(evidence_type: &str) -> bool {
    matches!(
        evidence_type.trim().to_ascii_uppercase().as_str(),
        "CARD" | "CREDIT_CARD" | "CORPORATE_CARD" | "CHECK_CARD"
    )
}

async fn mark_expired_donation_carryforwards(
    pool: &PgPool,
    tenant: &TenantRef,
    customer_id: i64,
    year_label: i32,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.donation_carryforwards cf
        SET expired_amount = remaining_amount,
            remaining_amount = 0,
            updated_at = NOW()
        FROM {schema}.business_years bys
        WHERE cf.by_id = bys.by_id
          AND bys.customer_id = $1
          AND cf.expires_year < $2
          AND cf.remaining_amount > 0
        "#
    );
    sqlx::query(&sql)
        .bind(customer_id)
        .bind(year_label)
        .execute(pool)
        .await
        .context("failed to expire donation carryforwards")?;
    Ok(())
}

async fn allocate_donation_carryforwards(
    pool: &PgPool,
    tenant: &TenantRef,
    customer_id: i64,
    year_label: i32,
    donation_type: &str,
    available_limit: i64,
) -> Result<(i64, Vec<Value>)> {
    let mut remaining_limit = available_limit.max(0);
    if remaining_limit <= 0 {
        return Ok((0, Vec::new()));
    }
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT cf.carryforward_id, cf.source_year, cf.remaining_amount, cf.expires_year
        FROM {schema}.donation_carryforwards cf
        JOIN {schema}.business_years bys ON bys.by_id = cf.by_id
        WHERE bys.customer_id = $1
          AND bys.year_label < $2
          AND cf.donation_type = $3
          AND cf.expires_year >= $2
          AND cf.remaining_amount > 0
        ORDER BY cf.expires_year, cf.source_year, cf.carryforward_id
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(customer_id)
        .bind(year_label)
        .bind(donation_type)
        .fetch_all(pool)
        .await
        .context("failed to load donation carryforwards")?;
    let mut used_total = 0;
    let mut allocations = Vec::new();
    for row in rows {
        if remaining_limit <= 0 {
            break;
        }
        let carryforward_id = row.get::<i64, _>("carryforward_id");
        let source_year = row.get::<i32, _>("source_year");
        let remaining_amount = row.get::<i64, _>("remaining_amount");
        let expires_year = row.get::<i32, _>("expires_year");
        let used = remaining_amount.min(remaining_limit);
        let update_sql = format!(
            r#"
            UPDATE {schema}.donation_carryforwards
            SET used_amount = used_amount + $1,
                remaining_amount = remaining_amount - $1,
                updated_at = NOW()
            WHERE carryforward_id = $2
            "#
        );
        sqlx::query(&update_sql)
            .bind(used)
            .bind(carryforward_id)
            .execute(pool)
            .await
            .context("failed to allocate donation carryforward")?;
        remaining_limit -= used;
        used_total += used;
        allocations.push(json!({
            "carryforward_id": carryforward_id,
            "source_year": source_year,
            "used": used,
            "expires_year": expires_year
        }));
    }
    Ok((used_total, allocations))
}

async fn insert_donation_carryforward(
    pool: &PgPool,
    tenant: &TenantRef,
    new_carryforward: NewDonationCarryforward<'_>,
) -> Result<DonationCarryforward> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.donation_carryforwards (
            by_id, source_year, donation_type, original_amount, remaining_amount,
            expires_year, adjustment_item_id
        )
        VALUES ($1, $2, $3, $4, $4, $5, $6)
        RETURNING carryforward_id, by_id, source_year, donation_type, original_amount,
                  used_amount, expired_amount, remaining_amount, expires_year,
                  adjustment_item_id, created_at, updated_at
        "#
    );
    sqlx::query_as::<_, DonationCarryforward>(&sql)
        .bind(new_carryforward.by_id)
        .bind(new_carryforward.source_year)
        .bind(new_carryforward.donation_type)
        .bind(new_carryforward.amount)
        .bind(new_carryforward.expires_year)
        .bind(new_carryforward.adjustment_item_id)
        .fetch_one(pool)
        .await
        .context("failed to insert donation carryforward")
}

async fn list_donation_carryforwards(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<DonationCarryforward>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT carryforward_id, by_id, source_year, donation_type, original_amount,
               used_amount, expired_amount, remaining_amount, expires_year,
               adjustment_item_id, created_at, updated_at
        FROM {schema}.donation_carryforwards
        WHERE by_id = $1
        ORDER BY source_year, donation_type, carryforward_id
        "#
    );
    sqlx::query_as::<_, DonationCarryforward>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list donation carryforwards")
}

async fn resolve_entertainment_revenue(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    gross_revenue: Option<i64>,
    revenue_breakdowns: Option<Vec<RevenueBreakdownInput>>,
) -> Result<(i64, Vec<Value>)> {
    let schema = quote_ident(&tenant.schema_name)?;
    if let Some(lines) = revenue_breakdowns {
        let delete_sql =
            format!("DELETE FROM {schema}.entertainment_revenue_breakdowns WHERE by_id = $1");
        sqlx::query(&delete_sql).bind(by_id).execute(pool).await?;
        let insert_sql = format!(
            r#"
            INSERT INTO {schema}.entertainment_revenue_breakdowns (
                by_id, revenue_category, amount
            )
            VALUES ($1, $2, $3)
            "#
        );
        for line in lines.iter().filter(|line| line.amount > 0) {
            sqlx::query(&insert_sql)
                .bind(by_id)
                .bind(line.revenue_category.trim())
                .bind(line.amount)
                .execute(pool)
                .await
                .context("failed to insert entertainment revenue breakdown")?;
        }
    }
    let sql = format!(
        r#"
        SELECT revenue_category, amount
        FROM {schema}.entertainment_revenue_breakdowns
        WHERE by_id = $1
        ORDER BY revenue_breakdown_id
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to load entertainment revenue breakdowns")?;
    let line_values = rows
        .iter()
        .map(|row| {
            json!({
                "revenue_category": row.get::<String, _>("revenue_category"),
                "amount": row.get::<i64, _>("amount")
            })
        })
        .collect::<Vec<_>>();
    let breakdown_total = rows
        .iter()
        .map(|row| row.get::<i64, _>("amount").max(0))
        .sum::<i64>();
    Ok((gross_revenue.unwrap_or(breakdown_total).max(0), line_values))
}

async fn insert_loan_interest_fact(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    weighted_average_loan_balance: i64,
    weighted_average_interest_rate_bps: i32,
    deemed_interest: i64,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let delete_sql = format!("DELETE FROM {schema}.loan_interest_facts WHERE by_id = $1");
    sqlx::query(&delete_sql).bind(by_id).execute(pool).await?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.loan_interest_facts (
            by_id, weighted_average_loan_balance, weighted_average_interest_rate_bps, deemed_interest
        )
        VALUES ($1, $2, $3, $4)
        "#
    );
    sqlx::query(&sql)
        .bind(by_id)
        .bind(weighted_average_loan_balance)
        .bind(weighted_average_interest_rate_bps)
        .bind(deemed_interest)
        .execute(pool)
        .await
        .context("failed to insert loan interest fact")?;
    Ok(())
}

async fn insert_valuation_position(
    pool: &PgPool,
    tenant: &TenantRef,
    new_position: NewValuationPosition<'_>,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let input = new_position.input;
    let sql = format!(
        r#"
        INSERT INTO {schema}.valuation_positions (
            by_id, module_code, item_code, item_name, position_type, monetary,
            valuation_method, book_amount, tax_amount, adjustment_amount, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#
    );
    sqlx::query(&sql)
        .bind(new_position.by_id)
        .bind(new_position.module_code)
        .bind(input.item_code.trim().to_ascii_uppercase())
        .bind(input.item_name.trim())
        .bind(input.position_type.as_deref().unwrap_or("GENERAL"))
        .bind(input.monetary.unwrap_or(true))
        .bind(new_position.valuation_method)
        .bind(input.book_amount)
        .bind(new_position.tax_amount)
        .bind(new_position.adjustment_amount)
        .bind(json!({
            "foreign_amount": input.foreign_amount,
            "book_rate": input.book_rate,
            "closing_rate": input.closing_rate
        }))
        .execute(pool)
        .await
        .context("failed to insert valuation position")?;
    Ok(())
}

async fn upsert_loss_carryforwards(
    pool: &PgPool,
    tenant: &TenantRef,
    customer_id: i64,
    losses: &[LossCarryforwardInput],
    current_year: i32,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.carryforward_loss (
            customer_id, origin_year, original_amount, remaining_amount, expires_year
        )
        VALUES ($1, $2, $3, $4, $5)
        "#
    );
    for loss in losses {
        let original = loss.original_amount.max(0);
        let remaining = loss.remaining_amount.unwrap_or(original).max(0);
        let expires_year = loss.expires_year.unwrap_or(loss.origin_year + 15);
        if original == 0 || loss.origin_year > current_year {
            continue;
        }
        sqlx::query(&sql)
            .bind(customer_id)
            .bind(loss.origin_year)
            .bind(original)
            .bind(remaining.min(original))
            .bind(expires_year)
            .execute(pool)
            .await
            .context("failed to insert loss carryforward")?;
    }
    Ok(())
}

async fn expire_loss_carryforwards(
    pool: &PgPool,
    tenant: &TenantRef,
    customer_id: i64,
    year_label: i32,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.carryforward_loss
        SET expired_amount = expired_amount + remaining_amount,
            remaining_amount = 0,
            updated_at = NOW()
        WHERE customer_id = $1
          AND expires_year < $2
          AND remaining_amount > 0
        "#
    );
    sqlx::query(&sql)
        .bind(customer_id)
        .bind(year_label)
        .execute(pool)
        .await
        .context("failed to expire loss carryforwards")?;
    Ok(())
}

async fn allocate_loss_carryforwards(
    pool: &PgPool,
    tenant: &TenantRef,
    customer_id: i64,
    year_label: i32,
    deduction_limit: i64,
) -> Result<(i64, Vec<Value>)> {
    let schema = quote_ident(&tenant.schema_name)?;
    let rows = sqlx::query(&format!(
        r#"
        SELECT loss_id, origin_year, remaining_amount, expires_year
        FROM {schema}.carryforward_loss
        WHERE customer_id = $1
          AND origin_year < $2
          AND expires_year >= $2
          AND remaining_amount > 0
        ORDER BY expires_year, origin_year, loss_id
        "#
    ))
    .bind(customer_id)
    .bind(year_label)
    .fetch_all(pool)
    .await
    .context("failed to load loss carryforwards")?;
    let mut remaining_limit = deduction_limit.max(0);
    let mut used_total = 0;
    let mut allocations = Vec::new();
    for row in rows {
        if remaining_limit <= 0 {
            break;
        }
        let loss_id = row.get::<i64, _>("loss_id");
        let origin_year = row.get::<i32, _>("origin_year");
        let remaining_amount = row.get::<i64, _>("remaining_amount");
        let expires_year = row.get::<i32, _>("expires_year");
        let used = remaining_amount.min(remaining_limit);
        sqlx::query(&format!(
            r#"
            UPDATE {schema}.carryforward_loss
            SET used_amount = used_amount + $1,
                remaining_amount = remaining_amount - $1,
                updated_at = NOW()
            WHERE loss_id = $2
            "#
        ))
        .bind(used)
        .bind(loss_id)
        .execute(pool)
        .await
        .context("failed to allocate loss carryforward")?;
        remaining_limit -= used;
        used_total += used;
        allocations.push(json!({
            "loss_id": loss_id,
            "origin_year": origin_year,
            "used": used,
            "expires_year": expires_year
        }));
    }
    Ok((used_total, allocations))
}

async fn list_loss_carryforwards(
    pool: &PgPool,
    tenant: &TenantRef,
    customer_id: i64,
) -> Result<Vec<LossCarryforwardRecord>> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query_as::<_, LossCarryforwardRecord>(&format!(
        r#"
        SELECT loss_id, customer_id, origin_year, original_amount, used_amount,
               expired_amount, remaining_amount, expires_year, created_at, updated_at
        FROM {schema}.carryforward_loss
        WHERE customer_id = $1
        ORDER BY expires_year, origin_year, loss_id
        "#
    ))
    .bind(customer_id)
    .fetch_all(pool)
    .await
    .context("failed to list loss carryforwards")
}

async fn load_customer_is_sme(pool: &PgPool, tenant: &TenantRef, customer_id: i64) -> Result<bool> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query_scalar::<_, bool>(&format!(
        "SELECT is_sme FROM {schema}.customers WHERE customer_id = $1"
    ))
    .bind(customer_id)
    .fetch_one(pool)
    .await
    .context("failed to load customer SME flag")
}

async fn replace_capital_changes(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    changes: &[CapitalChangeInput],
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query(&format!(
        "DELETE FROM {schema}.capital_changes WHERE by_id = $1"
    ))
    .bind(by_id)
    .execute(pool)
    .await?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.capital_changes (
            by_id, change_date, change_type, amount, description
        )
        VALUES ($1, $2, $3, $4, $5)
        "#
    );
    for change in changes.iter().filter(|change| change.amount != 0) {
        sqlx::query(&sql)
            .bind(by_id)
            .bind(change.change_date)
            .bind(change.change_type.trim().to_ascii_uppercase())
            .bind(change.amount)
            .bind(change.description.as_deref())
            .execute(pool)
            .await
            .context("failed to insert capital change")?;
    }
    Ok(())
}

async fn list_capital_changes(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<CapitalChange>> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query_as::<_, CapitalChange>(&format!(
        r#"
        SELECT capital_change_id, by_id, change_date, change_type, amount, description, created_at
        FROM {schema}.capital_changes
        WHERE by_id = $1
        ORDER BY change_date, capital_change_id
        "#
    ))
    .bind(by_id)
    .fetch_all(pool)
    .await
    .context("failed to list capital changes")
}

async fn aggregate_reserves(pool: &PgPool, tenant: &TenantRef, by_id: i64) -> Result<Vec<Value>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let rows = sqlx::query(&format!(
        r#"
        SELECT source_module, reserve_code, direction,
               COALESCE(SUM(amount), 0)::BIGINT AS amount,
               MAX(carryforward_to) AS carryforward_to
        FROM {schema}.reserves
        WHERE by_id = $1
          AND source_module <> 'B15'
        GROUP BY source_module, reserve_code, direction
        ORDER BY source_module, reserve_code
        "#
    ))
    .bind(by_id)
    .fetch_all(pool)
    .await
    .context("failed to aggregate reserves")?;
    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "source_module": row.get::<String, _>("source_module"),
                "reserve_code": row.get::<String, _>("reserve_code"),
                "direction": row.get::<String, _>("direction"),
                "amount": row.get::<i64, _>("amount"),
                "carryforward_to": row.get::<Option<i32>, _>("carryforward_to")
            })
        })
        .collect())
}

async fn credit_rate_bps(pool: &PgPool, law_version_id: i64, credit_type: &str) -> Result<i64> {
    let item_code = match credit_type {
        "RND" | "R_AND_D" => "RND_CREDIT_BPS",
        "INVESTMENT" | "INTEGRATED_INVESTMENT" => "INTEGRATED_INVESTMENT_CREDIT_BPS",
        "FOREIGN_TAX" => "FOREIGN_TAX_CREDIT_LIMIT_BPS",
        "DISASTER" => "DISASTER_CREDIT_BPS",
        "SME_SPECIAL" => "SME_SPECIAL_REDUCTION_BPS",
        "STARTUP" => "STARTUP_REDUCTION_BPS",
        _ => "RND_CREDIT_BPS",
    };
    tax_limit_amount(pool, law_version_id, item_code, 0).await
}

async fn insert_tax_credit_claim(
    pool: &PgPool,
    tenant: &TenantRef,
    claim: NewTaxCreditClaim<'_>,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query(&format!(
        r#"
        INSERT INTO {schema}.tax_credit_claims (
            by_id, credit_type, base_amount, rate_bps, requested_amount, allowed_amount
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    ))
    .bind(claim.by_id)
    .bind(claim.credit_type)
    .bind(claim.base_amount)
    .bind(claim.rate_bps)
    .bind(claim.requested_amount)
    .bind(claim.allowed_amount)
    .execute(pool)
    .await
    .context("failed to insert tax credit claim")?;
    Ok(())
}

async fn insert_minimum_tax_result(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    tax_base: i64,
    regular_tax: i64,
    minimum_tax: i64,
    additional_tax: i64,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query(&format!(
        r#"
        INSERT INTO {schema}.minimum_tax_results (
            by_id, tax_base, regular_tax, minimum_tax, additional_tax
        )
        VALUES ($1, $2, $3, $4, $5)
        "#
    ))
    .bind(by_id)
    .bind(tax_base)
    .bind(regular_tax)
    .bind(minimum_tax)
    .bind(additional_tax)
    .execute(pool)
    .await
    .context("failed to insert minimum tax result")?;
    Ok(())
}

async fn insert_penalty_tax_item(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    penalty_type: &str,
    penalty: &PenaltyTaxInput,
    penalty_amount: i64,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query(&format!(
        r#"
        INSERT INTO {schema}.penalty_tax_items (
            by_id, penalty_type, tax_base, rate_bps, days_late, reduction_bps, penalty_amount
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#
    ))
    .bind(by_id)
    .bind(penalty_type)
    .bind(penalty.tax_base)
    .bind(penalty.rate_bps)
    .bind(penalty.days_late)
    .bind(penalty.reduction_bps.unwrap_or(0).clamp(0, 10_000))
    .bind(penalty_amount)
    .execute(pool)
    .await
    .context("failed to insert penalty tax item")?;
    Ok(())
}

fn normalize_foreign_income_type(income_type: &str) -> Result<String> {
    let normalized = income_type.trim().to_ascii_uppercase();
    let allowed = [
        "INTEREST",
        "DIVIDEND",
        "ROYALTY",
        "SERVICE",
        "REAL_ESTATE",
        "CAPITAL_GAIN",
    ];
    if allowed.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(anyhow!("invalid foreign income type"))
    }
}

async fn insert_foreign_income_item(
    pool: &PgPool,
    tenant: &TenantRef,
    item: NewForeignIncomeItem<'_>,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query(&format!(
        r#"
        INSERT INTO {schema}.foreign_income_items (
            by_id, income_type, gross_amount, attributable_expense,
            pe_allocation_bps, allocated_income, withholding_tax
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#
    ))
    .bind(item.by_id)
    .bind(item.income_type)
    .bind(item.gross_amount)
    .bind(item.attributable_expense)
    .bind(item.pe_allocation_bps)
    .bind(item.allocated_income)
    .bind(item.withholding_tax)
    .execute(pool)
    .await
    .context("failed to insert foreign income item")?;
    Ok(())
}

async fn insert_consolidated_entity(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    entity: &ConsolidatedEntityInput,
    allocated_tax: i64,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query(&format!(
        r#"
        INSERT INTO {schema}.consolidated_entities (
            by_id, entity_code, entity_name, ownership_bps, taxable_income,
            standalone_tax, allocated_tax
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#
    ))
    .bind(by_id)
    .bind(entity.entity_code.trim().to_ascii_uppercase())
    .bind(entity.entity_name.trim())
    .bind(entity.ownership_bps)
    .bind(entity.taxable_income)
    .bind(entity.standalone_tax.unwrap_or(0).max(0))
    .bind(allocated_tax)
    .execute(pool)
    .await
    .context("failed to insert consolidated entity")?;
    Ok(())
}

async fn insert_consolidation_elimination(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    elimination: &ConsolidationEliminationInput,
    direction: &str,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query(&format!(
        r#"
        INSERT INTO {schema}.consolidation_eliminations (
            by_id, elimination_type, amount, direction, description
        )
        VALUES ($1, $2, $3, $4, $5)
        "#
    ))
    .bind(by_id)
    .bind(elimination.elimination_type.trim().to_ascii_uppercase())
    .bind(elimination.amount.max(0))
    .bind(direction)
    .bind(elimination.description.as_deref())
    .execute(pool)
    .await
    .context("failed to insert consolidation elimination")?;
    Ok(())
}

async fn depreciation_tax_life(pool: &PgPool, law_version_id: i64, category: &str) -> Result<i32> {
    if category.to_ascii_uppercase().contains("VEHICLE") {
        return Ok(5);
    }
    let lookup = if category.to_ascii_uppercase().contains("MACH") {
        "MACHINE"
    } else {
        category
    };
    let life = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT amount
        FROM tax_limits
        WHERE law_version_id = $1
          AND metadata->>'category' = 'DEPRECIATION_LIFE'
          AND (metadata->>'asset_category' = $2 OR item_code = $3)
        ORDER BY tax_limit_id DESC
        LIMIT 1
        "#,
    )
    .bind(law_version_id)
    .bind(lookup)
    .bind(format!("{}_USEFUL_LIFE_YEARS", lookup.to_ascii_uppercase()))
    .fetch_optional(pool)
    .await
    .context("failed to load depreciation life")?
    .flatten()
    .unwrap_or(5);
    Ok(i32::try_from(life).unwrap_or(5).max(1))
}

async fn insert_depreciation_row(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    asset_id: i64,
    book_amount: i64,
    tax_limit: i64,
    adjustment_amount: i64,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.depreciation (
            asset_id, by_id, book_amount, tax_limit, adjustment_amount
        )
        VALUES ($1, $2, $3, $4, $5)
        "#
    );
    sqlx::query(&sql)
        .bind(asset_id)
        .bind(by_id)
        .bind(book_amount)
        .bind(tax_limit)
        .bind(adjustment_amount)
        .execute(pool)
        .await
        .context("failed to insert depreciation result")?;
    Ok(())
}

async fn vehicle_business_use_bps(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    asset_id: i64,
) -> Result<Option<i32>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT COALESCE(SUM(total_distance_km), 0)::DOUBLE PRECISION AS total_km,
               COALESCE(SUM(business_distance_km), 0)::DOUBLE PRECISION AS business_km
        FROM {schema}.vehicle_usage_logs
        WHERE by_id = $1 AND asset_id = $2
        "#
    );
    let row = sqlx::query(&sql)
        .bind(by_id)
        .bind(asset_id)
        .fetch_one(pool)
        .await
        .context("failed to summarize vehicle usage")?;
    let total = row.get::<f64, _>("total_km");
    if total <= 0.0 {
        return Ok(None);
    }
    let business = row.get::<f64, _>("business_km").clamp(0.0, total);
    Ok(Some(((business / total) * 10_000.0).round() as i32))
}

async fn insert_tax_adjustment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    adjustment: NewAdjustment,
) -> Result<i64> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.tax_adjustments (
            by_id, adj_category, adj_code, amount, direction, description, snapshot_id, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING adjustment_id
        "#
    );
    sqlx::query_scalar::<_, i64>(&sql)
        .bind(by_id)
        .bind(adjustment.category)
        .bind(adjustment.code)
        .bind(adjustment.amount)
        .bind(adjustment.direction)
        .bind(adjustment.description)
        .bind(adjustment.snapshot_id)
        .bind(adjustment.metadata)
        .fetch_one(pool)
        .await
        .context("failed to insert tax adjustment")
}

async fn insert_adjustment_item(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    adjustment_id: i64,
    source_module: &str,
    item: &PreparedIncomeItem,
) -> Result<AdjustmentItem> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.adjustment_items (
            by_id, adjustment_id, section, item_code, item_name, amount,
            direction, disposition, source_module, law_ref, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING adjustment_item_id, by_id, adjustment_id, section, item_code,
                  item_name, amount, direction, disposition, source_module, law_ref,
                  metadata, created_at
        "#
    );
    let saved = sqlx::query_as::<_, AdjustmentItem>(&sql)
        .bind(by_id)
        .bind(adjustment_id)
        .bind(&item.section)
        .bind(&item.item_code)
        .bind(&item.item_name)
        .bind(item.amount)
        .bind(&item.direction)
        .bind(&item.disposition)
        .bind(source_module)
        .bind(&item.law_ref)
        .bind(&item.metadata)
        .fetch_one(pool)
        .await
        .context("failed to insert B-1 adjustment item")?;
    record_adjustment_item_history(
        pool,
        tenant,
        AdjustmentItemHistory {
            adjustment_item_id: saved.adjustment_item_id,
            by_id,
            source_module,
            action: "CREATE",
            old_data: None,
            new_data: Some(json!(&saved)),
            changed_by: "system",
        },
    )
    .await?;
    Ok(saved)
}

async fn archive_adjustment_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    source_module: &str,
    action: &str,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.adjustment_items_history (
            adjustment_item_id, by_id, source_module, action, old_data, changed_by
        )
        SELECT adjustment_item_id, by_id, source_module, $3, to_jsonb(item), 'system'
        FROM {schema}.adjustment_items item
        WHERE by_id = $1 AND source_module = $2
        "#
    );
    sqlx::query(&sql)
        .bind(by_id)
        .bind(source_module)
        .bind(action)
        .execute(pool)
        .await
        .context("failed to archive adjustment item history")?;
    Ok(())
}

struct AdjustmentItemHistory<'a> {
    adjustment_item_id: i64,
    by_id: i64,
    source_module: &'a str,
    action: &'a str,
    old_data: Option<Value>,
    new_data: Option<Value>,
    changed_by: &'a str,
}

async fn record_adjustment_item_history(
    pool: &PgPool,
    tenant: &TenantRef,
    history: AdjustmentItemHistory<'_>,
) -> Result<()> {
    let AdjustmentItemHistory {
        adjustment_item_id,
        by_id,
        source_module,
        action,
        old_data,
        new_data,
        changed_by,
    } = history;
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.adjustment_items_history (
            adjustment_item_id, by_id, source_module, action, old_data, new_data, changed_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#
    );
    sqlx::query(&sql)
        .bind(adjustment_item_id)
        .bind(by_id)
        .bind(source_module)
        .bind(action)
        .bind(old_data)
        .bind(new_data)
        .bind(changed_by)
        .execute(pool)
        .await
        .context("failed to insert adjustment item history")?;
    Ok(())
}

async fn insert_reserve(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    adjustment_id: i64,
    source_module: &str,
    item: &PreparedIncomeItem,
    carryforward_to: i32,
) -> Result<ReserveRecord> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.reserves (
            by_id, adjustment_id, reserve_code, amount, direction, carryforward_to, source_module
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING reserve_id, by_id, adjustment_id, reserve_code, amount,
                  direction, carryforward_to, source_module, created_at
        "#
    );
    sqlx::query_as::<_, ReserveRecord>(&sql)
        .bind(by_id)
        .bind(adjustment_id)
        .bind(&item.item_code)
        .bind(item.amount)
        .bind(&item.direction)
        .bind(carryforward_to)
        .bind(source_module)
        .fetch_one(pool)
        .await
        .context("failed to insert reserve")
}

async fn persist_adjustments(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    result: &CalculationResult,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.tax_adjustments (
            by_id, adj_category, adj_code, amount, direction, description, snapshot_id, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#
    );

    let rows = [
        (
            "INCOME",
            "ACCOUNTING_INCOME",
            result.accounting_income,
            "INFO",
            "회계상 당기순이익",
        ),
        (
            "INCOME",
            "ADDBACKS",
            result.addbacks,
            "ADD",
            "익금산입/손금불산입 합계",
        ),
        (
            "INCOME",
            "DEDUCTIONS",
            result.deductions,
            "DEDUCT",
            "손금산입/익금불산입 합계",
        ),
        (
            "TAX_BASE",
            "TAXABLE_INCOME",
            result.taxable_income,
            "INFO",
            "과세표준",
        ),
        (
            "TAX",
            "CORPORATE_TAX",
            result.corporate_tax,
            "INFO",
            "법인세",
        ),
        (
            "TAX",
            "LOCAL_INCOME_TAX",
            result.local_income_tax,
            "INFO",
            "지방소득세",
        ),
        (
            "TAX",
            "TAX_CREDITS",
            result.tax_credits,
            "DEDUCT",
            "세액공제",
        ),
        (
            "TAX",
            "TOTAL_TAX_DUE",
            result.total_tax_due,
            "INFO",
            "총 납부세액",
        ),
    ];

    for (category, code, amount, direction, description) in rows {
        sqlx::query(&sql)
            .bind(by_id)
            .bind(category)
            .bind(code)
            .bind(amount)
            .bind(direction)
            .bind(description)
            .bind(result.snapshot_id)
            .bind(&result.details)
            .execute(pool)
            .await
            .context("failed to persist tax adjustment")?;
    }

    Ok(())
}

pub async fn list_adjustments(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<TaxAdjustment>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT adjustment_id, by_id, adj_category, adj_code, amount, direction,
               description, snapshot_id, metadata, status, created_at
        FROM {schema}.tax_adjustments
        WHERE by_id = $1
        ORDER BY created_at DESC, adjustment_id DESC
        "#
    );

    sqlx::query_as::<_, TaxAdjustment>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list adjustments")
}

pub async fn list_income_adjustment_items(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<AdjustmentItem>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT adjustment_item_id, by_id, adjustment_id, section, item_code,
               item_name, amount, direction, disposition, source_module, law_ref,
               metadata, created_at
        FROM {schema}.adjustment_items
        WHERE by_id = $1 AND source_module = 'B1'
        ORDER BY section, adjustment_item_id
        "#
    );
    sqlx::query_as::<_, AdjustmentItem>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list B-1 adjustment items")
}

pub async fn list_adjustment_items_by_module(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: &str,
) -> Result<Vec<AdjustmentItem>> {
    let module_code = normalize_any_adjustment_module(module_code)?;
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT adjustment_item_id, by_id, adjustment_id, section, item_code,
               item_name, amount, direction, disposition, source_module, law_ref,
               metadata, created_at
        FROM {schema}.adjustment_items
        WHERE by_id = $1 AND source_module = $2
        ORDER BY section, adjustment_item_id
        "#
    );
    sqlx::query_as::<_, AdjustmentItem>(&sql)
        .bind(by_id)
        .bind(module_code)
        .fetch_all(pool)
        .await
        .context("failed to list asset based adjustment items")
}

pub async fn list_adjustment_item_history(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    module_code: Option<&str>,
) -> Result<Vec<Value>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let module_code = module_code
        .map(normalize_any_adjustment_module)
        .transpose()?;
    let sql = format!(
        r#"
        SELECT history_id, adjustment_item_id, by_id, source_module, action,
               old_data, new_data, changed_by, changed_at
        FROM {schema}.adjustment_items_history
        WHERE by_id = $1
          AND ($2::TEXT IS NULL OR source_module = $2)
        ORDER BY changed_at DESC, history_id DESC
        LIMIT 300
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(by_id)
        .bind(module_code)
        .fetch_all(pool)
        .await
        .context("failed to list adjustment item history")?;
    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "history_id": row.get::<i64, _>("history_id"),
                "adjustment_item_id": row.get::<Option<i64>, _>("adjustment_item_id"),
                "by_id": row.get::<i64, _>("by_id"),
                "source_module": row.get::<String, _>("source_module"),
                "action": row.get::<String, _>("action"),
                "old_data": row.get::<Option<Value>, _>("old_data"),
                "new_data": row.get::<Option<Value>, _>("new_data"),
                "changed_by": row.get::<String, _>("changed_by"),
                "changed_at": row.get::<chrono::DateTime<chrono::Utc>, _>("changed_at")
            })
        })
        .collect())
}

pub async fn create_adjustment_item_attachment(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: CreateAdjustmentAttachmentRequest,
) -> Result<Value> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "adjustment attachments").await?;
    if request.file_name.trim().is_empty() {
        return Err(anyhow!("invalid attachment file_name"));
    }
    let schema = quote_ident(&tenant.schema_name)?;
    sqlx::query_scalar::<_, i64>(&format!(
        "SELECT adjustment_item_id FROM {schema}.adjustment_items WHERE by_id = $1 AND adjustment_item_id = $2"
    ))
    .bind(by_id)
    .bind(request.adjustment_item_id)
    .fetch_one(pool)
    .await
    .context("adjustment item not found")?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.adjustment_item_attachments (
            adjustment_item_id, by_id, file_name, content_type, storage_url, memo, uploaded_by
        )
        VALUES ($1, $2, $3, COALESCE($4, 'application/octet-stream'), $5, $6, COALESCE($7, 'system'))
        RETURNING attachment_id, adjustment_item_id, by_id, file_name, content_type,
                  storage_url, memo, uploaded_by, created_at
        "#
    );
    let row = sqlx::query(&sql)
        .bind(request.adjustment_item_id)
        .bind(by_id)
        .bind(request.file_name.trim())
        .bind(request.content_type)
        .bind(request.storage_url)
        .bind(request.memo)
        .bind(request.uploaded_by)
        .fetch_one(pool)
        .await
        .context("failed to create adjustment item attachment")?;
    Ok(attachment_json(row))
}

pub async fn list_adjustment_item_attachments(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    adjustment_item_id: i64,
) -> Result<Vec<Value>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT attachment_id, adjustment_item_id, by_id, file_name, content_type,
               storage_url, memo, uploaded_by, created_at
        FROM {schema}.adjustment_item_attachments
        WHERE by_id = $1 AND adjustment_item_id = $2
        ORDER BY created_at DESC, attachment_id DESC
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(by_id)
        .bind(adjustment_item_id)
        .fetch_all(pool)
        .await
        .context("failed to list adjustment item attachments")?;
    Ok(rows.into_iter().map(attachment_json).collect())
}

fn attachment_json(row: sqlx::postgres::PgRow) -> Value {
    json!({
        "attachment_id": row.get::<i64, _>("attachment_id"),
        "adjustment_item_id": row.get::<i64, _>("adjustment_item_id"),
        "by_id": row.get::<i64, _>("by_id"),
        "file_name": row.get::<String, _>("file_name"),
        "content_type": row.get::<String, _>("content_type"),
        "storage_url": row.get::<Option<String>, _>("storage_url"),
        "memo": row.get::<Option<String>, _>("memo"),
        "uploaded_by": row.get::<String, _>("uploaded_by"),
        "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at")
    })
}

pub async fn list_reserves(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<ReserveRecord>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT reserve_id, by_id, adjustment_id, reserve_code, amount,
               direction, carryforward_to, source_module, created_at
        FROM {schema}.reserves
        WHERE by_id = $1
        ORDER BY created_at DESC, reserve_id DESC
        "#
    );
    sqlx::query_as::<_, ReserveRecord>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list reserves")
}

pub async fn create_vehicle_usage_log(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: CreateVehicleUsageLogRequest,
) -> Result<VehicleUsageLog> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "vehicle usage").await?;
    if request.total_distance_km < 0.0 || request.business_distance_km < 0.0 {
        return Err(anyhow!("invalid vehicle usage distance"));
    }
    let business = request
        .business_distance_km
        .min(request.total_distance_km)
        .max(0.0);
    let total = request.total_distance_km.max(0.0);
    let bps = if total > 0.0 {
        ((business / total) * 10_000.0).round() as i32
    } else {
        0
    };
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.vehicle_usage_logs (
            by_id, asset_id, usage_month, total_distance_km, business_distance_km, business_use_bps
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (by_id, asset_id, usage_month)
        DO UPDATE SET
            total_distance_km = EXCLUDED.total_distance_km,
            business_distance_km = EXCLUDED.business_distance_km,
            business_use_bps = EXCLUDED.business_use_bps
        RETURNING usage_log_id, by_id, asset_id, usage_month, total_distance_km,
                  business_distance_km, business_use_bps, created_at
        "#
    );
    sqlx::query_as::<_, VehicleUsageLog>(&sql)
        .bind(by_id)
        .bind(request.asset_id)
        .bind(request.usage_month)
        .bind(total)
        .bind(business)
        .bind(bps)
        .fetch_one(pool)
        .await
        .context("failed to upsert vehicle usage log")
}

pub async fn list_vehicle_usage_logs(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<VehicleUsageLog>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT usage_log_id, by_id, asset_id, usage_month, total_distance_km,
               business_distance_km, business_use_bps, created_at
        FROM {schema}.vehicle_usage_logs
        WHERE by_id = $1
        ORDER BY asset_id, usage_month
        "#
    );
    sqlx::query_as::<_, VehicleUsageLog>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list vehicle usage logs")
}

pub async fn generate_form(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    form_code: &str,
) -> Result<FormData> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "forms").await?;
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let snapshot = ensure_law_snapshot(pool, tenant, by_id).await?;
    let form_version_id = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT form_version_id
        FROM form_versions
        WHERE form_code = $1
          AND status IN ('APPROVED', 'ACTIVE')
          AND effective_from <= $2
          AND (effective_to IS NULL OR effective_to >= $3)
        ORDER BY effective_from DESC, form_version_id DESC
        LIMIT 1
        "#,
    )
    .bind(form_code)
    .bind(by.end_date)
    .bind(by.start_date)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("no approved form version for {form_code}"))?;

    let adjustments = list_adjustments(pool, tenant, by_id).await?;
    let summary = summarize_adjustments(&adjustments);
    let mut data_json = build_form_payload(form_code, &summary, snapshot.snapshot_id)?;
    apply_form_relationships(
        pool,
        tenant,
        by_id,
        by.start_date,
        by.end_date,
        form_code,
        &mut data_json,
    )
    .await?;

    let schema = quote_ident(&tenant.schema_name)?;
    let old_data = load_form_optional(pool, tenant, by_id, form_code)
        .await?
        .map(|form| form.data_json);
    let sql = format!(
        r#"
        INSERT INTO {schema}.form_data (by_id, form_code, form_version_id, data_json, snapshot_id)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (by_id, form_code)
        DO UPDATE SET
            form_version_id = EXCLUDED.form_version_id,
            data_json = EXCLUDED.data_json,
            snapshot_id = EXCLUDED.snapshot_id,
            status = 'GENERATED',
            updated_at = NOW()
        RETURNING form_data_id, by_id, form_code, form_version_id, data_json,
                  snapshot_id, status, created_at, updated_at
        "#
    );

    let form = sqlx::query_as::<_, FormData>(&sql)
        .bind(by_id)
        .bind(form_code)
        .bind(form_version_id)
        .bind(data_json)
        .bind(snapshot.snapshot_id)
        .fetch_one(pool)
        .await
        .context("failed to upsert form data")?;
    insert_form_data_history(
        pool,
        tenant,
        FormDataHistoryInsert {
            form: &form,
            change_type: "GENERATE",
            old_data,
            new_data: form.data_json.clone(),
            changed_by: "system",
            reason: Some("form engine generation"),
        },
    )
    .await?;
    Ok(form)
}

pub async fn get_form(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    form_code: &str,
) -> Result<FormData> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT form_data_id, by_id, form_code, form_version_id, data_json,
               snapshot_id, status, created_at, updated_at
        FROM {schema}.form_data
        WHERE by_id = $1 AND form_code = $2
        "#
    );

    sqlx::query_as::<_, FormData>(&sql)
        .bind(by_id)
        .bind(form_code)
        .fetch_one(pool)
        .await
        .context("form data not found")
}

pub async fn update_form_data(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    form_code: &str,
    request: UpdateFormDataRequest,
) -> Result<FormData> {
    tenant::ensure_business_year_editable(pool, tenant, by_id, "forms").await?;
    let current = match load_form_optional(pool, tenant, by_id, form_code).await? {
        Some(form) => form,
        None => generate_form(pool, tenant, by_id, form_code).await?,
    };
    if let Some(expected_updated_at) = request.expected_updated_at.as_ref() {
        if current.updated_at != *expected_updated_at {
            return Err(anyhow!(
                "updated_at conflict: form data was modified by another request"
            ));
        }
    }
    let mut data_json = current.data_json.clone();
    let old_data = data_json.clone();
    let fields = request
        .fields
        .as_object()
        .ok_or_else(|| anyhow!("fields must be a JSON object"))?;
    for (field, value) in fields {
        if field == "_meta" {
            continue;
        }
        set_form_field(&mut data_json, field, value.clone())?;
        set_form_field_meta(
            &mut data_json,
            field,
            "manual",
            Some("user override".to_string()),
            true,
        )?;
    }
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        UPDATE {schema}.form_data
        SET data_json = $3, status = 'MANUAL', updated_at = NOW()
        WHERE by_id = $1
          AND form_code = $2
          AND ($4::timestamptz IS NULL OR updated_at = $4)
        RETURNING form_data_id, by_id, form_code, form_version_id, data_json,
                  snapshot_id, status, created_at, updated_at
        "#
    );
    let form = sqlx::query_as::<_, FormData>(&sql)
        .bind(by_id)
        .bind(form_code)
        .bind(data_json)
        .bind(request.expected_updated_at)
        .fetch_optional(pool)
        .await
        .context("failed to update form data")?
        .ok_or_else(|| anyhow!("updated_at conflict: form data was modified by another request"))?;
    insert_form_data_history(
        pool,
        tenant,
        FormDataHistoryInsert {
            form: &form,
            change_type: "MANUAL_UPDATE",
            old_data: Some(old_data),
            new_data: form.data_json.clone(),
            changed_by: request.changed_by.as_deref().unwrap_or("system"),
            reason: request.reason.as_deref(),
        },
    )
    .await?;
    Ok(form)
}

pub async fn preview_form(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    form_code: &str,
) -> Result<FormPreviewResult> {
    let form = match load_form_optional(pool, tenant, by_id, form_code).await? {
        Some(form) => form,
        None => generate_form(pool, tenant, by_id, form_code).await?,
    };
    let version = load_form_version(pool, form.form_version_id).await?;
    let validations = validate_form_data(pool, form.form_version_id, &form.data_json).await?;
    let history = list_form_data_history(pool, tenant, by_id, form_code).await?;
    let mut fields = template_fields(&version.template_json);
    if fields.is_empty() {
        fields = form
            .data_json
            .as_object()
            .map(|object| {
                object
                    .keys()
                    .filter(|key| *key != "_meta")
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
    }
    let fields = fields
        .into_iter()
        .map(|field| {
            let value = form.data_json.get(&field).cloned().unwrap_or(Value::Null);
            let meta = form
                .data_json
                .get("_meta")
                .and_then(|value| value.get(&field));
            let source = meta
                .and_then(|value| value.get("source"))
                .and_then(Value::as_str)
                .unwrap_or("manual")
                .to_string();
            let source_ref = meta
                .and_then(|value| value.get("source_ref"))
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let editable = meta
                .and_then(|value| value.get("editable"))
                .and_then(Value::as_bool)
                .unwrap_or(true);
            FormPreviewField {
                label: form_field_label(&field),
                field_path: field,
                value,
                source,
                source_ref,
                editable,
            }
        })
        .collect();
    Ok(FormPreviewResult {
        form,
        fields,
        validations,
        history,
    })
}

pub async fn list_form_attachments(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<FormAttachmentSummary>> {
    let forms = [
        ("FORM3", "과세표준 및 세액조정계산서"),
        ("FORM15", "소득금액조정명세서"),
        ("FORM22", "기부금 조정명세서"),
    ];
    let _ = forms;
    let forms = form_attachment_catalog();
    let mut summaries = Vec::with_capacity(forms.len());
    for (form_code, form_name) in forms {
        let form = load_form_optional(pool, tenant, by_id, form_code).await?;
        let (status, validation_count, total_amount, updated_at) = if let Some(form) = form {
            let validations =
                validate_form_data(pool, form.form_version_id, &form.data_json).await?;
            (
                form.status,
                validations.len(),
                form_total_amount(&form.data_json),
                Some(form.updated_at),
            )
        } else {
            ("NOT_GENERATED".to_string(), 0, 0, None)
        };
        summaries.push(FormAttachmentSummary {
            form_code: form_code.to_string(),
            form_name: form_name.to_string(),
            generated: status != "NOT_GENERATED",
            status,
            validation_count,
            total_amount,
            updated_at,
        });
    }
    Ok(summaries)
}

pub async fn check_form_linkage(pool: &PgPool, tenant: &TenantRef, by_id: i64) -> Result<Value> {
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let relationships = sqlx::query(
        r#"
        SELECT source_form, source_field, target_form, target_field, rule_json
        FROM form_relationships
        WHERE effective_from <= $1
          AND (effective_to IS NULL OR effective_to >= $2)
        ORDER BY relationship_id
        "#,
    )
    .bind(by.end_date)
    .bind(by.start_date)
    .fetch_all(pool)
    .await
    .context("failed to load form linkage relationships")?;
    let mut differences = Vec::new();
    for relationship in relationships {
        let source_form = relationship.get::<String, _>("source_form");
        let source_field = relationship.get::<String, _>("source_field");
        let target_form = relationship.get::<String, _>("target_form");
        let target_field = relationship.get::<String, _>("target_field");
        let Some(source) = load_form_optional(pool, tenant, by_id, &source_form).await? else {
            differences.push(json!({
                "source": format!("{source_form}.{source_field}"),
                "target": format!("{target_form}.{target_field}"),
                "issue": "MISSING_SOURCE_FORM"
            }));
            continue;
        };
        let Some(target) = load_form_optional(pool, tenant, by_id, &target_form).await? else {
            differences.push(json!({
                "source": format!("{source_form}.{source_field}"),
                "target": format!("{target_form}.{target_field}"),
                "issue": "MISSING_TARGET_FORM"
            }));
            continue;
        };
        let source_value = source
            .data_json
            .get(&source_field)
            .cloned()
            .unwrap_or(Value::Null);
        let target_value = target
            .data_json
            .get(&target_field)
            .cloned()
            .unwrap_or(Value::Null);
        if source_value != target_value {
            differences.push(json!({
                "source": format!("{source_form}.{source_field}"),
                "target": format!("{target_form}.{target_field}"),
                "source_value": source_value,
                "target_value": target_value,
                "issue": "VALUE_MISMATCH"
            }));
        }
    }
    Ok(json!({
        "tenant_code": tenant.tenant_code.clone(),
        "by_id": by_id,
        "balanced": differences.is_empty(),
        "differences": differences
    }))
}

pub async fn generate_form_pdf(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    form_code: &str,
) -> Result<FormOutputFile> {
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let preview = preview_form(pool, tenant, by_id, form_code).await?;
    let watermark = match by.status.as_str() {
        "FILED" => "FILED",
        "APPROVED" => "APPROVED",
        "AMENDED" => "AMENDED",
        _ => "DRAFT",
    };
    let title = format!("CIT {form_code} {}", preview.form.form_code);
    let mut lines = vec![
        format!("Tenant: {}", tenant.tenant_code),
        format!("Business year id: {by_id}"),
        format!("Status: {}", preview.form.status),
        format!("Watermark: {watermark}"),
        format!("Seal: CIT-{watermark}-{}", tenant.tenant_code),
    ];
    for field in &preview.fields {
        lines.push(format!(
            "{} = {} [{}]",
            field.label,
            pdf_value(&field.value),
            field.source
        ));
    }
    if !preview.validations.is_empty() {
        lines.push("Validation issues:".to_string());
        for issue in &preview.validations {
            lines.push(format!(
                "{} {} {}",
                issue.severity, issue.field_path, issue.message
            ));
        }
    }
    let contents = render_simple_pdf(&title, &lines, watermark);
    let output = FormOutputFile {
        file_name: format!("{}_{}_{}.pdf", tenant.tenant_code, by_id, form_code),
        content_type: "application/pdf".to_string(),
        contents,
    };
    record_print_history(
        pool,
        tenant,
        PrintHistoryEntry {
            by_id,
            form_code: Some(form_code),
            file_name: &output.file_name,
            content_type: &output.content_type,
            watermark,
            metadata: json!({ "form_status": preview.form.status }),
        },
    )
    .await?;
    Ok(output)
}

pub async fn generate_form_pdf_bundle(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<FormOutputFile> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (form_code, _) in form_attachment_catalog() {
            let file = generate_form_pdf(pool, tenant, by_id, form_code).await?;
            writer
                .start_file(file.file_name, options)
                .context("failed to start zip entry")?;
            writer
                .write_all(&file.contents)
                .context("failed to write zip entry")?;
        }
        writer.finish().context("failed to finalize pdf bundle")?;
    }
    let output = FormOutputFile {
        file_name: format!("{}_{}_forms.zip", tenant.tenant_code, by_id),
        content_type: "application/zip".to_string(),
        contents: cursor.into_inner(),
    };
    let by = tenant::get_business_year(pool, tenant, by_id).await?;
    let watermark = match by.status.as_str() {
        "FILED" => "FILED",
        "APPROVED" => "APPROVED",
        "AMENDED" => "AMENDED",
        _ => "DRAFT",
    };
    record_print_history(
        pool,
        tenant,
        PrintHistoryEntry {
            by_id,
            form_code: None,
            file_name: &output.file_name,
            content_type: &output.content_type,
            watermark,
            metadata: json!({ "bundle": true }),
        },
    )
    .await?;
    Ok(output)
}

pub async fn list_print_history(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
) -> Result<Vec<Value>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT print_id, by_id, form_code, file_name, content_type, watermark,
               status, printed_by, metadata, created_at
        FROM {schema}.print_history
        WHERE by_id = $1
        ORDER BY created_at DESC, print_id DESC
        LIMIT 200
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list print history")?;
    Ok(rows.into_iter().map(print_history_json).collect())
}

struct PrintHistoryEntry<'a> {
    by_id: i64,
    form_code: Option<&'a str>,
    file_name: &'a str,
    content_type: &'a str,
    watermark: &'a str,
    metadata: Value,
}

async fn record_print_history(
    pool: &PgPool,
    tenant: &TenantRef,
    entry: PrintHistoryEntry<'_>,
) -> Result<()> {
    let PrintHistoryEntry {
        by_id,
        form_code,
        file_name,
        content_type,
        watermark,
        metadata,
    } = entry;
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.print_history (
            by_id, form_code, file_name, content_type, watermark, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    );
    sqlx::query(&sql)
        .bind(by_id)
        .bind(form_code)
        .bind(file_name)
        .bind(content_type)
        .bind(watermark)
        .bind(metadata)
        .execute(pool)
        .await
        .context("failed to record print history")?;
    Ok(())
}

fn print_history_json(row: sqlx::postgres::PgRow) -> Value {
    json!({
        "print_id": row.get::<i64, _>("print_id"),
        "by_id": row.get::<i64, _>("by_id"),
        "form_code": row.get::<Option<String>, _>("form_code"),
        "file_name": row.get::<String, _>("file_name"),
        "content_type": row.get::<String, _>("content_type"),
        "watermark": row.get::<String, _>("watermark"),
        "status": row.get::<String, _>("status"),
        "printed_by": row.get::<String, _>("printed_by"),
        "metadata": row.get::<Value, _>("metadata"),
        "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at")
    })
}

fn form_attachment_catalog() -> &'static [(&'static str, &'static str)] {
    &[
        ("FORM3", "Corporate tax base and tax adjustment"),
        ("FORM15", "Income adjustment statement"),
        ("FORM22", "Donation adjustment statement"),
        ("FORM32", "Reserve rollforward statement"),
        ("FORM50", "E-filing summary statement"),
        ("ATT01", "Financial statement attachment"),
        ("ATT02", "Asset register attachment"),
        ("ATT03", "Transaction detail attachment"),
        ("ATT04", "Vehicle usage attachment"),
        ("ATT05", "Workflow approval attachment"),
        ("ATT06", "Validation result attachment"),
        ("ATT07", "Tax credit attachment"),
        ("ATT08", "Loss carryforward attachment"),
        ("ATT09", "Foreign income attachment"),
        ("ATT10", "Consolidated tax attachment"),
    ]
}

fn form_total_amount(data_json: &Value) -> i64 {
    for key in [
        "total_tax_due",
        "taxable_income",
        "corporate_tax",
        "accounting_income",
        "addbacks",
        "donations",
    ] {
        let total = data_json.get(key).map(numeric_total).unwrap_or_default();
        if total != 0 {
            return total;
        }
    }
    numeric_total(data_json)
}

fn numeric_total(value: &Value) -> i64 {
    match value {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|value| value.round() as i64))
            .unwrap_or_default(),
        Value::Array(items) => items.iter().map(numeric_total).sum(),
        Value::Object(object) => object
            .iter()
            .filter(|(key, _)| key.as_str() != "_meta")
            .map(|(_, value)| numeric_total(value))
            .sum(),
        _ => 0,
    }
}

fn pdf_value(value: &Value) -> String {
    let rendered = match value {
        Value::Null => "-".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    };
    if rendered.chars().count() > 140 {
        format!("{}...", rendered.chars().take(140).collect::<String>())
    } else {
        rendered
    }
}

fn render_simple_pdf(title: &str, lines: &[String], watermark: &str) -> Vec<u8> {
    let mut content_lines = vec![
        "BT".to_string(),
        "/F1 18 Tf".to_string(),
        "50 792 Td".to_string(),
        format!("({}) Tj", escape_pdf_text(title)),
        "/F1 10 Tf".to_string(),
        "0 -18 Td".to_string(),
        format!("(Watermark: {}) Tj", escape_pdf_text(watermark)),
    ];
    let max_lines = 48;
    for line in lines.iter().take(max_lines) {
        content_lines.push("0 -14 Td".to_string());
        content_lines.push(format!("({}) Tj", escape_pdf_text(line)));
    }
    if lines.len() > max_lines {
        content_lines.push("0 -14 Td".to_string());
        content_lines.push(format!("(... {} more lines) Tj", lines.len() - max_lines));
    }
    content_lines.push("ET".to_string());
    let content = content_lines.join("\n");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            content
        ),
    ];

    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }
    let xref_start = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_start
        )
        .as_bytes(),
    );
    bytes
}

fn escape_pdf_text(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_graphic() || character == ' ' {
                character
            } else {
                '?'
            }
        })
        .flat_map(|character| match character {
            '(' | ')' | '\\' => vec!['\\', character],
            _ => vec![character],
        })
        .collect()
}

fn summarize_adjustments(adjustments: &[TaxAdjustment]) -> Value {
    let mut latest = serde_json::Map::new();
    if let Some(adjustment) = adjustments.first() {
        latest.insert("details".to_string(), adjustment.metadata.clone());
    }
    for adjustment in adjustments.iter().rev() {
        latest.insert(adjustment.adj_code.clone(), json!(adjustment.amount));
    }
    Value::Object(latest)
}

fn build_form_payload(form_code: &str, summary: &Value, snapshot_id: i64) -> Result<Value> {
    let get = |key: &str| summary.get(key).and_then(Value::as_i64).unwrap_or(0);
    let donations = summary
        .get("details")
        .and_then(|value| value.get("donations"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let donation_number = |key: &str| {
        donations
            .get(key)
            .and_then(Value::as_i64)
            .unwrap_or_default()
    };
    let mut payload = match form_code {
        "FORM3" => json!({
            "snapshot_id": snapshot_id,
            "taxable_income": get("TAXABLE_INCOME"),
            "corporate_tax": get("CORPORATE_TAX"),
            "local_income_tax": get("LOCAL_INCOME_TAX"),
            "tax_credits": get("TAX_CREDITS"),
            "total_tax_due": get("TOTAL_TAX_DUE")
        }),
        "FORM15" => json!({
            "snapshot_id": snapshot_id,
            "accounting_income": get("ACCOUNTING_INCOME"),
            "addbacks": get("ADDBACKS"),
            "deductions": get("DEDUCTIONS"),
            "taxable_income": get("TAXABLE_INCOME")
        }),
        "FORM22" => json!({
            "snapshot_id": snapshot_id,
            "donations": summary
                .get("details")
                .and_then(|value| value.get("donations"))
                .cloned()
                .unwrap_or_else(|| json!({}))
        }),
        "FORM32" => json!({
            "snapshot_id": snapshot_id,
            "taxable_income": get("TAXABLE_INCOME"),
            "addbacks": get("ADDBACKS"),
            "deductions": get("DEDUCTIONS"),
            "reserve_basis": get("ADDBACKS") - get("DEDUCTIONS")
        }),
        "FORM50" => json!({
            "snapshot_id": snapshot_id,
            "taxable_income": get("TAXABLE_INCOME"),
            "corporate_tax": get("CORPORATE_TAX"),
            "local_income_tax": get("LOCAL_INCOME_TAX"),
            "total_tax_due": get("TOTAL_TAX_DUE"),
            "efile_ready": get("TOTAL_TAX_DUE") > 0
        }),
        code if code.starts_with("ATT") => json!({
            "snapshot_id": snapshot_id,
            "attachment_code": code,
            "taxable_income": get("TAXABLE_INCOME"),
            "total_tax_due": get("TOTAL_TAX_DUE"),
            "amount": get("TOTAL_TAX_DUE").max(get("TAXABLE_INCOME"))
        }),
        _ => return Err(anyhow!("unsupported form code {form_code}")),
    };
    if form_code == "FORM22" {
        set_form_field(
            &mut payload,
            "deductible_donations",
            json!(donation_number("deductible")),
        )?;
        set_form_field(
            &mut payload,
            "non_deductible_donations",
            json!(donation_number("non_deductible")),
        )?;
    }
    if let Some(object) = payload.as_object().cloned() {
        for field in object.keys().filter(|field| *field != "snapshot_id") {
            set_form_field_meta(
                &mut payload,
                field,
                "auto",
                Some("tax adjustment summary".to_string()),
                true,
            )?;
        }
    }
    Ok(payload)
}

async fn load_form_optional(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    form_code: &str,
) -> Result<Option<FormData>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT form_data_id, by_id, form_code, form_version_id, data_json,
               snapshot_id, status, created_at, updated_at
        FROM {schema}.form_data
        WHERE by_id = $1 AND form_code = $2
        "#
    );
    sqlx::query_as::<_, FormData>(&sql)
        .bind(by_id)
        .bind(form_code)
        .fetch_optional(pool)
        .await
        .context("failed to load optional form data")
}

async fn load_form_version(pool: &PgPool, form_version_id: i64) -> Result<FormVersionLite> {
    sqlx::query_as::<_, FormVersionLite>(
        r#"
        SELECT form_version_id, template_json
        FROM form_versions
        WHERE form_version_id = $1
        "#,
    )
    .bind(form_version_id)
    .fetch_one(pool)
    .await
    .context("failed to load form version")
}

#[derive(sqlx::FromRow)]
struct FormVersionLite {
    #[allow(dead_code)]
    form_version_id: i64,
    template_json: Value,
}

async fn apply_form_relationships(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
    target_form: &str,
    data_json: &mut Value,
) -> Result<()> {
    let relationships = sqlx::query(
        r#"
        SELECT source_form, source_field, target_field, rule_json
        FROM form_relationships
        WHERE target_form = $1
          AND effective_from <= $2
          AND (effective_to IS NULL OR effective_to >= $3)
        ORDER BY relationship_id
        "#,
    )
    .bind(target_form)
    .bind(end_date)
    .bind(start_date)
    .fetch_all(pool)
    .await
    .context("failed to load form relationships")?;

    for relationship in relationships {
        let source_form = relationship.get::<String, _>("source_form");
        let source_field = relationship.get::<String, _>("source_field");
        let target_field = relationship.get::<String, _>("target_field");
        let rule_json = relationship.get::<Value, _>("rule_json");
        let Some(source) = load_form_optional(pool, tenant, by_id, &source_form).await? else {
            continue;
        };
        let source_value = source
            .data_json
            .get(&source_field)
            .cloned()
            .unwrap_or(Value::Null);
        let operation = rule_json
            .get("operation")
            .and_then(Value::as_str)
            .unwrap_or("copy_latest")
            .to_ascii_uppercase();
        match operation.as_str() {
            "ADD" | "SUM" | "SUBTOTAL" => {
                let current = data_json
                    .get(&target_field)
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                let addition = source_value.as_i64().unwrap_or(0);
                set_form_field(data_json, &target_field, json!(current + addition))?;
            }
            "COPY" | "COPY_LATEST" => {
                set_form_field(data_json, &target_field, source_value)?;
            }
            "NULL_AS_ZERO" => {
                set_form_field(
                    data_json,
                    &target_field,
                    json!(source_value.as_i64().unwrap_or(0)),
                )?;
            }
            "ROUND" => {
                let scale = rule_json.get("scale").and_then(Value::as_i64).unwrap_or(0);
                let value = source_value
                    .as_f64()
                    .or_else(|| source_value.as_i64().map(|value| value as f64))
                    .unwrap_or(0.0);
                let factor = 10_f64.powi(scale.clamp(0, 6) as i32);
                set_form_field(
                    data_json,
                    &target_field,
                    json!((value * factor).round() / factor),
                )?;
            }
            _ => continue,
        }
        set_form_field_meta(
            data_json,
            &target_field,
            "auto_relationship",
            Some(format!("{source_form}.{source_field}")),
            false,
        )?;
    }
    Ok(())
}

async fn validate_form_data(
    pool: &PgPool,
    form_version_id: i64,
    data_json: &Value,
) -> Result<Vec<FormValidationIssue>> {
    let rows = sqlx::query(
        r#"
        SELECT field_path, rule_code, severity, message, rule_json
        FROM form_validations
        WHERE form_version_id = $1 AND active = TRUE
        ORDER BY validation_id
        "#,
    )
    .bind(form_version_id)
    .fetch_all(pool)
    .await
    .context("failed to load form validations")?;

    let mut issues = Vec::new();
    for row in rows {
        let field_path = row.get::<String, _>("field_path");
        let rule_code = row.get::<String, _>("rule_code");
        let severity = row.get::<String, _>("severity");
        let message = row.get::<String, _>("message");
        let rule_json = row.get::<Value, _>("rule_json");
        let value = data_json.get(&field_path);
        let failed = match rule_code.as_str() {
            "REQUIRED" => value
                .map(|value| value.is_null() || value.as_str().is_some_and(str::is_empty))
                .unwrap_or(true),
            "MIN" => {
                let minimum = rule_json.get("min").and_then(Value::as_i64).unwrap_or(0);
                value.and_then(Value::as_i64).unwrap_or(0) < minimum
            }
            "EQUALS_FIELD" => {
                let other = rule_json.get("field").and_then(Value::as_str).unwrap_or("");
                value != data_json.get(other)
            }
            _ => false,
        };
        if failed {
            issues.push(FormValidationIssue {
                field_path,
                rule_code,
                severity,
                message,
            });
        }
    }
    Ok(issues)
}

async fn list_form_data_history(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    form_code: &str,
) -> Result<Vec<FormDataHistory>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT history_id, form_data_id, by_id, form_code, change_type, changed_by,
               reason, old_data, new_data, changed_at
        FROM {schema}.form_data_history
        WHERE by_id = $1 AND form_code = $2
        ORDER BY changed_at DESC, history_id DESC
        LIMIT 20
        "#
    );
    sqlx::query_as::<_, FormDataHistory>(&sql)
        .bind(by_id)
        .bind(form_code)
        .fetch_all(pool)
        .await
        .context("failed to list form data history")
}

struct FormDataHistoryInsert<'a> {
    form: &'a FormData,
    change_type: &'a str,
    old_data: Option<Value>,
    new_data: Value,
    changed_by: &'a str,
    reason: Option<&'a str>,
}

async fn insert_form_data_history(
    pool: &PgPool,
    tenant: &TenantRef,
    item: FormDataHistoryInsert<'_>,
) -> Result<()> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.form_data_history (
            form_data_id, by_id, form_code, change_type, changed_by, reason, old_data, new_data
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#
    );
    sqlx::query(&sql)
        .bind(item.form.form_data_id)
        .bind(item.form.by_id)
        .bind(&item.form.form_code)
        .bind(item.change_type)
        .bind(item.changed_by)
        .bind(item.reason)
        .bind(item.old_data)
        .bind(item.new_data)
        .execute(pool)
        .await
        .context("failed to insert form data history")?;
    Ok(())
}

fn set_form_field(data_json: &mut Value, field: &str, value: Value) -> Result<()> {
    let object = data_json
        .as_object_mut()
        .ok_or_else(|| anyhow!("form data must be a JSON object"))?;
    object.insert(field.to_string(), value);
    Ok(())
}

fn set_form_field_meta(
    data_json: &mut Value,
    field: &str,
    source: &str,
    source_ref: Option<String>,
    editable: bool,
) -> Result<()> {
    let object = data_json
        .as_object_mut()
        .ok_or_else(|| anyhow!("form data must be a JSON object"))?;
    let meta = object.entry("_meta").or_insert_with(|| json!({}));
    let meta_object = meta
        .as_object_mut()
        .ok_or_else(|| anyhow!("form metadata must be a JSON object"))?;
    meta_object.insert(
        field.to_string(),
        json!({
            "source": source,
            "source_ref": source_ref,
            "editable": editable
        }),
    );
    Ok(())
}

fn template_fields(template_json: &Value) -> Vec<String> {
    template_json
        .get("fields")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn form_field_label(field: &str) -> String {
    match field {
        "taxable_income" => "과세표준",
        "corporate_tax" => "산출세액",
        "local_income_tax" => "지방소득세",
        "tax_credits" => "세액공제",
        "total_tax_due" => "총 납부세액",
        "accounting_income" => "결산서상 당기순이익",
        "addbacks" => "익금산입/손금불산입",
        "deductions" => "손금산입/익금불산입",
        _ => field,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::{calculate_corporate_tax, minimum_tax_extra_due};
    use crate::domain::TaxRate;
    use chrono::NaiveDate;
    use serde_json::json;

    fn rate(from: i64, to: Option<i64>, bps: i32, deduction: i64) -> TaxRate {
        TaxRate {
            tax_rate_id: 1,
            law_version_id: 1,
            item_code: "CORPORATE_TAX".to_string(),
            taxable_from: from,
            taxable_to: to,
            base_tax: 0,
            rate_bps: bps,
            progressive_deduction: deduction,
            effective_from: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
            effective_to: None,
            metadata: json!({}),
        }
    }

    #[test]
    fn progressive_tax_rate_uses_matching_bracket() {
        let rates = vec![
            rate(0, Some(200_000_000), 900, 0),
            rate(200_000_001, Some(20_000_000_000), 1900, 20_000_000),
        ];
        assert_eq!(calculate_corporate_tax(100_000_000, &rates), 9_000_000);
        assert_eq!(calculate_corporate_tax(300_000_000, &rates), 37_000_000);
    }

    #[test]
    fn minimum_tax_extra_due_handles_regression_cases() {
        let cases = [
            (60_000_000, 500_000_000, 1_000, 0),
            (50_000_000, 500_000_000, 1_000, 0),
            (30_000_000, 500_000_000, 1_000, 20_000_000),
            (0, 0, 1_000, 0),
            (100_000_000, 1_000_000_000, 1_700, 70_000_000),
        ];
        for (regular_tax, tax_base, bps, expected) in cases {
            assert_eq!(minimum_tax_extra_due(regular_tax, tax_base, bps), expected);
        }
    }
}
