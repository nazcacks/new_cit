use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use crate::{
    db::quote_ident,
    domain::{
        AdjustmentItem, AssetBasedAdjustmentRequest, AssetBasedAdjustmentResult,
        CalculateAdjustmentRequest, CalculationResult, CreateIncomeAdjustmentRequest,
        CreateLawAmendmentRequest, CreateTaxLawRequest, CreateTaxLimitRequest,
        CreateTaxRateRequest, CreateVehicleUsageLogRequest, FormData, IncomeAdjustmentItemInput,
        IncomeAdjustmentResult, LawAmendmentHistory, LawSnapshot, LawVersioningImpactRequest,
        ReserveRecord, TaxAdjustment, TaxLawVersion, TaxLimit, TaxRate, TenantRef,
        UpdateTaxLawStatusRequest, VehicleUsageLog,
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
        ORDER BY effective_from DESC, law_version_id DESC
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

pub async fn calculate_adjustments(
    pool: &PgPool,
    tenant: &TenantRef,
    by_id: i64,
    request: CalculateAdjustmentRequest,
) -> Result<CalculationResult> {
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
    sqlx::query_as::<_, AdjustmentItem>(&sql)
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
        .context("failed to insert B-1 adjustment item")
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
    let module_code = normalize_asset_module(module_code)?;
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
    let data_json = build_form_payload(form_code, &summary, snapshot.snapshot_id)?;

    let schema = quote_ident(&tenant.schema_name)?;
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

    sqlx::query_as::<_, FormData>(&sql)
        .bind(by_id)
        .bind(form_code)
        .bind(form_version_id)
        .bind(data_json)
        .bind(snapshot.snapshot_id)
        .fetch_one(pool)
        .await
        .context("failed to upsert form data")
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
    let payload = match form_code {
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
        _ => return Err(anyhow!("unsupported form code {form_code}")),
    };
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::calculate_corporate_tax;
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
}
