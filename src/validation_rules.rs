use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use crate::{
    db::quote_ident,
    domain::{
        AssetBsReconcileResult, DismissValidationIssueRequest, StdFsValidationResult, TenantRef,
        TransactionIsReconcileResult, ValidationIssue, ValidationRuleRecord, ValidationRunResult,
        VehicleB10ReconcileResult,
    },
    std_fs, tax, tax_data, tenant,
};

pub async fn list_rules(pool: &PgPool) -> Result<Vec<ValidationRuleRecord>> {
    sqlx::query_as::<_, ValidationRuleRecord>(
        r#"
        SELECT rule_code, severity, area, message_template, applies_to, active, created_at
        FROM validation_rules
        WHERE active = TRUE
        ORDER BY area, rule_code
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list validation rules")
}

pub async fn run_validation(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    by_id: i64,
) -> Result<ValidationRunResult> {
    tenant::ensure_business_year_editable(pool, tenant_ref, by_id, "validation").await?;
    let rules = list_rules(pool).await?;
    let facts = ValidationFacts::load(pool, tenant_ref, by_id).await?;
    let mut candidates = Vec::new();

    for rule in &rules {
        if let Some(issue) = evaluate_rule(rule, &facts) {
            candidates.push(issue);
        }
    }

    supersede_open_issues(pool, tenant_ref, by_id).await?;
    let mut issues = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        issues.push(insert_issue(pool, tenant_ref, by_id, candidate).await?);
    }

    let error_count = issues
        .iter()
        .filter(|issue| issue.severity == "ERROR")
        .count();
    let warn_count = issues
        .iter()
        .filter(|issue| issue.severity == "WARN")
        .count();
    let info_count = issues
        .iter()
        .filter(|issue| issue.severity == "INFO")
        .count();

    Ok(ValidationRunResult {
        by_id,
        total_rules: rules.len(),
        executed_rules: rules.len(),
        pass: error_count == 0,
        error_count,
        warn_count,
        info_count,
        issues,
    })
}

pub async fn list_issues(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    by_id: i64,
) -> Result<Vec<ValidationIssue>> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        SELECT issue_id, by_id, rule_code, severity, area, message,
               target_path, status, metadata, created_at, dismissed_at
        FROM {schema}.validation_issues
        WHERE by_id = $1
        ORDER BY
            CASE severity WHEN 'ERROR' THEN 0 WHEN 'WARN' THEN 1 ELSE 2 END,
            created_at DESC,
            issue_id DESC
        "#
    );
    sqlx::query_as::<_, ValidationIssue>(&sql)
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list validation issues")
}

pub async fn dismiss_issue(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    issue_id: i64,
    request: DismissValidationIssueRequest,
) -> Result<ValidationIssue> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let by_id = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT by_id FROM {schema}.validation_issues WHERE issue_id = $1"
    ))
    .bind(issue_id)
    .fetch_one(pool)
    .await
    .context("validation issue not found")?;
    tenant::ensure_business_year_editable(pool, tenant_ref, by_id, "validation").await?;
    let sql = format!(
        r#"
        UPDATE {schema}.validation_issues
        SET status = 'DISMISSED',
            dismissed_at = NOW(),
            metadata = metadata || jsonb_build_object(
                'dismiss_reason', COALESCE($2, ''),
                'dismissed_by', COALESCE($3, 'system')
            )
        WHERE issue_id = $1
        RETURNING issue_id, by_id, rule_code, severity, area, message,
                  target_path, status, metadata, created_at, dismissed_at
        "#
    );
    sqlx::query_as::<_, ValidationIssue>(&sql)
        .bind(issue_id)
        .bind(request.reason)
        .bind(request.dismissed_by)
        .fetch_one(pool)
        .await
        .context("validation issue not found")
}

struct ValidationFacts {
    by_id: i64,
    status: String,
    tax_data: crate::domain::TaxDataValidationSummary,
    vehicle_log_count: i64,
    adjustment_modules: HashSet<String>,
    generated_forms: HashSet<String>,
    form_validation_count: i64,
    efiling_count: i64,
    reserve_count: i64,
    notification_count: i64,
    std_fs: Option<StdFsValidationResult>,
    asset_bs: Option<AssetBsReconcileResult>,
    transaction_is: Option<TransactionIsReconcileResult>,
    vehicle_b10: Option<VehicleB10ReconcileResult>,
}

struct CandidateIssue {
    rule_code: String,
    severity: String,
    area: String,
    message: String,
    target_path: Option<String>,
    metadata: Value,
}

impl ValidationFacts {
    async fn load(pool: &PgPool, tenant_ref: &TenantRef, by_id: i64) -> Result<Self> {
        let business_year = tenant::get_business_year(pool, tenant_ref, by_id).await?;
        let tax_data = tax_data::validation_summary(pool, tenant_ref, by_id).await?;
        let schema = quote_ident(&tenant_ref.schema_name)?;

        let vehicle_log_count = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {schema}.vehicle_usage_logs WHERE by_id = $1"
        ))
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("failed to count vehicle usage logs")?;

        let adjustment_modules = sqlx::query(&format!(
            "SELECT source_module, COUNT(*) AS item_count FROM {schema}.adjustment_items WHERE by_id = $1 GROUP BY source_module"
        ))
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to count adjustment items")?
        .into_iter()
        .filter_map(|row| {
            let count = row.get::<i64, _>("item_count");
            (count > 0).then(|| row.get::<String, _>("source_module"))
        })
        .collect::<HashSet<_>>();

        let generated_forms = sqlx::query(&format!(
            "SELECT form_code FROM {schema}.form_data WHERE by_id = $1"
        ))
        .bind(by_id)
        .fetch_all(pool)
        .await
        .context("failed to list generated forms")?
        .into_iter()
        .map(|row| row.get::<String, _>("form_code"))
        .collect::<HashSet<_>>();

        let form_validation_count = sqlx::query_scalar::<_, i64>(&format!(
            r#"
            SELECT COUNT(*)
            FROM {schema}.form_data f
            JOIN public.form_validations v
              ON v.form_version_id = f.form_version_id
             AND v.active = TRUE
            WHERE f.by_id = $1
            "#
        ))
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("failed to count form validations")?;

        let efiling_count = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {schema}.efiling_history WHERE by_id = $1"
        ))
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("failed to count efilings")?;

        let reserve_count = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {schema}.reserves WHERE by_id = $1"
        ))
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("failed to count reserves")?;

        let notification_count = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {schema}.notifications WHERE by_id = $1 AND status = 'UNREAD'"
        ))
        .bind(by_id)
        .fetch_one(pool)
        .await
        .context("failed to count notifications")?;

        let std_fs = std_fs::validate_workspace_statements(pool, tenant_ref, by_id)
            .await
            .ok();
        let asset_bs = tax_data::asset_bs_reconcile(pool, tenant_ref, by_id)
            .await
            .ok();
        let transaction_is = tax_data::transaction_is_reconcile(pool, tenant_ref, by_id)
            .await
            .ok();
        let vehicle_b10 = tax::vehicle_b10_reconcile(pool, tenant_ref, by_id)
            .await
            .ok();

        Ok(Self {
            by_id,
            status: business_year.status,
            tax_data,
            vehicle_log_count,
            adjustment_modules,
            generated_forms,
            form_validation_count,
            efiling_count,
            reserve_count,
            notification_count,
            std_fs,
            asset_bs,
            transaction_is,
            vehicle_b10,
        })
    }
}

fn evaluate_rule(rule: &ValidationRuleRecord, facts: &ValidationFacts) -> Option<CandidateIssue> {
    let mut values = HashMap::new();
    values.insert("by_id", facts.by_id.to_string());
    values.insert("fs_line_count", facts.tax_data.fs_line_count.to_string());
    values.insert(
        "unresolved_mapping_count",
        facts.tax_data.unresolved_mapping_count.to_string(),
    );
    values.insert(
        "mandatory_mapping_missing_count",
        facts.tax_data.mandatory_mapping_missing_count.to_string(),
    );
    values.insert(
        "mandatory_mapping_missing_codes",
        facts.tax_data.mandatory_mapping_missing_codes.join(","),
    );
    values.insert("asset_count", facts.tax_data.asset_count.to_string());
    values.insert(
        "business_vehicle_count",
        facts.tax_data.business_vehicle_count.to_string(),
    );
    values.insert(
        "transaction_count",
        facts.tax_data.transaction_count.to_string(),
    );
    values.insert("status", facts.status.clone());
    values.insert(
        "std_bs_balance_diff",
        std_fs_issue_by_code(facts, "CHK_STDBS_BALANCE")
            .map(|issue| issue.difference.abs())
            .unwrap_or(0)
            .to_string(),
    );
    values.insert(
        "std_bs_vs_fs_diff",
        std_fs_issue_by_code(facts, "CHK_STDBS_VS_FS")
            .map(|issue| issue.difference.abs())
            .unwrap_or(0)
            .to_string(),
    );
    values.insert(
        "std_is_vs_fs_diff",
        std_fs_issue_by_code(facts, "CHK_STDIS_VS_FS")
            .map(|issue| issue.difference.abs())
            .unwrap_or(0)
            .to_string(),
    );
    values.insert(
        "std_fs_unmapped_count",
        facts
            .std_fs
            .as_ref()
            .map(|result| result.unmapped_count)
            .unwrap_or(0)
            .to_string(),
    );
    values.insert(
        "std_fs_confirmed",
        facts
            .std_fs
            .as_ref()
            .map(|result| if result.confirmed { "Y" } else { "N" })
            .unwrap_or("N")
            .to_string(),
    );
    values.insert(
        "ppe_cost_diff",
        asset_bs_issue_by_code(facts, "CHK_PPE_COST")
            .map(|issue| issue.difference.abs())
            .unwrap_or(0)
            .to_string(),
    );
    values.insert(
        "accum_depr_diff",
        asset_bs_issue_by_code(facts, "CHK_ACCUM_DEPR")
            .map(|issue| issue.difference.abs())
            .unwrap_or(0)
            .to_string(),
    );
    values.insert(
        "intangible_diff",
        asset_bs_issue_by_code(facts, "CHK_INTANGIBLE")
            .map(|issue| issue.difference.abs())
            .unwrap_or(0)
            .to_string(),
    );
    values.insert(
        "donation_txn_is_diff",
        transaction_is_issue_by_code(facts, "CHK_DONATION_TXN")
            .map(max_transaction_reconcile_difference)
            .unwrap_or(0)
            .to_string(),
    );
    values.insert(
        "entertain_txn_is_diff",
        transaction_is_issue_by_code(facts, "CHK_ENTERTAIN_TXN")
            .map(max_transaction_reconcile_difference)
            .unwrap_or(0)
            .to_string(),
    );
    values.insert(
        "interest_txn_is_diff",
        transaction_is_issue_by_code(facts, "CHK_INTEREST_TXN")
            .map(max_transaction_reconcile_difference)
            .unwrap_or(0)
            .to_string(),
    );
    values.insert(
        "vehicle_usage_default_count",
        vehicle_b10_issue_count_by_code(facts, "CHK_VEHICLE_USAGE_BPS").to_string(),
    );
    values.insert(
        "b10_link_diff",
        vehicle_b10_max_difference_by_code(facts, "CHK_B10_LINK").to_string(),
    );

    let failed = match rule.rule_code.as_str() {
        "TD_FS_REQUIRED" => facts.tax_data.fs_line_count == 0,
        "TD_FS_BALANCED" => facts.tax_data.fs_line_count > 0 && !facts.tax_data.balanced,
        "TD_MAPPING_RESOLVED" => facts.tax_data.unresolved_mapping_count > 0,
        "TD_TAX_REQUIRED_MAPPINGS" => facts.tax_data.mandatory_mapping_missing_count > 0,
        "TD_ASSET_REGISTER" => facts.tax_data.asset_count == 0,
        "TD_VEHICLE_USAGE" => {
            facts.tax_data.business_vehicle_count > 0 && facts.vehicle_log_count == 0
        }
        "TD_TRANSACTIONS" => facts.tax_data.transaction_count == 0,
        "FORM_VALIDATIONS_CLEAR" => facts.form_validation_count > 0,
        "EF_FILE_GENERATED" => facts.efiling_count == 0 && facts.status == "FILED",
        "WF_READY_FOR_APPROVAL" => facts.status == "DRAFT",
        "WF_FILE_LOCKED" => facts.status == "FILED" && facts.efiling_count == 0,
        "POST_UNREAD_NOTIFICATIONS" => facts.notification_count > 0,
        "RESERVE_REGISTERED" => facts.reserve_count == 0,
        code if code.starts_with("ADJ_B") => {
            let module_code = format!(
                "B{}",
                code.trim_start_matches("ADJ_B").trim_start_matches('0')
            );
            !facts.adjustment_modules.contains(&module_code)
        }
        code if code.starts_with("FORM_") => {
            let form_code = code.trim_start_matches("FORM_");
            !facts.generated_forms.contains(form_code)
        }
        code if code.starts_with("EF_") => false,
        code if code.starts_with("RP_") => false,
        code if code.starts_with("WF_") => false,
        "CHK_STDBS_BALANCE" => std_fs_rule_failed(facts, "CHK_STDBS_BALANCE"),
        "CHK_STDBS_VS_FS" => std_fs_rule_failed(facts, "CHK_STDBS_VS_FS"),
        "CHK_STDIS_VS_FS" => std_fs_rule_failed(facts, "CHK_STDIS_VS_FS"),
        "CHK_STDFS_UNMAPPED" => std_fs_rule_failed(facts, "CHK_STDFS_UNMAPPED"),
        "CHK_STDFS_CONFIRMED" => std_fs_confirmed_rule_failed(facts),
        "CHK_PPE_COST" => asset_bs_rule_failed(facts, "CHK_PPE_COST"),
        "CHK_ACCUM_DEPR" => asset_bs_rule_failed(facts, "CHK_ACCUM_DEPR"),
        "CHK_INTANGIBLE" => asset_bs_rule_failed(facts, "CHK_INTANGIBLE"),
        "CHK_DONATION_TXN" => transaction_is_rule_failed(facts, "CHK_DONATION_TXN"),
        "CHK_ENTERTAIN_TXN" => transaction_is_rule_failed(facts, "CHK_ENTERTAIN_TXN"),
        "CHK_INTEREST_TXN" => transaction_is_rule_failed(facts, "CHK_INTEREST_TXN"),
        "CHK_VEHICLE_USAGE_BPS" => vehicle_b10_rule_failed(facts, "CHK_VEHICLE_USAGE_BPS"),
        "CHK_B10_LINK" => vehicle_b10_rule_failed(facts, "CHK_B10_LINK"),
        _ => false,
    };

    if failed {
        let asset_issue = asset_bs_issue_by_code(facts, &rule.rule_code);
        let transaction_issue = transaction_is_issue_by_code(facts, &rule.rule_code);
        let vehicle_issues = vehicle_b10_issues_by_code(facts, &rule.rule_code);
        Some(CandidateIssue {
            rule_code: rule.rule_code.clone(),
            severity: rule.severity.clone(),
            area: rule.area.clone(),
            message: render_message(&rule.message_template, &values),
            target_path: Some(rule.applies_to.clone()),
            metadata: json!({
                "applies_to": rule.applies_to,
                "rule_code": rule.rule_code,
                "std_fs_validation": facts.std_fs,
                "asset_reconcile": asset_issue,
                "transaction_is_reconcile": transaction_issue,
                "vehicle_b10_reconcile": vehicle_issues,
            }),
        })
    } else {
        None
    }
}

fn std_fs_issue_by_code<'a>(
    facts: &'a ValidationFacts,
    rule_code: &str,
) -> Option<&'a crate::domain::StdFsValidationIssue> {
    facts
        .std_fs
        .as_ref()?
        .issues
        .iter()
        .find(|issue| issue.rule_code == rule_code)
}

fn std_fs_rule_failed(facts: &ValidationFacts, rule_code: &str) -> bool {
    std_fs_issue_by_code(facts, rule_code)
        .map(|issue| !issue.passed)
        .unwrap_or(false)
}

fn std_fs_confirmed_rule_failed(facts: &ValidationFacts) -> bool {
    facts
        .std_fs
        .as_ref()
        .map(|result| !result.confirmed)
        .unwrap_or(true)
}

fn asset_bs_issue_by_code<'a>(
    facts: &'a ValidationFacts,
    rule_code: &str,
) -> Option<&'a crate::domain::AssetBsReconcileIssue> {
    facts
        .asset_bs
        .as_ref()?
        .issues
        .iter()
        .find(|issue| issue.rule_code == rule_code)
}

fn asset_bs_rule_failed(facts: &ValidationFacts, rule_code: &str) -> bool {
    asset_bs_issue_by_code(facts, rule_code)
        .map(|issue| !issue.passed)
        .unwrap_or(false)
}

fn transaction_is_issue_by_code<'a>(
    facts: &'a ValidationFacts,
    rule_code: &str,
) -> Option<&'a crate::domain::TransactionIsReconcileIssue> {
    facts
        .transaction_is
        .as_ref()?
        .issues
        .iter()
        .find(|issue| issue.rule_code == rule_code)
}

fn transaction_is_rule_failed(facts: &ValidationFacts, rule_code: &str) -> bool {
    transaction_is_issue_by_code(facts, rule_code)
        .map(|issue| !issue.passed)
        .unwrap_or(false)
}

fn max_transaction_reconcile_difference(issue: &crate::domain::TransactionIsReconcileIssue) -> i64 {
    issue
        .transaction_is_difference
        .abs()
        .max(issue.is_std_difference.abs())
}

fn vehicle_b10_issues_by_code<'a>(
    facts: &'a ValidationFacts,
    rule_code: &str,
) -> Vec<&'a crate::domain::VehicleB10ReconcileIssue> {
    facts
        .vehicle_b10
        .as_ref()
        .map(|result| {
            result
                .issues
                .iter()
                .filter(|issue| issue.rule_code == rule_code)
                .collect()
        })
        .unwrap_or_default()
}

fn vehicle_b10_rule_failed(facts: &ValidationFacts, rule_code: &str) -> bool {
    vehicle_b10_issues_by_code(facts, rule_code)
        .into_iter()
        .any(|issue| !issue.passed)
}

fn vehicle_b10_issue_count_by_code(facts: &ValidationFacts, rule_code: &str) -> usize {
    vehicle_b10_issues_by_code(facts, rule_code)
        .into_iter()
        .filter(|issue| !issue.passed)
        .count()
}

fn vehicle_b10_max_difference_by_code(facts: &ValidationFacts, rule_code: &str) -> i64 {
    vehicle_b10_issues_by_code(facts, rule_code)
        .into_iter()
        .filter(|issue| !issue.passed)
        .map(|issue| issue.difference.abs())
        .max()
        .unwrap_or(0)
}

fn render_message(template: &str, values: &HashMap<&str, String>) -> String {
    values
        .iter()
        .fold(template.to_string(), |message, (key, value)| {
            message.replace(&format!("{{{key}}}"), value)
        })
}

async fn supersede_open_issues(pool: &PgPool, tenant_ref: &TenantRef, by_id: i64) -> Result<()> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        "UPDATE {schema}.validation_issues SET status = 'SUPERSEDED' WHERE by_id = $1 AND status = 'OPEN'"
    );
    sqlx::query(&sql)
        .bind(by_id)
        .execute(pool)
        .await
        .context("failed to supersede validation issues")?;
    Ok(())
}

async fn insert_issue(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    by_id: i64,
    issue: CandidateIssue,
) -> Result<ValidationIssue> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.validation_issues (
            by_id, rule_code, severity, area, message, target_path, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING issue_id, by_id, rule_code, severity, area, message,
                  target_path, status, metadata, created_at, dismissed_at
        "#
    );
    sqlx::query_as::<_, ValidationIssue>(&sql)
        .bind(by_id)
        .bind(issue.rule_code)
        .bind(issue.severity)
        .bind(issue.area)
        .bind(issue.message)
        .bind(issue.target_path)
        .bind(issue.metadata)
        .fetch_one(pool)
        .await
        .context("failed to insert validation issue")
}
