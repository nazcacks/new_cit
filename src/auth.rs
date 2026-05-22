use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    domain::{AccessibleTenant, AuthUser, LoginRequest, LoginResponse},
    modules, tenant,
};

const TOTP_STEP_SECONDS: i64 = 30;
const TOTP_WINDOW: i64 = 1;
const TOTP_DIGITS: u32 = 6;
const PASSWORD_MAX_AGE_DAYS: i64 = 90;

#[derive(sqlx::FromRow)]
struct LoginCandidate {
    user_id: i64,
    tenant_id: i64,
    tenant_code: String,
    tenant_name: String,
    schema_name: String,
    login_id: String,
    user_name: String,
    email: Option<String>,
    status: String,
    use_2fa: bool,
    locked: bool,
    pwd_changed_at: Option<DateTime<Utc>>,
    roles: Vec<String>,
}

pub async fn login(
    pool: &PgPool,
    request: LoginRequest,
    client_ip: Option<&str>,
) -> Result<LoginResponse> {
    let user = sqlx::query_as::<_, LoginCandidate>(
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
            u.locked,
            u.pwd_changed_at,
            COALESCE(
                ARRAY_AGG(ur.role_code ORDER BY ur.role_code)
                    FILTER (WHERE ur.role_code IS NOT NULL),
                ARRAY[]::TEXT[]
            ) AS roles
        FROM users u
        JOIN tenants t ON t.tenant_id = u.tenant_id
        LEFT JOIN user_roles ur ON ur.user_id = u.user_id
        WHERE t.tenant_code = $1
          AND u.login_id = $2
          AND u.password_hash = crypt($3, u.password_hash)
          AND t.status = 'ACTIVE'
        GROUP BY u.user_id, t.tenant_id
        "#,
    )
    .bind(request.tenant_code.trim().to_ascii_lowercase())
    .bind(request.login_id.trim())
    .bind(request.password)
    .fetch_optional(pool)
    .await
    .context("failed to verify login")?
    .ok_or_else(|| anyhow!("invalid tenant, login id, or password"))?;

    enforce_login_candidate_state(&user)?;
    enforce_ip_allowlist(pool, user.tenant_id, client_ip).await?;
    enforce_2fa_for_user(pool, user.user_id, user.use_2fa, request.otp.as_deref()).await?;

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
            last_login_ip = $2,
            pwd_fail_count = 0
        WHERE user_id = $1
        "#,
    )
    .bind(user.user_id)
    .bind(client_ip)
    .execute(pool)
    .await
    .context("failed to update login timestamp")?;

    record_login(pool, Some(user.user_id), client_ip, None, true, None).await?;

    let expires_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        "SELECT expires_at FROM auth_sessions WHERE session_token = $1",
    )
    .bind(token)
    .fetch_one(pool)
    .await
    .context("failed to load session expiry")?;

    let auth_user = AuthUser {
        user_id: user.user_id,
        tenant_id: user.tenant_id,
        tenant_code: user.tenant_code,
        tenant_name: user.tenant_name,
        schema_name: user.schema_name,
        login_id: user.login_id,
        user_name: user.user_name,
        email: user.email,
        status: user.status,
        use_2fa: user.use_2fa,
        roles: user.roles,
    };

    Ok(LoginResponse {
        token,
        token_type: "Bearer",
        expires_at,
        accessible_tenants: accessible_tenants(pool, &auth_user).await?,
        user: auth_user,
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
            COALESCE(
                ARRAY_AGG(ur.role_code ORDER BY ur.role_code)
                    FILTER (WHERE ur.role_code IS NOT NULL),
                ARRAY[]::TEXT[]
            ) AS roles
        FROM auth_sessions s
        JOIN users u ON u.user_id = s.user_id
        JOIN tenants t ON t.tenant_id = s.tenant_id
        LEFT JOIN user_roles ur ON ur.user_id = u.user_id
        WHERE s.session_token = $1
          AND s.revoked_at IS NULL
          AND s.expires_at > NOW()
          AND u.status = 'ACTIVE'
          AND t.status = 'ACTIVE'
        GROUP BY u.user_id, t.tenant_id, s.session_token
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
        accessible_tenants: accessible_tenants(pool, &user).await?,
        user,
        modules: modules::module_tree(),
    })
}

pub async fn switch_tenant(
    pool: &PgPool,
    current_token: Uuid,
    user: &AuthUser,
    target_tenant_code: &str,
) -> Result<LoginResponse> {
    let target_tenant_code = tenant::normalize_tenant_code(target_tenant_code)?;
    let tenants = accessible_tenants(pool, user).await?;
    let target = tenants
        .iter()
        .find(|item| item.tenant_code == target_tenant_code)
        .ok_or_else(|| anyhow!("tenant switch denied"))?;
    if target.current {
        return me(pool, current_token).await;
    }
    let token = create_session(pool, user.user_id, target.tenant_id).await?;
    sqlx::query(
        r#"
        UPDATE auth_sessions
        SET revoked_at = NOW()
        WHERE session_token = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(current_token)
    .execute(pool)
    .await
    .context("failed to revoke previous tenant session")?;
    me(pool, token).await
}

async fn create_session(pool: &PgPool, user_id: i64, tenant_id: i64) -> Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO auth_sessions (user_id, tenant_id)
        VALUES ($1, $2)
        RETURNING session_token
        "#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .context("failed to create tenant switch session")
}

pub async fn accessible_tenants(pool: &PgPool, user: &AuthUser) -> Result<Vec<AccessibleTenant>> {
    if user.roles.iter().any(|role| role == "SUPER_ADMIN") {
        let rows = sqlx::query(
            r#"
            SELECT tenant_id, tenant_code, tenant_name, 'SUPER_ADMIN'::TEXT AS role,
                   tenant_id = $1 AS current
            FROM tenants
            WHERE status = 'ACTIVE'
            ORDER BY tenant_code
            "#,
        )
        .bind(user.tenant_id)
        .fetch_all(pool)
        .await
        .context("failed to load super-admin accessible tenants")?;
        return Ok(rows.into_iter().map(accessible_tenant_from_row).collect());
    }

    let rows = sqlx::query(
        r#"
        SELECT
            t.tenant_id,
            t.tenant_code,
            t.tenant_name,
            COALESCE(
                (ARRAY_AGG(ur.role_code ORDER BY ur.role_code)
                    FILTER (WHERE ur.role_code IS NOT NULL))[1],
                'USER'
            ) AS role,
            t.tenant_id = $1 AS current
        FROM tenants t
        JOIN users u ON u.tenant_id = t.tenant_id
        LEFT JOIN user_roles ur ON ur.user_id = u.user_id
        WHERE t.status = 'ACTIVE'
          AND u.status = 'ACTIVE'
          AND u.locked = FALSE
          AND u.login_id = $2
        GROUP BY t.tenant_id, t.tenant_code, t.tenant_name
        ORDER BY t.tenant_code
        "#,
    )
    .bind(user.tenant_id)
    .bind(&user.login_id)
    .fetch_all(pool)
    .await
    .context("failed to load accessible tenants")?;
    let tenants = rows
        .into_iter()
        .map(accessible_tenant_from_row)
        .collect::<Vec<_>>();
    if tenants.is_empty() {
        Ok(vec![AccessibleTenant {
            tenant_id: user.tenant_id,
            tenant_code: user.tenant_code.clone(),
            tenant_name: user.tenant_name.clone(),
            role: user
                .roles
                .first()
                .cloned()
                .unwrap_or_else(|| "USER".to_string()),
            current: true,
        }])
    } else {
        Ok(tenants)
    }
}

fn accessible_tenant_from_row(row: sqlx::postgres::PgRow) -> AccessibleTenant {
    AccessibleTenant {
        tenant_id: row.get("tenant_id"),
        tenant_code: row.get("tenant_code"),
        tenant_name: row.get("tenant_name"),
        role: row.get("role"),
        current: row.get("current"),
    }
}

fn enforce_login_candidate_state(user: &LoginCandidate) -> Result<()> {
    if user.locked || user.status.eq_ignore_ascii_case("LOCKED") {
        return Err(anyhow!("account locked"));
    }
    if !user.status.eq_ignore_ascii_case("ACTIVE") {
        return Err(anyhow!("account is not active"));
    }
    enforce_password_freshness(user.pwd_changed_at)
}

fn enforce_password_freshness(pwd_changed_at: Option<DateTime<Utc>>) -> Result<()> {
    let Some(pwd_changed_at) = pwd_changed_at else {
        return Err(anyhow!("password expired"));
    };
    let age_days = Utc::now()
        .signed_duration_since(pwd_changed_at)
        .num_days();
    if age_days >= PASSWORD_MAX_AGE_DAYS {
        Err(anyhow!("password expired"))
    } else {
        Ok(())
    }
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
        sqlx::query(
            r#"
            UPDATE users
            SET pwd_fail_count = pwd_fail_count + 1,
                locked = CASE WHEN pwd_fail_count + 1 >= 5 THEN TRUE ELSE locked END,
                status = CASE WHEN pwd_fail_count + 1 >= 5 THEN 'LOCKED' ELSE status END
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await
        .context("failed to increment failed-login count")?;
    }

    record_login(
        pool,
        user_id,
        None,
        None,
        false,
        Some("INVALID_CREDENTIALS"),
    )
    .await
}

async fn record_login(
    pool: &PgPool,
    user_id: Option<i64>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    success: bool,
    fail_reason: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO login_history (user_id, ip_address, user_agent, success, fail_reason, session_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(user_id)
    .bind(ip_address)
    .bind(user_agent)
    .bind(success)
    .bind(fail_reason)
    .bind(if success { Some("web") } else { None })
    .execute(pool)
    .await
    .context("failed to insert login history")?;
    Ok(())
}

pub async fn enforce_ip_allowlist(
    pool: &PgPool,
    tenant_id: i64,
    client_ip: Option<&str>,
) -> Result<()> {
    let allowed_ips = sqlx::query_scalar::<_, Option<String>>(
        "SELECT allowed_ips FROM tenants WHERE tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .context("failed to load tenant IP allowlist")?;

    let Some(allowed_ips) = allowed_ips.filter(|value| !value.trim().is_empty()) else {
        return Ok(());
    };
    let client_ip = client_ip
        .and_then(|value| first_ip(value).ok())
        .unwrap_or_else(|| "127.0.0.1".to_string());
    if ip_allowed(&client_ip, &allowed_ips) {
        Ok(())
    } else {
        Err(anyhow!("client IP is not allowed for this tenant"))
    }
}

pub async fn enforce_2fa_for_user(
    pool: &PgPool,
    user_id: i64,
    use_2fa: bool,
    otp: Option<&str>,
) -> Result<()> {
    if !use_2fa {
        return Ok(());
    }
    let code = otp
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("2fa otp is required"))?;
    let secret =
        sqlx::query_scalar::<_, Option<String>>("SELECT totp_secret FROM users WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .context("failed to load 2fa secret")?
            .ok_or_else(|| anyhow!("2fa enrollment is required"))?;
    if verify_totp(pool, &secret, code).await? {
        Ok(())
    } else {
        Err(anyhow!("invalid 2fa otp"))
    }
}

pub async fn verify_totp(pool: &PgPool, secret: &str, code: &str) -> Result<bool> {
    let code = code.trim();
    if code.len() != TOTP_DIGITS as usize
        || !code.chars().all(|character| character.is_ascii_digit())
    {
        return Ok(false);
    }
    let secret_bytes = decode_totp_secret(secret)?;
    let counter = Utc::now().timestamp() / TOTP_STEP_SECONDS;
    for drift in -TOTP_WINDOW..=TOTP_WINDOW {
        if hotp(pool, &secret_bytes, counter + drift).await? == code {
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn hotp(pool: &PgPool, secret: &[u8], counter: i64) -> Result<String> {
    if counter < 0 {
        return Ok("000000".to_string());
    }
    let message = (counter as u64).to_be_bytes().to_vec();
    let digest = sqlx::query_scalar::<_, Vec<u8>>("SELECT hmac($1::bytea, $2::bytea, 'sha1')")
        .bind(message)
        .bind(secret)
        .fetch_one(pool)
        .await
        .context("failed to calculate totp hmac")?;
    if digest.len() < 20 {
        return Err(anyhow!("invalid totp hmac length"));
    }
    let offset = usize::from(digest[19] & 0x0f);
    let binary = (u32::from(digest[offset] & 0x7f) << 24)
        | (u32::from(digest[offset + 1]) << 16)
        | (u32::from(digest[offset + 2]) << 8)
        | u32::from(digest[offset + 3]);
    Ok(format!(
        "{:0width$}",
        binary % 10_u32.pow(TOTP_DIGITS),
        width = TOTP_DIGITS as usize
    ))
}

fn decode_totp_secret(secret: &str) -> Result<Vec<u8>> {
    let normalized = secret
        .chars()
        .filter(|character| {
            !character.is_ascii_whitespace() && *character != '-' && *character != '='
        })
        .map(|character| character.to_ascii_uppercase())
        .collect::<String>();
    if normalized.is_empty() {
        return Err(anyhow!("totp secret is required"));
    }
    let mut buffer = 0_u32;
    let mut bits = 0_u8;
    let mut bytes = Vec::new();
    for character in normalized.chars() {
        let value = match character {
            'A'..='Z' => character as u32 - 'A' as u32,
            '2'..='7' => character as u32 - '2' as u32 + 26,
            _ => return Ok(secret.as_bytes().to_vec()),
        };
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            bytes.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    if bytes.is_empty() {
        Ok(secret.as_bytes().to_vec())
    } else {
        Ok(bytes)
    }
}

fn ip_allowed(client_ip: &str, allowlist: &str) -> bool {
    let Ok(ip) = client_ip.parse::<IpAddr>() else {
        return false;
    };
    allowlist
        .split([',', '\n', ';'])
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .any(|entry| entry == "*" || ip_entry_matches(ip, entry))
}

fn ip_entry_matches(ip: IpAddr, entry: &str) -> bool {
    if let Ok(exact) = entry.parse::<IpAddr>() {
        return exact == ip;
    }
    let Some((base, prefix)) = entry.split_once('/') else {
        return false;
    };
    let Ok(prefix) = prefix.parse::<u8>() else {
        return false;
    };
    match (ip, base.parse::<IpAddr>()) {
        (IpAddr::V4(ip), Ok(IpAddr::V4(base))) if prefix <= 32 => cidr_v4(ip, base, prefix),
        (IpAddr::V6(ip), Ok(IpAddr::V6(base))) if prefix <= 128 => cidr_v6(ip, base, prefix),
        _ => false,
    }
}

fn cidr_v4(ip: Ipv4Addr, base: Ipv4Addr, prefix: u8) -> bool {
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    (u32::from(ip) & mask) == (u32::from(base) & mask)
}

fn cidr_v6(ip: Ipv6Addr, base: Ipv6Addr, prefix: u8) -> bool {
    let mask = if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    };
    (u128::from(ip) & mask) == (u128::from(base) & mask)
}

fn first_ip(value: &str) -> Result<String> {
    let candidate = value
        .split(',')
        .next()
        .unwrap_or(value)
        .trim()
        .trim_matches('"')
        .trim_start_matches("for=")
        .trim_matches(['[', ']']);
    candidate
        .parse::<IpAddr>()
        .map(|ip| ip.to_string())
        .context("invalid client IP")
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
