use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use crate::{
    db::quote_ident,
    domain::{
        CalculateAdjustmentRequest, CalculationResult, CreateLawAmendmentRequest,
        CreateTaxLawRequest, CreateTaxLimitRequest, CreateTaxRateRequest, FormData,
        LawAmendmentHistory, LawSnapshot, LawVersioningImpactRequest, TaxAdjustment, TaxLawVersion,
        TaxLimit, TaxRate, TenantRef, UpdateTaxLawStatusRequest,
    },
    tenant,
};

pub async fn create_tax_law(pool: &PgPool, request: CreateTaxLawRequest) -> Result<TaxLawVersion> {
    sqlx::query_as::<_, TaxLawVersion>(
        r#"
        INSERT INTO tax_law_versions (
            version_code, law_name, effective_from, effective_to, metadata
        )
        VALUES ($1, $2, $3, $4, COALESCE($5, '{}'::jsonb))
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
        WHERE status = 'APPROVED'
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
        WHERE status = 'APPROVED'
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
          AND status = 'APPROVED'
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
