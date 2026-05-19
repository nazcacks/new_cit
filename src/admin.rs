use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use crate::{
    db::quote_ident,
    domain::{
        AdminUser, CreateAdminUserRequest, Role, RolePermission, RolePermissionInput, TenantRef,
        UpdateAdminUserRequest, UpdateAdminUserStatusRequest, UpdateRolePermissionsRequest,
        UserCustomerAccess, UserCustomerAccessInput,
    },
    tenant,
};

const DEFAULT_ROLE: &str = "ASSISTANT";
const DEFAULT_ACCESS_LEVEL: &str = "VIEWER";
const DEFAULT_WORK_SCOPES: &[&str] = &["INFO", "ADJUST", "FORM", "VALIDATE", "PRINT"];
const ALLOWED_STATUSES: &[&str] = &["ACTIVE", "LOCKED", "WITHDRAWN"];
const ALLOWED_ACCESS_LEVELS: &[&str] = &[
    "OWNER",
    "CO_WORKER",
    "REVIEWER",
    "ASSISTANT",
    "VIEWER",
    "BLOCKED",
];
const ALLOWED_WORK_SCOPES: &[&str] = &[
    "INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST",
];
const ALLOWED_EFFECTS: &[&str] = &["ALLOW", "DENY"];

pub async fn list_users(pool: &PgPool, tenant_code: &str) -> Result<Vec<AdminUser>> {
    let tenant = tenant::resolve_tenant(pool, tenant_code).await?;
    let rows = sqlx::query(
        r#"
        SELECT
            u.user_id,
            u.tenant_id,
            t.tenant_code,
            u.login_id,
            u.user_name,
            u.email,
            u.phone,
            u.status,
            u.locked,
            u.use_2fa,
            u.pwd_fail_count,
            u.last_login_at,
            COALESCE(
                ARRAY_AGG(ur.role_code ORDER BY ur.role_code)
                    FILTER (WHERE ur.role_code IS NOT NULL),
                ARRAY[]::TEXT[]
            ) AS roles
        FROM users u
        JOIN tenants t ON t.tenant_id = u.tenant_id
        LEFT JOIN user_roles ur ON ur.user_id = u.user_id
        WHERE u.tenant_id = $1
        GROUP BY u.user_id, t.tenant_code
        ORDER BY u.login_id
        "#,
    )
    .bind(tenant.tenant_id)
    .fetch_all(pool)
    .await
    .context("failed to list admin users")?;

    let mut users = Vec::with_capacity(rows.len());
    for row in rows {
        let user_id = row.get::<i64, _>("user_id");
        users.push(AdminUser {
            user_id,
            tenant_id: row.get("tenant_id"),
            tenant_code: row.get("tenant_code"),
            login_id: row.get("login_id"),
            user_name: row.get("user_name"),
            email: row.get("email"),
            phone: row.get("phone"),
            status: row.get("status"),
            locked: row.get("locked"),
            use_2fa: row.get("use_2fa"),
            pwd_fail_count: row.get("pwd_fail_count"),
            last_login_at: row.get("last_login_at"),
            roles: row.get("roles"),
            customer_access: load_customer_access(pool, user_id).await?,
        });
    }
    Ok(users)
}

pub async fn create_user(
    pool: &PgPool,
    tenant_code: &str,
    request: CreateAdminUserRequest,
) -> Result<AdminUser> {
    let tenant = tenant::resolve_tenant(pool, tenant_code).await?;
    let login_id = normalized_required(&request.login_id, "login_id")?;
    let user_name = normalized_required(&request.user_name, "user_name")?;
    if request.password.len() < 8 {
        return Err(anyhow!("invalid password: at least 8 characters required"));
    }
    let status = normalize_choice(
        request.status.as_deref().unwrap_or("ACTIVE"),
        ALLOWED_STATUSES,
        "status",
    )?;
    let row = sqlx::query(
        r#"
        INSERT INTO users (
            tenant_id, login_id, password_hash, user_name, email, phone,
            use_2fa, totp_secret, status, pwd_changed_at
        )
        VALUES ($1, $2, crypt($3, gen_salt('bf')), $4, $5, $6, COALESCE($7, TRUE), $8, $9, NOW())
        RETURNING user_id
        "#,
    )
    .bind(tenant.tenant_id)
    .bind(&login_id)
    .bind(request.password)
    .bind(user_name)
    .bind(request.email)
    .bind(request.phone)
    .bind(request.use_2fa)
    .bind(request.totp_secret)
    .bind(status)
    .fetch_one(pool)
    .await
    .context("failed to create admin user")?;
    let user_id = row.get::<i64, _>("user_id");

    replace_roles(pool, user_id, request.roles.as_deref()).await?;
    replace_customer_access(
        pool,
        &tenant,
        user_id,
        request.customer_access.as_deref().unwrap_or(&[]),
    )
    .await?;
    audit(
        pool,
        "system",
        "USER_CREATE",
        &format!("{tenant_code}/{login_id}"),
        json!({ "tenant_id": tenant.tenant_id }),
    )
    .await?;
    get_user(pool, tenant_code, &login_id).await
}

pub async fn update_user(
    pool: &PgPool,
    tenant_code: &str,
    login_id: &str,
    request: UpdateAdminUserRequest,
) -> Result<AdminUser> {
    let tenant = tenant::resolve_tenant(pool, tenant_code).await?;
    let user = find_user_id(pool, tenant.tenant_id, login_id).await?;

    sqlx::query(
        r#"
        UPDATE users
        SET user_name = COALESCE($3, user_name),
            email = COALESCE($4, email),
            phone = COALESCE($5, phone),
            use_2fa = COALESCE($6, use_2fa),
            totp_secret = COALESCE($7, totp_secret)
        WHERE tenant_id = $1 AND login_id = $2
        "#,
    )
    .bind(tenant.tenant_id)
    .bind(login_id)
    .bind(request.user_name)
    .bind(request.email)
    .bind(request.phone)
    .bind(request.use_2fa)
    .bind(request.totp_secret)
    .execute(pool)
    .await
    .context("failed to update admin user")?;

    if let Some(roles) = request.roles.as_deref() {
        replace_roles(pool, user, Some(roles)).await?;
    }
    if let Some(access) = request.customer_access.as_deref() {
        replace_customer_access(pool, &tenant, user, access).await?;
    }
    audit(
        pool,
        "system",
        "USER_UPDATE",
        &format!("{tenant_code}/{login_id}"),
        json!({ "tenant_id": tenant.tenant_id }),
    )
    .await?;
    get_user(pool, tenant_code, login_id).await
}

pub async fn update_user_status(
    pool: &PgPool,
    tenant_code: &str,
    login_id: &str,
    request: UpdateAdminUserStatusRequest,
) -> Result<AdminUser> {
    let tenant = tenant::resolve_tenant(pool, tenant_code).await?;
    if let Some(status) = request.status.as_deref() {
        normalize_choice(status, ALLOWED_STATUSES, "status")?;
    }
    sqlx::query(
        r#"
        UPDATE users
        SET status = COALESCE($3, status),
            locked = COALESCE($4, locked),
            pwd_fail_count = CASE
                WHEN COALESCE($4, locked) = FALSE OR COALESCE($3, status) = 'ACTIVE' THEN 0
                ELSE pwd_fail_count
            END
        WHERE tenant_id = $1 AND login_id = $2
        "#,
    )
    .bind(tenant.tenant_id)
    .bind(login_id)
    .bind(
        request
            .status
            .map(|value| value.trim().to_ascii_uppercase()),
    )
    .bind(request.locked)
    .execute(pool)
    .await
    .context("failed to update user status")?;
    audit(
        pool,
        "system",
        "USER_STATUS_UPDATE",
        &format!("{tenant_code}/{login_id}"),
        json!({ "locked": request.locked }),
    )
    .await?;
    get_user(pool, tenant_code, login_id).await
}

pub async fn reset_2fa(pool: &PgPool, tenant_code: &str, login_id: &str) -> Result<AdminUser> {
    let tenant = tenant::resolve_tenant(pool, tenant_code).await?;
    sqlx::query(
        r#"
        UPDATE users
        SET totp_secret = NULL,
            use_2fa = FALSE
        WHERE tenant_id = $1 AND login_id = $2
        "#,
    )
    .bind(tenant.tenant_id)
    .bind(login_id)
    .execute(pool)
    .await
    .context("failed to reset 2fa")?;
    audit(
        pool,
        "system",
        "USER_2FA_RESET",
        &format!("{tenant_code}/{login_id}"),
        json!({}),
    )
    .await?;
    get_user(pool, tenant_code, login_id).await
}

pub async fn get_user(pool: &PgPool, tenant_code: &str, login_id: &str) -> Result<AdminUser> {
    list_users(pool, tenant_code)
        .await?
        .into_iter()
        .find(|user| user.login_id == login_id)
        .ok_or_else(|| anyhow!("user not found"))
}

pub async fn list_roles(pool: &PgPool) -> Result<Vec<Role>> {
    sqlx::query_as::<_, Role>(
        r#"
        SELECT role_code, role_name, description, system_role, created_at
        FROM roles
        ORDER BY system_role DESC, role_code
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list roles")
}

pub async fn list_role_permissions(pool: &PgPool) -> Result<Vec<RolePermission>> {
    sqlx::query_as::<_, RolePermission>(
        r#"
        SELECT role_code, module_code, function_code, effect, updated_at
        FROM role_permissions
        ORDER BY role_code, module_code, function_code
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list role permissions")
}

pub async fn replace_role_permissions(
    pool: &PgPool,
    role_code: &str,
    request: UpdateRolePermissionsRequest,
) -> Result<Vec<RolePermission>> {
    let role_code = normalize_role(role_code);
    sqlx::query("SELECT role_code FROM roles WHERE role_code = $1")
        .bind(&role_code)
        .fetch_one(pool)
        .await
        .context("role not found")?;

    sqlx::query("DELETE FROM role_permissions WHERE role_code = $1")
        .bind(&role_code)
        .execute(pool)
        .await
        .context("failed to clear role permissions")?;

    for permission in request.permissions {
        insert_role_permission(pool, &role_code, permission).await?;
    }
    audit(
        pool,
        "system",
        "ROLE_PERMISSION_REPLACE",
        &role_code,
        json!({}),
    )
    .await?;
    list_role_permissions(pool).await
}

async fn insert_role_permission(
    pool: &PgPool,
    role_code: &str,
    permission: RolePermissionInput,
) -> Result<()> {
    let module_code = normalized_required(&permission.module_code, "module_code")?;
    let function_code =
        normalized_required(&permission.function_code, "function_code")?.to_ascii_uppercase();
    let effect = normalize_choice(
        permission.effect.as_deref().unwrap_or("ALLOW"),
        ALLOWED_EFFECTS,
        "effect",
    )?;
    sqlx::query(
        r#"
        INSERT INTO role_permissions (role_code, module_code, function_code, effect)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (role_code, module_code, function_code)
        DO UPDATE SET effect = EXCLUDED.effect, updated_at = NOW()
        "#,
    )
    .bind(role_code)
    .bind(module_code)
    .bind(function_code)
    .bind(effect)
    .execute(pool)
    .await
    .context("failed to insert role permission")?;
    Ok(())
}

async fn replace_roles(pool: &PgPool, user_id: i64, roles: Option<&[String]>) -> Result<()> {
    let roles = roles
        .filter(|items| !items.is_empty())
        .map(|items| {
            items
                .iter()
                .map(|role| normalize_role(role))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![DEFAULT_ROLE.to_string()]);

    sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .context("failed to clear user roles")?;

    for role in roles {
        sqlx::query(
            r#"
            INSERT INTO user_roles (user_id, role_code, granted_by)
            VALUES ($1, $2, 'system')
            "#,
        )
        .bind(user_id)
        .bind(role)
        .execute(pool)
        .await
        .context("failed to insert user role")?;
    }
    Ok(())
}

async fn replace_customer_access(
    pool: &PgPool,
    tenant: &TenantRef,
    user_id: i64,
    access_items: &[UserCustomerAccessInput],
) -> Result<()> {
    let mut prepared = Vec::with_capacity(access_items.len());
    for item in access_items {
        if item.customer_id <= 0 {
            return Err(anyhow!("invalid customer_id"));
        }
        let customer_work_scopes =
            load_customer_work_scopes(pool, tenant, item.customer_id).await?;
        let access_level = normalize_choice(
            item.access_level.as_deref().unwrap_or(DEFAULT_ACCESS_LEVEL),
            ALLOWED_ACCESS_LEVELS,
            "access_level",
        )?;
        let work_scopes = if item.work_scopes.is_some() {
            normalized_work_scopes(item.work_scopes.as_deref())?
        } else {
            default_work_scopes_for_customer(&customer_work_scopes)
        };
        for scope in &work_scopes {
            if !customer_work_scopes.contains(scope) {
                return Err(anyhow!(
                    "invalid work_scope for customer target work scope: {scope}"
                ));
            }
        }
        prepared.push((
            item.customer_id,
            access_level,
            item.is_primary.unwrap_or(false),
            item.valid_from,
            item.valid_to,
            work_scopes,
        ));
    }

    sqlx::query("DELETE FROM user_customer_access WHERE user_id = $1 AND tenant_id = $2")
        .bind(user_id)
        .bind(tenant.tenant_id)
        .execute(pool)
        .await
        .context("failed to clear user customer access")?;

    for (customer_id, access_level, is_primary, valid_from, valid_to, work_scopes) in prepared {
        let access_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO user_customer_access (
                user_id, tenant_id, customer_id, access_level, is_primary, valid_from, valid_to
            )
            VALUES ($1, $2, $3, $4, COALESCE($5, FALSE), $6, $7)
            RETURNING access_id
            "#,
        )
        .bind(user_id)
        .bind(tenant.tenant_id)
        .bind(customer_id)
        .bind(access_level)
        .bind(is_primary)
        .bind(valid_from)
        .bind(valid_to)
        .fetch_one(pool)
        .await
        .context("failed to insert user customer access")?;

        for scope in work_scopes {
            sqlx::query(
                r#"
                INSERT INTO user_customer_work_scope (access_id, work_scope)
                VALUES ($1, $2)
                "#,
            )
            .bind(access_id)
            .bind(scope)
            .execute(pool)
            .await
            .context("failed to insert user customer work scope")?;
        }
    }
    Ok(())
}

async fn load_customer_work_scopes(
    pool: &PgPool,
    tenant: &TenantRef,
    customer_id: i64,
) -> Result<Vec<String>> {
    let schema = quote_ident(&tenant.schema_name)?;
    let sql = format!(
        r#"
        SELECT work_scopes
        FROM {schema}.customers
        WHERE tenant_id = $1 AND customer_id = $2 AND status = 'ACTIVE'
        "#
    );

    sqlx::query_scalar::<_, Vec<String>>(&sql)
        .bind(tenant.tenant_id)
        .bind(customer_id)
        .fetch_one(pool)
        .await
        .context("customer not found for tenant")
}

async fn load_customer_access(pool: &PgPool, user_id: i64) -> Result<Vec<UserCustomerAccess>> {
    let rows = sqlx::query(
        r#"
        SELECT
            a.customer_id,
            a.access_level,
            a.is_primary,
            a.valid_from,
            a.valid_to,
            COALESCE(
                ARRAY_AGG(w.work_scope ORDER BY w.work_scope)
                    FILTER (WHERE w.work_scope IS NOT NULL),
                ARRAY[]::TEXT[]
            ) AS work_scopes
        FROM user_customer_access a
        LEFT JOIN user_customer_work_scope w ON w.access_id = a.access_id
        WHERE a.user_id = $1
        GROUP BY a.access_id
        ORDER BY a.customer_id
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("failed to load user customer access")?;

    Ok(rows
        .into_iter()
        .map(|row| UserCustomerAccess {
            customer_id: row.get("customer_id"),
            access_level: row.get("access_level"),
            is_primary: row.get("is_primary"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            work_scopes: row.get("work_scopes"),
        })
        .collect())
}

async fn find_user_id(pool: &PgPool, tenant_id: i64, login_id: &str) -> Result<i64> {
    sqlx::query_scalar::<_, i64>("SELECT user_id FROM users WHERE tenant_id = $1 AND login_id = $2")
        .bind(tenant_id)
        .bind(login_id)
        .fetch_one(pool)
        .await
        .context("user not found")
}

fn normalized_required(value: &str, field: &str) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(anyhow!("invalid {field}"));
    }
    Ok(normalized.to_string())
}

fn normalize_role(role: &str) -> String {
    role.trim().to_ascii_uppercase()
}

fn normalize_choice(value: &str, allowed: &[&str], field: &str) -> Result<String> {
    let normalized = value.trim().to_ascii_uppercase();
    if !allowed.contains(&normalized.as_str()) {
        return Err(anyhow!("invalid {field}"));
    }
    Ok(normalized)
}

fn normalized_work_scopes(scopes: Option<&[String]>) -> Result<Vec<String>> {
    let mut normalized = scopes
        .filter(|items| !items.is_empty())
        .map(|items| {
            items
                .iter()
                .map(|scope| normalize_choice(scope, ALLOWED_WORK_SCOPES, "work_scope"))
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_else(|| {
            DEFAULT_WORK_SCOPES
                .iter()
                .map(|scope| (*scope).to_string())
                .collect()
        });
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn default_work_scopes_for_customer(customer_work_scopes: &[String]) -> Vec<String> {
    let mut scopes = DEFAULT_WORK_SCOPES
        .iter()
        .filter(|scope| {
            customer_work_scopes
                .iter()
                .any(|allowed| allowed == **scope)
        })
        .map(|scope| (*scope).to_string())
        .collect::<Vec<_>>();
    if scopes.is_empty() {
        scopes = customer_work_scopes.to_vec();
    }
    scopes
}

async fn audit(
    pool: &PgPool,
    actor: &str,
    action: &str,
    target: &str,
    metadata: Value,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO admin_audit_events (actor, action, target, metadata)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(actor)
    .bind(action)
    .bind(target)
    .bind(metadata)
    .execute(pool)
    .await
    .context("failed to insert admin audit event")?;
    Ok(())
}
