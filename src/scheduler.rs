use anyhow::{Context, Result};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use crate::{domain::TenantRef, tenant};

pub async fn run_due_alerts(pool: &PgPool) -> Result<Value> {
    let tenants = sqlx::query(
        r#"
        SELECT tenant_id, tenant_code, schema_name
        FROM tenants
        WHERE status = 'ACTIVE'
        ORDER BY tenant_code
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to load tenants for scheduler")?;

    let mut results = Vec::new();
    let mut total_created = 0_i64;
    for row in tenants {
        let tenant_ref = TenantRef {
            tenant_id: row.get("tenant_id"),
            tenant_code: row.get("tenant_code"),
            schema_name: row.get("schema_name"),
        };
        let created = tenant::ensure_due_notifications(pool, &tenant_ref).await?;
        total_created += created;
        results.push(json!({
            "tenant_code": tenant_ref.tenant_code,
            "created": created
        }));
    }
    Ok(json!({
        "scheduler": "due-alerts",
        "created": total_created,
        "tenants": results
    }))
}
