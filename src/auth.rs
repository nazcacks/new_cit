use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{AuthUser, LoginRequest, LoginResponse},
    modules,
};

pub async fn login(pool: &PgPool, request: LoginRequest) -> Result<LoginResponse> {
    let user = sqlx::query_as::<_, AuthUser>(
        r#"
        SELECT
            u.user_id,
            t.tenant_id,
            t.tenant_code,
            t.tenant_name,
            t.schema_name,
            u.login_id,
            u.user_name,
            u.email,
            u.status,
            u.use_2fa,
            ARRAY['SYSTEM_ADMIN', 'TAX_MANAGER']::TEXT[] AS roles
        FROM users u
        JOIN tenants t ON t.tenant_id = u.tenant_id
        WHERE t.tenant_code = $1
          AND u.login_id = $2
          AND u.password_hash = crypt($3, u.password_hash)
          AND u.status = 'ACTIVE'
          AND t.status = 'ACTIVE'
          AND u.locked = FALSE
        "#,
    )
    .bind(request.tenant_code.trim().to_ascii_lowercase())
    .bind(request.login_id.trim())
    .bind(request.password)
    .fetch_optional(pool)
    .await
    .context("failed to verify login")?
    .ok_or_else(|| anyhow!("invalid tenant, login id, or password"))?;

    let token = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO auth_sessions (user_id, tenant_id)
        VALUES ($1, $2)
        RETURNING session_token
        "#,
    )
    .bind(user.user_id)
    .bind(user.tenant_id)
    .fetch_one(pool)
    .await
    .context("failed to create session")?;

    sqlx::query(
        r#"
        UPDATE users
        SET last_login_at = NOW(),
            pwd_fail_count = 0
        WHERE user_id = $1
        "#,
    )
    .bind(user.user_id)
    .execute(pool)
    .await
    .context("failed to update login timestamp")?;

    record_login(pool, Some(user.user_id), true, None).await?;

    let expires_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        "SELECT expires_at FROM auth_sessions WHERE session_token = $1",
    )
    .bind(token)
    .fetch_one(pool)
    .await
    .context("failed to load session expiry")?;

    Ok(LoginResponse {
        token,
        token_type: "Bearer",
        expires_at,
        user,
        modules: modules::module_tree(),
    })
}

pub async fn logout(pool: &PgPool, token: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE auth_sessions
        SET revoked_at = NOW()
        WHERE session_token = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(token)
    .execute(pool)
    .await
    .context("failed to revoke session")?;
    Ok(())
}

pub async fn me(pool: &PgPool, token: Uuid) -> Result<LoginResponse> {
    let user = sqlx::query_as::<_, AuthUser>(
        r#"
        SELECT
            u.user_id,
            t.tenant_id,
            t.tenant_code,
            t.tenant_name,
            t.schema_name,
            u.login_id,
            u.user_name,
            u.email,
            u.status,
            u.use_2fa,
            ARRAY['SYSTEM_ADMIN', 'TAX_MANAGER']::TEXT[] AS roles
        FROM auth_sessions s
        JOIN users u ON u.user_id = s.user_id
        JOIN tenants t ON t.tenant_id = s.tenant_id
        WHERE s.session_token = $1
          AND s.revoked_at IS NULL
          AND s.expires_at > NOW()
          AND u.status = 'ACTIVE'
          AND t.status = 'ACTIVE'
        "#,
    )
    .bind(token)
    .fetch_optional(pool)
    .await
    .context("failed to load session")?
    .ok_or_else(|| anyhow!("invalid or expired session"))?;

    sqlx::query("UPDATE auth_sessions SET last_seen_at = NOW() WHERE session_token = $1")
        .bind(token)
        .execute(pool)
        .await
        .context("failed to touch session")?;

    let expires_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        "SELECT expires_at FROM auth_sessions WHERE session_token = $1",
    )
    .bind(token)
    .fetch_one(pool)
    .await
    .context("failed to load session expiry")?;

    Ok(LoginResponse {
        token,
        token_type: "Bearer",
        expires_at,
        user,
        modules: modules::module_tree(),
    })
}

pub async fn record_failed_login(pool: &PgPool, tenant_code: &str, login_id: &str) -> Result<()> {
    let user_id = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT u.user_id
        FROM users u
        JOIN tenants t ON t.tenant_id = u.tenant_id
        WHERE t.tenant_code = $1 AND u.login_id = $2
        "#,
    )
    .bind(tenant_code.trim().to_ascii_lowercase())
    .bind(login_id.trim())
    .fetch_optional(pool)
    .await
    .context("failed to find failed-login user")?;

    if let Some(user_id) = user_id {
        sqlx::query("UPDATE users SET pwd_fail_count = pwd_fail_count + 1 WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .context("failed to increment failed-login count")?;
    }

    record_login(pool, user_id, false, Some("INVALID_CREDENTIALS")).await
}

async fn record_login(
    pool: &PgPool,
    user_id: Option<i64>,
    success: bool,
    fail_reason: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO login_history (user_id, success, fail_reason, session_id)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(success)
    .bind(fail_reason)
    .bind(if success { Some("web") } else { None })
    .execute(pool)
    .await
    .context("failed to insert login history")?;
    Ok(())
}

pub fn parse_bearer_token(value: Option<&str>) -> Result<Uuid> {
    let Some(value) = value else {
        return Err(anyhow!("missing authorization token"));
    };
    let token = value
        .strip_prefix("Bearer ")
        .ok_or_else(|| anyhow!("authorization token must use Bearer scheme"))?;
    Uuid::parse_str(token).context("authorization token must be a UUID")
}
