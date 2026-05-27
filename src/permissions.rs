use std::collections::HashSet;

use anyhow::{anyhow, Context, Result};
use chrono::{NaiveDate, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use crate::{
    db::quote_ident,
    domain::{
        AuthUser, BusinessYear, CreateAccessDelegationRequest, Customer, RoleMenuFunctionInput,
        TenantRef, UpdateMenuFunctionsRequest, UpdateRoleMenuFunctionsRequest,
    },
    tenant,
};

const MASKED_VALUE: &str = "***MASKED***";

#[derive(Debug, Clone, Serialize)]
pub struct PermissionDecision {
    pub module_code: String,
    pub function_code: String,
    pub allowed: bool,
    pub denied: bool,
    pub matched: Vec<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataScope {
    All,
    Assigned,
    Owned,
    None,
}

pub async fn list_function_codes(pool: &PgPool) -> Result<Vec<Value>> {
    let rows = sqlx::query(
        r#"
        SELECT function_code, function_name, description, sort_order, active
        FROM function_codes
        ORDER BY sort_order, function_code
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list function codes")?;
    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "function_code": row.get::<String, _>("function_code"),
                "function_name": row.get::<String, _>("function_name"),
                "description": row.get::<Option<String>, _>("description"),
                "sort_order": row.get::<i32, _>("sort_order"),
                "active": row.get::<bool, _>("active")
            })
        })
        .collect())
}

pub async fn list_menu_functions(pool: &PgPool) -> Result<Vec<Value>> {
    let rows = sqlx::query(
        r#"
        SELECT mf.menu_key, mn.label, mf.function_code, fc.function_name, mf.enabled, mf.updated_at
        FROM menu_functions mf
        LEFT JOIN menu_nodes mn ON mn.menu_key = mf.menu_key
        LEFT JOIN function_codes fc ON fc.function_code = mf.function_code
        ORDER BY mf.menu_key, fc.sort_order, mf.function_code
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list menu functions")?;
    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "menu_key": row.get::<String, _>("menu_key"),
                "label": row.get::<Option<String>, _>("label"),
                "function_code": row.get::<String, _>("function_code"),
                "function_name": row.get::<Option<String>, _>("function_name"),
                "enabled": row.get::<bool, _>("enabled"),
                "updated_at": row.get::<chrono::DateTime<Utc>, _>("updated_at")
            })
        })
        .collect())
}

pub async fn replace_menu_functions(
    pool: &PgPool,
    menu_key: &str,
    request: UpdateMenuFunctionsRequest,
) -> Result<Vec<Value>> {
    let menu_key = menu_key.trim();
    if menu_key.is_empty() {
        return Err(anyhow!("invalid menu_key"));
    }
    sqlx::query("SELECT menu_key FROM menu_nodes WHERE menu_key = $1")
        .bind(menu_key)
        .fetch_one(pool)
        .await
        .context("menu not found")?;
    sqlx::query("DELETE FROM menu_functions WHERE menu_key = $1")
        .bind(menu_key)
        .execute(pool)
        .await
        .context("failed to clear menu functions")?;
    for function_code in normalized_function_codes(&request.functions)? {
        sqlx::query(
            r#"
            INSERT INTO menu_functions (menu_key, function_code, enabled)
            VALUES ($1, $2, TRUE)
            ON CONFLICT (menu_key, function_code)
            DO UPDATE SET enabled = TRUE, updated_at = NOW()
            "#,
        )
        .bind(menu_key)
        .bind(function_code)
        .execute(pool)
        .await
        .context("failed to insert menu function")?;
    }
    list_menu_functions(pool).await
}

pub async fn list_role_menu_functions(pool: &PgPool) -> Result<Vec<Value>> {
    let rows = sqlx::query(
        r#"
        SELECT role_code, menu_key, function_code, effect, updated_at
        FROM role_menu_functions
        ORDER BY role_code, menu_key, function_code
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list role menu functions")?;
    Ok(rows.into_iter().map(role_menu_function_json).collect())
}

pub async fn replace_role_menu_functions(
    pool: &PgPool,
    role_code: &str,
    request: UpdateRoleMenuFunctionsRequest,
) -> Result<Vec<Value>> {
    let role_code = normalize_role(role_code);
    sqlx::query("SELECT role_code FROM roles WHERE role_code = $1")
        .bind(&role_code)
        .fetch_one(pool)
        .await
        .context("role not found")?;
    sqlx::query("DELETE FROM role_menu_functions WHERE role_code = $1")
        .bind(&role_code)
        .execute(pool)
        .await
        .context("failed to clear role menu functions")?;
    for grant in request.grants {
        insert_role_menu_function(pool, &role_code, grant).await?;
    }
    list_role_menu_functions(pool).await
}

pub async fn list_access_delegations(pool: &PgPool, tenant: &TenantRef) -> Result<Vec<Value>> {
    let rows = sqlx::query(
        r#"
        SELECT d.delegation_id,
               grantor.login_id AS grantor_login_id,
               delegatee.login_id AS delegatee_login_id,
               d.customer_id, d.work_scope, d.valid_from, d.valid_to,
               d.status, d.reason, d.created_at
        FROM access_delegations d
        JOIN users grantor ON grantor.user_id = d.grantor_user_id
        JOIN users delegatee ON delegatee.user_id = d.delegatee_user_id
        WHERE d.tenant_id = $1
        ORDER BY d.created_at DESC, d.delegation_id DESC
        "#,
    )
    .bind(tenant.tenant_id)
    .fetch_all(pool)
    .await
    .context("failed to list access delegations")?;
    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "delegation_id": row.get::<i64, _>("delegation_id"),
                "grantor_login_id": row.get::<String, _>("grantor_login_id"),
                "delegatee_login_id": row.get::<String, _>("delegatee_login_id"),
                "customer_id": row.get::<i64, _>("customer_id"),
                "work_scope": row.get::<String, _>("work_scope"),
                "valid_from": row.get::<Option<NaiveDate>, _>("valid_from"),
                "valid_to": row.get::<Option<NaiveDate>, _>("valid_to"),
                "status": row.get::<String, _>("status"),
                "reason": row.get::<Option<String>, _>("reason"),
                "created_at": row.get::<chrono::DateTime<Utc>, _>("created_at")
            })
        })
        .collect())
}

pub async fn create_access_delegation(
    pool: &PgPool,
    tenant: &TenantRef,
    request: CreateAccessDelegationRequest,
) -> Result<Value> {
    if request.customer_id <= 0 {
        return Err(anyhow!("invalid customer_id"));
    }
    let work_scope = request.work_scope.trim().to_ascii_uppercase();
    if !matches!(
        work_scope.as_str(),
        "INFO" | "ADJUST" | "FORM" | "VALIDATE" | "APPROVE" | "PRINT" | "EFILE" | "POST"
    ) {
        return Err(anyhow!("invalid work_scope"));
    }
    let grantor = user_id(pool, tenant.tenant_id, &request.grantor_login_id).await?;
    let delegatee = user_id(pool, tenant.tenant_id, &request.delegatee_login_id).await?;
    let row = sqlx::query(
        r#"
        INSERT INTO access_delegations (
            tenant_id, grantor_user_id, delegatee_user_id, customer_id,
            work_scope, valid_from, valid_to, reason
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING delegation_id, customer_id, work_scope, valid_from, valid_to,
                  status, reason, created_at
        "#,
    )
    .bind(tenant.tenant_id)
    .bind(grantor)
    .bind(delegatee)
    .bind(request.customer_id)
    .bind(&work_scope)
    .bind(request.valid_from)
    .bind(request.valid_to)
    .bind(request.reason)
    .fetch_one(pool)
    .await
    .context("failed to create access delegation")?;

    Ok(json!({
        "delegation_id": row.get::<i64, _>("delegation_id"),
        "grantor_login_id": request.grantor_login_id,
        "delegatee_login_id": request.delegatee_login_id,
        "customer_id": row.get::<i64, _>("customer_id"),
        "work_scope": row.get::<String, _>("work_scope"),
        "valid_from": row.get::<Option<NaiveDate>, _>("valid_from"),
        "valid_to": row.get::<Option<NaiveDate>, _>("valid_to"),
        "status": row.get::<String, _>("status"),
        "reason": row.get::<Option<String>, _>("reason"),
        "created_at": row.get::<chrono::DateTime<Utc>, _>("created_at")
    }))
}

pub async fn evaluate_permission(
    pool: &PgPool,
    user_id: i64,
    module_code: &str,
    function_code: &str,
) -> Result<PermissionDecision> {
    let module_code = module_code.trim().to_ascii_lowercase();
    let function_code = function_code.trim().to_ascii_uppercase();
    if module_code.is_empty() || function_code.is_empty() {
        return Err(anyhow!("invalid permission target"));
    }
    let mut matched = role_permission_matches(pool, user_id, &module_code, &function_code).await?;
    matched.extend(role_menu_function_matches(pool, user_id, &module_code, &function_code).await?);
    let denied = matched
        .iter()
        .any(|item| item.get("effect").and_then(Value::as_str) == Some("DENY"));
    let allowed = !denied
        && matched
            .iter()
            .any(|item| item.get("effect").and_then(Value::as_str) == Some("ALLOW"));
    Ok(PermissionDecision {
        module_code,
        function_code,
        allowed,
        denied,
        matched,
    })
}

pub async fn has_permission(
    pool: &PgPool,
    user: &AuthUser,
    module_code: &str,
    function_code: &str,
) -> Result<bool> {
    if user.roles.iter().any(|role| role == "SUPER_ADMIN") {
        return Ok(true);
    }
    Ok(
        evaluate_permission(pool, user.user_id, module_code, function_code)
            .await?
            .allowed,
    )
}

pub async fn filtered_customers(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    user: &AuthUser,
) -> Result<Vec<Customer>> {
    ensure_tenant_access(user, tenant_ref)?;
    let mut customers = tenant::list_customers(pool, tenant_ref).await?;
    let scope = data_scope_for_user(pool, user, "customers").await?;
    if let Some(customer_ids) = visible_customer_ids(pool, tenant_ref, user, scope).await? {
        let allowed = customer_ids.into_iter().collect::<HashSet<_>>();
        customers.retain(|customer| allowed.contains(&customer.customer_id));
    }
    let can_mask_off = has_permission(pool, user, "customers", "MASK_OFF").await?;
    if !can_mask_off {
        for customer in &mut customers {
            customer.biz_reg_no = mask_identifier(&customer.biz_reg_no);
            customer.corp_reg_no = customer.corp_reg_no.as_deref().map(mask_identifier);
        }
    }
    Ok(customers)
}

pub async fn filtered_business_years(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    user: &AuthUser,
) -> Result<Vec<BusinessYear>> {
    ensure_tenant_access(user, tenant_ref)?;
    let mut years = tenant::list_business_years(pool, tenant_ref).await?;
    let scope = data_scope_for_user(pool, user, "business-years").await?;
    if let Some(customer_ids) = visible_customer_ids(pool, tenant_ref, user, scope).await? {
        let allowed = customer_ids.into_iter().collect::<HashSet<_>>();
        years.retain(|year| allowed.contains(&year.customer_id));
    }
    Ok(years)
}

pub async fn has_customer_work_scope(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    user: &AuthUser,
    customer_id: i64,
    work_scope: &str,
) -> Result<bool> {
    ensure_tenant_access(user, tenant_ref)?;
    if customer_id <= 0 {
        return Ok(false);
    }
    if user.roles.iter().any(|role| {
        matches!(
            role.as_str(),
            "SUPER_ADMIN" | "TENANT_ADMIN" | "SYSTEM_ADMIN"
        )
    }) {
        return Ok(true);
    }
    let work_scope = work_scope.trim().to_ascii_uppercase();
    if !matches!(
        work_scope.as_str(),
        "INFO" | "ADJUST" | "FORM" | "VALIDATE" | "APPROVE" | "PRINT" | "EFILE" | "POST"
    ) {
        return Err(anyhow!("invalid work_scope"));
    }
    let assigned = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM user_customer_access a
            JOIN user_customer_work_scope w ON w.access_id = a.access_id
            WHERE a.user_id = $1
              AND a.tenant_id = $2
              AND a.customer_id = $3
              AND a.access_level <> 'BLOCKED'
              AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
              AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
              AND w.work_scope = $4
        )
        "#,
    )
    .bind(user.user_id)
    .bind(tenant_ref.tenant_id)
    .bind(customer_id)
    .bind(&work_scope)
    .fetch_one(pool)
    .await
    .context("failed to check assigned customer work scope")?;
    if assigned {
        return Ok(true);
    }
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM access_delegations
            WHERE tenant_id = $1
              AND delegatee_user_id = $2
              AND customer_id = $3
              AND work_scope = $4
              AND status = 'ACTIVE'
              AND (valid_from IS NULL OR valid_from <= CURRENT_DATE)
              AND (valid_to IS NULL OR valid_to >= CURRENT_DATE)
        )
        "#,
    )
    .bind(tenant_ref.tenant_id)
    .bind(user.user_id)
    .bind(customer_id)
    .bind(&work_scope)
    .fetch_one(pool)
    .await
    .context("failed to check delegated customer work scope")
}

pub async fn verify_audit_chain(pool: &PgPool, tenant: &TenantRef) -> Result<Value> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        WITH ordered AS (
            SELECT audit_id, table_name, record_id, action, old_data, new_data, changed_by,
                   prev_hash, hash_current,
                   LAG(hash_current) OVER (ORDER BY audit_id) AS expected_prev_hash
            FROM {schema}.audit_logs
        )
        SELECT audit_id, prev_hash, expected_prev_hash, hash_current,
               md5(COALESCE(expected_prev_hash, '') || table_name || record_id || action ||
                   COALESCE(old_data::text, '') || COALESCE(new_data::text, '') || changed_by) AS expected_hash
        FROM ordered
        ORDER BY audit_id
        "#
    );
    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .context("failed to verify audit hash chain")?;
    let mut broken = Vec::new();
    for row in &rows {
        let audit_id = row.get::<i64, _>("audit_id");
        let prev_hash = row.get::<Option<String>, _>("prev_hash");
        let expected_prev_hash = row.get::<Option<String>, _>("expected_prev_hash");
        let hash_current = row.get::<Option<String>, _>("hash_current");
        let expected_hash = row.get::<Option<String>, _>("expected_hash");
        if prev_hash != expected_prev_hash || hash_current != expected_hash {
            broken.push(json!({
                "audit_id": audit_id,
                "prev_hash": prev_hash,
                "expected_prev_hash": expected_prev_hash,
                "hash_current": hash_current,
                "expected_hash": expected_hash
            }));
        }
    }
    Ok(json!({
        "tenant_code": tenant.tenant_code,
        "checked": rows.len(),
        "valid": broken.is_empty(),
        "broken": broken
    }))
}

async fn role_permission_matches(
    pool: &PgPool,
    user_id: i64,
    module_code: &str,
    function_code: &str,
) -> Result<Vec<Value>> {
    let rows = sqlx::query(
        r#"
        SELECT rp.role_code, rp.module_code, rp.function_code, rp.effect, 'ROLE_PERMISSION' AS source
        FROM role_permissions rp
        JOIN user_roles ur ON ur.role_code = rp.role_code
        WHERE ur.user_id = $1
          AND (
              rp.module_code = '*'
              OR rp.module_code = $2
              OR $2 LIKE rp.module_code || '.%'
          )
          AND (rp.function_code = '*' OR rp.function_code = $3)
        ORDER BY rp.effect DESC, rp.role_code
        "#,
    )
    .bind(user_id)
    .bind(module_code)
    .bind(function_code)
    .fetch_all(pool)
    .await
    .context("failed to evaluate role permissions")?;
    Ok(rows.into_iter().map(permission_match_json).collect())
}

async fn role_menu_function_matches(
    pool: &PgPool,
    user_id: i64,
    module_code: &str,
    function_code: &str,
) -> Result<Vec<Value>> {
    let rows = sqlx::query(
        r#"
        SELECT rmf.role_code, mn.menu_key AS module_code, rmf.function_code, rmf.effect,
               'ROLE_MENU_FUNCTION' AS source
        FROM role_menu_functions rmf
        JOIN user_roles ur ON ur.role_code = rmf.role_code
        JOIN menu_nodes mn ON mn.menu_key = rmf.menu_key
        WHERE ur.user_id = $1
          AND (mn.menu_key = $2 OR mn.required_perm_module = $2 OR mn.path LIKE '%' || $2 || '%')
          AND (rmf.function_code = '*' OR rmf.function_code = $3)
        "#,
    )
    .bind(user_id)
    .bind(module_code)
    .bind(function_code)
    .fetch_all(pool)
    .await
    .context("failed to evaluate role menu functions")?;
    Ok(rows.into_iter().map(permission_match_json).collect())
}

async fn data_scope_for_user(
    pool: &PgPool,
    user: &AuthUser,
    module_code: &str,
) -> Result<DataScope> {
    if user.roles.iter().any(|role| role == "SUPER_ADMIN") {
        return Ok(DataScope::All);
    }
    let rows = sqlx::query(
        r#"
        SELECT rds.data_scope
        FROM role_data_scopes rds
        JOIN user_roles ur ON ur.role_code = rds.role_code
        WHERE ur.user_id = $1
          AND (rds.module_code = '*' OR rds.module_code = $2)
        "#,
    )
    .bind(user.user_id)
    .bind(module_code)
    .fetch_all(pool)
    .await
    .context("failed to load role data scopes")?;
    if rows.is_empty() {
        return Ok(default_scope(user));
    }
    Ok(rows
        .into_iter()
        .map(|row| parse_scope(&row.get::<String, _>("data_scope")))
        .max_by_key(|scope| scope_rank(*scope))
        .unwrap_or(DataScope::None))
}

async fn visible_customer_ids(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    user: &AuthUser,
    scope: DataScope,
) -> Result<Option<Vec<i64>>> {
    match scope {
        DataScope::All => Ok(None),
        DataScope::None => Ok(Some(Vec::new())),
        DataScope::Assigned | DataScope::Owned => {
            let mut ids = assigned_customer_ids(pool, tenant_ref, user.user_id, scope).await?;
            ids.extend(delegated_customer_ids(pool, tenant_ref, user.user_id).await?);
            ids.sort_unstable();
            ids.dedup();
            Ok(Some(ids))
        }
    }
}

async fn assigned_customer_ids(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    user_id: i64,
    scope: DataScope,
) -> Result<Vec<i64>> {
    let owned_filter = if scope == DataScope::Owned {
        "AND (a.access_level = 'OWNER' OR a.is_primary = TRUE)"
    } else {
        ""
    };
    let sql = format!(
        r#"
        SELECT a.customer_id
        FROM user_customer_access a
        WHERE a.user_id = $1
          AND a.tenant_id = $2
          AND a.access_level <> 'BLOCKED'
          AND (a.valid_from IS NULL OR a.valid_from <= CURRENT_DATE)
          AND (a.valid_to IS NULL OR a.valid_to >= CURRENT_DATE)
          {owned_filter}
        "#
    );
    sqlx::query_scalar::<_, i64>(&sql)
        .bind(user_id)
        .bind(tenant_ref.tenant_id)
        .fetch_all(pool)
        .await
        .context("failed to load assigned customer ids")
}

async fn delegated_customer_ids(
    pool: &PgPool,
    tenant_ref: &TenantRef,
    user_id: i64,
) -> Result<Vec<i64>> {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT customer_id
        FROM access_delegations
        WHERE tenant_id = $1
          AND delegatee_user_id = $2
          AND status = 'ACTIVE'
          AND (valid_from IS NULL OR valid_from <= CURRENT_DATE)
          AND (valid_to IS NULL OR valid_to >= CURRENT_DATE)
        "#,
    )
    .bind(tenant_ref.tenant_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("failed to load delegated customer ids")
}

fn ensure_tenant_access(user: &AuthUser, tenant_ref: &TenantRef) -> Result<()> {
    if user.tenant_id == tenant_ref.tenant_id || user.roles.iter().any(|role| role == "SUPER_ADMIN")
    {
        Ok(())
    } else {
        Err(anyhow!("tenant access denied"))
    }
}

fn normalize_role(role_code: &str) -> String {
    role_code.trim().to_ascii_uppercase()
}

fn normalized_function_codes(functions: &[String]) -> Result<Vec<String>> {
    let mut normalized = functions
        .iter()
        .map(|function| {
            let code = function.trim().to_ascii_uppercase();
            if code.is_empty() {
                return Err(anyhow!("invalid function_code"));
            }
            Ok(code)
        })
        .collect::<Result<Vec<_>>>()?;
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

async fn insert_role_menu_function(
    pool: &PgPool,
    role_code: &str,
    grant: RoleMenuFunctionInput,
) -> Result<()> {
    let menu_key = grant.menu_key.trim();
    let function_code = grant.function_code.trim().to_ascii_uppercase();
    let effect = grant
        .effect
        .as_deref()
        .unwrap_or("ALLOW")
        .trim()
        .to_ascii_uppercase();
    if menu_key.is_empty()
        || function_code.is_empty()
        || !matches!(effect.as_str(), "ALLOW" | "DENY")
    {
        return Err(anyhow!("invalid role menu function grant"));
    }
    sqlx::query(
        r#"
        INSERT INTO role_menu_functions (role_code, menu_key, function_code, effect)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (role_code, menu_key, function_code)
        DO UPDATE SET effect = EXCLUDED.effect, updated_at = NOW()
        "#,
    )
    .bind(role_code)
    .bind(menu_key)
    .bind(function_code)
    .bind(effect)
    .execute(pool)
    .await
    .context("failed to insert role menu function")?;
    Ok(())
}

async fn user_id(pool: &PgPool, tenant_id: i64, login_id: &str) -> Result<i64> {
    sqlx::query_scalar::<_, i64>("SELECT user_id FROM users WHERE tenant_id = $1 AND login_id = $2")
        .bind(tenant_id)
        .bind(login_id.trim())
        .fetch_one(pool)
        .await
        .context("user not found")
}

fn role_menu_function_json(row: sqlx::postgres::PgRow) -> Value {
    json!({
        "role_code": row.get::<String, _>("role_code"),
        "menu_key": row.get::<String, _>("menu_key"),
        "function_code": row.get::<String, _>("function_code"),
        "effect": row.get::<String, _>("effect"),
        "updated_at": row.get::<chrono::DateTime<Utc>, _>("updated_at")
    })
}

fn permission_match_json(row: sqlx::postgres::PgRow) -> Value {
    json!({
        "role_code": row.get::<String, _>("role_code"),
        "module_code": row.get::<String, _>("module_code"),
        "function_code": row.get::<String, _>("function_code"),
        "effect": row.get::<String, _>("effect"),
        "source": row.get::<String, _>("source")
    })
}

fn default_scope(user: &AuthUser) -> DataScope {
    if user.roles.iter().any(|role| {
        matches!(
            role.as_str(),
            "SUPER_ADMIN" | "TENANT_ADMIN" | "SYSTEM_ADMIN"
        )
    }) {
        DataScope::All
    } else {
        DataScope::Assigned
    }
}

fn parse_scope(value: &str) -> DataScope {
    match value.trim().to_ascii_uppercase().as_str() {
        "ALL" => DataScope::All,
        "ASSIGNED" => DataScope::Assigned,
        "OWNED" => DataScope::Owned,
        _ => DataScope::None,
    }
}

fn scope_rank(scope: DataScope) -> u8 {
    match scope {
        DataScope::None => 0,
        DataScope::Owned => 1,
        DataScope::Assigned => 2,
        DataScope::All => 3,
    }
}

fn mask_identifier(value: &str) -> String {
    let visible = value.chars().rev().take(4).collect::<Vec<_>>();
    if visible.is_empty() {
        MASKED_VALUE.to_string()
    } else {
        let suffix = visible.into_iter().rev().collect::<String>();
        format!("***{suffix}")
    }
}
