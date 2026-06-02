use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use crate::{
    db::quote_ident,
    domain::{
        CreateFormRelationshipRequest, CreateFormVersionRequest, CreateTaxFormRequest,
        FormMigrationRequest, FormMigrationResult, FormRelationship, FormVersion,
        ResolveFormVersionQuery, TaxForm, TenantRef, UpdateFormVersionStatusRequest,
    },
    tenant,
};

const ACTIVE_FORM_STATUSES: &[&str] = &["APPROVED", "ACTIVE"];
const ALLOWED_FORM_STATUSES: &[&str] = &["DRAFT", "REVIEWED", "APPROVED", "ACTIVE", "RETIRED"];

pub async fn list_tax_forms(pool: &PgPool) -> Result<Vec<TaxForm>> {
    sqlx::query_as::<_, TaxForm>(
        r#"
        SELECT form_id, form_code, form_name, form_group, description, active, created_at, updated_at
        FROM tax_forms
        ORDER BY form_code
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list tax forms")
}

pub async fn create_tax_form(pool: &PgPool, request: CreateTaxFormRequest) -> Result<TaxForm> {
    let form_code = normalize_required(&request.form_code, "form_code")?;
    let form_name = normalize_required(&request.form_name, "form_name")?;
    sqlx::query_as::<_, TaxForm>(
        r#"
        INSERT INTO tax_forms (form_code, form_name, form_group, description, active)
        VALUES ($1, $2, $3, $4, COALESCE($5, TRUE))
        ON CONFLICT (form_code) DO UPDATE
        SET form_name = EXCLUDED.form_name,
            form_group = EXCLUDED.form_group,
            description = EXCLUDED.description,
            active = EXCLUDED.active,
            updated_at = NOW()
        RETURNING form_id, form_code, form_name, form_group, description, active, created_at, updated_at
        "#,
    )
    .bind(form_code)
    .bind(form_name)
    .bind(request.form_group)
    .bind(request.description)
    .bind(request.active)
    .fetch_one(pool)
    .await
    .context("failed to upsert tax form")
}

pub async fn list_form_versions(pool: &PgPool) -> Result<Vec<FormVersion>> {
    sqlx::query_as::<_, FormVersion>(
        r#"
        SELECT form_version_id, form_code, form_name, version_no, effective_from, effective_to,
               template_json, status
        FROM form_versions
        ORDER BY form_code, effective_from DESC, form_version_id DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list form versions")
}

pub async fn create_form_version(
    pool: &PgPool,
    request: CreateFormVersionRequest,
) -> Result<FormVersion> {
    let form_code = normalize_required(&request.form_code, "form_code")?;
    let form_name = normalize_required(&request.form_name, "form_name")?;
    let version_no = normalize_required(&request.version_no, "version_no")?;
    let status = normalize_form_status(request.status.as_deref().unwrap_or("DRAFT"))?;
    let template_json = request
        .template_json
        .unwrap_or_else(|| json!({ "fields": [] }));

    create_tax_form(
        pool,
        CreateTaxFormRequest {
            form_code: form_code.clone(),
            form_name: form_name.clone(),
            form_group: Some("CIT".to_string()),
            description: Some("서식 버전에서 생성됨".to_string()),
            active: Some(true),
        },
    )
    .await?;

    let version = sqlx::query_as::<_, FormVersion>(
        r#"
        INSERT INTO form_versions (
            form_code, form_name, version_no, effective_from, effective_to, template_json, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING form_version_id, form_code, form_name, version_no, effective_from, effective_to,
                  template_json, status
        "#,
    )
    .bind(form_code)
    .bind(form_name)
    .bind(version_no)
    .bind(request.effective_from)
    .bind(request.effective_to)
    .bind(&template_json)
    .bind(status)
    .fetch_one(pool)
    .await
    .context("failed to create form version")?;

    upsert_form_template(pool, version.form_version_id, &version.template_json).await?;
    Ok(version)
}

pub async fn update_form_version_status(
    pool: &PgPool,
    form_version_id: i64,
    request: UpdateFormVersionStatusRequest,
) -> Result<FormVersion> {
    let status = normalize_form_status(&request.status)?;
    sqlx::query_as::<_, FormVersion>(
        r#"
        UPDATE form_versions
        SET status = $1
        WHERE form_version_id = $2
        RETURNING form_version_id, form_code, form_name, version_no, effective_from, effective_to,
                  template_json, status
        "#,
    )
    .bind(status)
    .bind(form_version_id)
    .fetch_one(pool)
    .await
    .context("failed to update form version status")
}

pub async fn list_form_relationships(pool: &PgPool) -> Result<Vec<FormRelationship>> {
    sqlx::query_as::<_, FormRelationship>(
        r#"
        SELECT relationship_id, source_form, source_field, target_form, target_field,
               rule_json, effective_from, effective_to
        FROM form_relationships
        ORDER BY source_form, target_form, relationship_id
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list form relationships")
}

pub async fn create_form_relationship(
    pool: &PgPool,
    request: CreateFormRelationshipRequest,
) -> Result<FormRelationship> {
    sqlx::query_as::<_, FormRelationship>(
        r#"
        INSERT INTO form_relationships (
            source_form, source_field, target_form, target_field, rule_json, effective_from, effective_to
        )
        VALUES ($1, $2, $3, $4, COALESCE($5, '{}'::jsonb), $6, $7)
        RETURNING relationship_id, source_form, source_field, target_form, target_field,
                  rule_json, effective_from, effective_to
        "#,
    )
    .bind(normalize_required(&request.source_form, "source_form")?)
    .bind(normalize_required(&request.source_field, "source_field")?)
    .bind(normalize_required(&request.target_form, "target_form")?)
    .bind(normalize_required(&request.target_field, "target_field")?)
    .bind(request.rule_json)
    .bind(request.effective_from)
    .bind(request.effective_to)
    .fetch_one(pool)
    .await
    .context("failed to create form relationship")
}

pub async fn check_form_relationship_cycles(pool: &PgPool) -> Result<Value> {
    let rows = sqlx::query(
        r#"
        SELECT source_form, source_field, target_form, target_field
        FROM form_relationships
        ORDER BY relationship_id
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to load form relationship graph")?;
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut references = Vec::new();
    for row in rows {
        let source = format!(
            "{}.{}",
            row.get::<String, _>("source_form"),
            row.get::<String, _>("source_field")
        );
        let target = format!(
            "{}.{}",
            row.get::<String, _>("target_form"),
            row.get::<String, _>("target_field")
        );
        graph
            .entry(source.clone())
            .or_default()
            .push(target.clone());
        references.push(json!({ "source": source, "target": target }));
    }
    let mut cycles = Vec::new();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for node in graph.keys() {
        let mut path = Vec::new();
        find_cycles(
            node,
            &graph,
            &mut visiting,
            &mut visited,
            &mut path,
            &mut cycles,
        );
    }
    Ok(json!({
        "valid": cycles.is_empty(),
        "relationship_count": references.len(),
        "references": references,
        "cycles": cycles
    }))
}

pub async fn resolve_form_version(
    pool: &PgPool,
    query: ResolveFormVersionQuery,
) -> Result<FormVersion> {
    let tenant_ref = tenant::resolve_tenant(pool, &query.tenant_code).await?;
    resolve_form_version_for_business_year(pool, &tenant_ref, query.by_id, &query.form_code).await
}

pub async fn dry_run_migration(
    pool: &PgPool,
    request: FormMigrationRequest,
) -> Result<FormMigrationResult> {
    let tenant_ref = tenant::resolve_tenant(pool, &request.tenant_code).await?;
    let target = get_form_version(pool, request.to_version_id).await?;
    if target.form_code != request.form_code {
        return Err(anyhow!("invalid target form version for form_code"));
    }
    let current =
        load_current_form_data(pool, &tenant_ref, request.by_id, &request.form_code).await?;
    let source_fields = match current.from_version_id {
        Some(form_version_id) => {
            template_fields(&get_form_version(pool, form_version_id).await?.template_json)
        }
        None => Vec::new(),
    };
    let target_fields = template_fields(&target.template_json);
    let added_fields = target_fields
        .iter()
        .filter(|field| !source_fields.contains(*field))
        .cloned()
        .collect::<Vec<_>>();
    let removed_fields = source_fields
        .iter()
        .filter(|field| !target_fields.contains(*field))
        .cloned()
        .collect::<Vec<_>>();
    let common_fields = target_fields
        .iter()
        .filter(|field| source_fields.contains(*field))
        .cloned()
        .collect::<Vec<_>>();
    let executable = ACTIVE_FORM_STATUSES.contains(&target.status.as_str());

    Ok(FormMigrationResult {
        mode: "DRY_RUN".to_string(),
        tenant_code: request.tenant_code,
        by_id: request.by_id,
        form_code: request.form_code,
        from_version_id: current.from_version_id,
        to_version_id: request.to_version_id,
        added_fields,
        removed_fields,
        common_fields,
        executable,
        message: if executable {
            "마이그레이션을 실행할 수 있습니다.".to_string()
        } else {
            "대상 서식 버전이 활성 상태가 아닙니다.".to_string()
        },
    })
}

pub async fn execute_migration(
    pool: &PgPool,
    request: FormMigrationRequest,
) -> Result<FormMigrationResult> {
    let dry_run = dry_run_migration(pool, request).await?;
    if !dry_run.executable {
        return Err(anyhow!("invalid form migration target status"));
    }
    let tenant_ref = tenant::resolve_tenant(pool, &dry_run.tenant_code).await?;
    tenant::ensure_business_year_editable(pool, &tenant_ref, dry_run.by_id, "form migration")
        .await?;
    let current =
        load_current_form_data(pool, &tenant_ref, dry_run.by_id, &dry_run.form_code).await?;
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.form_data (by_id, form_code, form_version_id, data_json, status)
        VALUES ($1, $2, $3, $4, 'MIGRATED')
        ON CONFLICT (by_id, form_code)
        DO UPDATE SET
            form_version_id = EXCLUDED.form_version_id,
            data_json = EXCLUDED.data_json,
            status = 'MIGRATED',
            updated_at = NOW()
        "#
    );
    sqlx::query(&sql)
        .bind(dry_run.by_id)
        .bind(&dry_run.form_code)
        .bind(dry_run.to_version_id)
        .bind(current.data_json.unwrap_or_else(|| json!({})))
        .execute(pool)
        .await
        .context("failed to execute form migration")?;
    insert_migration_history(pool, &tenant_ref, &dry_run, "EXECUTE").await?;

    Ok(FormMigrationResult {
        mode: "EXECUTE".to_string(),
        message: "마이그레이션이 실행되었습니다.".to_string(),
        ..dry_run
    })
}

pub async fn rollback_migration(
    pool: &PgPool,
    request: FormMigrationRequest,
) -> Result<FormMigrationResult> {
    let tenant_ref = tenant::resolve_tenant(pool, &request.tenant_code).await?;
    tenant::ensure_business_year_editable(pool, &tenant_ref, request.by_id, "form migration")
        .await?;
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        SELECT from_version_id, to_version_id
        FROM {schema}.form_data_migration_history
        WHERE by_id = $1 AND form_code = $2
        ORDER BY migrated_at DESC, migration_id DESC
        LIMIT 1
        "#
    );
    let row = sqlx::query(&sql)
        .bind(request.by_id)
        .bind(&request.form_code)
        .fetch_one(pool)
        .await
        .context("form migration history not found")?;
    let from_version_id = row.get::<Option<i64>, _>("from_version_id");
    let to_version_id = row.get::<i64, _>("to_version_id");

    if let Some(version_id) = from_version_id {
        let update_sql = format!(
            r#"
            UPDATE {schema}.form_data
            SET form_version_id = $3, status = 'ROLLBACK', updated_at = NOW()
            WHERE by_id = $1 AND form_code = $2
            "#
        );
        sqlx::query(&update_sql)
            .bind(request.by_id)
            .bind(&request.form_code)
            .bind(version_id)
            .execute(pool)
            .await
            .context("failed to rollback form data")?;
    } else {
        let delete_sql =
            format!("DELETE FROM {schema}.form_data WHERE by_id = $1 AND form_code = $2");
        sqlx::query(&delete_sql)
            .bind(request.by_id)
            .bind(&request.form_code)
            .execute(pool)
            .await
            .context("failed to rollback newly created form data")?;
    }

    Ok(FormMigrationResult {
        mode: "ROLLBACK".to_string(),
        tenant_code: request.tenant_code,
        by_id: request.by_id,
        form_code: request.form_code,
        from_version_id,
        to_version_id,
        added_fields: Vec::new(),
        removed_fields: Vec::new(),
        common_fields: Vec::new(),
        executable: true,
        message: "마이그레이션이 롤백되었습니다.".to_string(),
    })
}

async fn resolve_form_version_for_business_year(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    by_id: i64,
    form_code: &str,
) -> Result<FormVersion> {
    let by = tenant::get_business_year(pool, tenant_ref, by_id).await?;
    sqlx::query_as::<_, FormVersion>(
        r#"
        SELECT form_version_id, form_code, form_name, version_no, effective_from, effective_to,
               template_json, status
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
    .fetch_one(pool)
    .await
    .context("failed to resolve form version")
}

async fn get_form_version(pool: &PgPool, form_version_id: i64) -> Result<FormVersion> {
    sqlx::query_as::<_, FormVersion>(
        r#"
        SELECT form_version_id, form_code, form_name, version_no, effective_from, effective_to,
               template_json, status
        FROM form_versions
        WHERE form_version_id = $1
        "#,
    )
    .bind(form_version_id)
    .fetch_one(pool)
    .await
    .context("form version not found")
}

async fn upsert_form_template(
    pool: &PgPool,
    form_version_id: i64,
    template_json: &Value,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO form_templates (form_version_id, template_type, template_json, checksum)
        VALUES ($1, 'JSON', $2, md5($2::TEXT))
        ON CONFLICT (form_version_id, template_type)
        DO UPDATE SET template_json = EXCLUDED.template_json, checksum = EXCLUDED.checksum
        "#,
    )
    .bind(form_version_id)
    .bind(template_json)
    .execute(pool)
    .await
    .context("failed to upsert form template")?;
    Ok(())
}

struct CurrentFormData {
    from_version_id: Option<i64>,
    data_json: Option<Value>,
}

async fn load_current_form_data(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    by_id: i64,
    form_code: &str,
) -> Result<CurrentFormData> {
    tenant::get_business_year(pool, tenant_ref, by_id).await?;
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        SELECT form_version_id, data_json
        FROM {schema}.form_data
        WHERE by_id = $1 AND form_code = $2
        "#
    );
    let row = sqlx::query(&sql)
        .bind(by_id)
        .bind(form_code)
        .fetch_optional(pool)
        .await
        .context("failed to load current form data")?;
    Ok(CurrentFormData {
        from_version_id: row.as_ref().map(|row| row.get("form_version_id")),
        data_json: row.as_ref().map(|row| row.get("data_json")),
    })
}

async fn insert_migration_history(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    result: &FormMigrationResult,
    mode: &str,
) -> Result<()> {
    let schema = quote_ident(&tenant_ref.schema_name)?;
    let sql = format!(
        r#"
        INSERT INTO {schema}.form_data_migration_history (
            by_id, form_code, from_version_id, to_version_id, result_json
        )
        VALUES ($1, $2, $3, $4, $5)
        "#
    );
    sqlx::query(&sql)
        .bind(result.by_id)
        .bind(&result.form_code)
        .bind(result.from_version_id)
        .bind(result.to_version_id)
        .bind(json!({
            "mode": mode,
            "added_fields": &result.added_fields,
            "removed_fields": &result.removed_fields
        }))
        .execute(pool)
        .await
        .context("failed to insert form migration history")?;
    Ok(())
}

fn normalize_required(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("invalid {field}"));
    }
    Ok(value.to_string())
}

fn normalize_form_status(status: &str) -> Result<String> {
    let status = status.trim().to_ascii_uppercase();
    if !ALLOWED_FORM_STATUSES.contains(&status.as_str()) {
        return Err(anyhow!("invalid form status"));
    }
    Ok(status)
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

fn find_cycles(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Value>,
) {
    if let Some(position) = path.iter().position(|item| item == node) {
        cycles.push(json!({ "path": path[position..].to_vec() }));
        return;
    }
    if visited.contains(node) || !visiting.insert(node.to_string()) {
        return;
    }
    path.push(node.to_string());
    if let Some(next_nodes) = graph.get(node) {
        for next in next_nodes {
            find_cycles(next, graph, visiting, visited, path, cycles);
        }
    }
    path.pop();
    visiting.remove(node);
    visited.insert(node.to_string());
}
