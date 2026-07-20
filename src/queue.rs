use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::time;
use uuid::Uuid;

use crate::{domain::Job, efiling, erp, state::AppState, tenant};

pub async fn enqueue(
    pool: &PgPool,
    job_type: &str,
    payload: Value,
    max_attempts: i32,
) -> Result<Job> {
    sqlx::query_as::<_, Job>(
        r#"
        INSERT INTO jobs (job_type, payload, max_attempts)
        VALUES ($1, $2, GREATEST($3, 1))
        RETURNING job_id, job_type, payload, status, attempts, max_attempts, next_run_at,
                  locked_at, last_error, result, created_at, updated_at,
                  completed_at, dead_lettered_at
        "#,
    )
    .bind(job_type)
    .bind(payload)
    .bind(max_attempts)
    .fetch_one(pool)
    .await
    .context("failed to enqueue job")
}

pub async fn get_job(pool: &PgPool, job_id: Uuid) -> Result<Job> {
    sqlx::query_as::<_, Job>(
        r#"
        SELECT job_id, job_type, payload, status, attempts, max_attempts, next_run_at,
               locked_at, last_error, result, created_at, updated_at,
               completed_at, dead_lettered_at
        FROM jobs
        WHERE job_id = $1
        "#,
    )
    .bind(job_id)
    .fetch_one(pool)
    .await
    .context("job not found")
}

pub async fn list_jobs(pool: &PgPool, status: Option<&str>) -> Result<Vec<Job>> {
    let query = if status.is_some() {
        r#"
        SELECT job_id, job_type, payload, status, attempts, max_attempts, next_run_at,
               locked_at, last_error, result, created_at, updated_at,
               completed_at, dead_lettered_at
        FROM jobs
        WHERE status = $1
        ORDER BY created_at DESC
        LIMIT 100
        "#
    } else {
        r#"
        SELECT job_id, job_type, payload, status, attempts, max_attempts, next_run_at,
               locked_at, last_error, result, created_at, updated_at,
               completed_at, dead_lettered_at
        FROM jobs
        WHERE $1::text IS NULL
        ORDER BY created_at DESC
        LIMIT 100
        "#
    };

    sqlx::query_as::<_, Job>(query)
        .bind(status)
        .fetch_all(pool)
        .await
        .context("failed to list jobs")
}

pub async fn retry_dead_letter(pool: &PgPool, job_id: Uuid) -> Result<Job> {
    sqlx::query_as::<_, Job>(
        r#"
        UPDATE jobs
        SET status = 'pending',
            attempts = 0,
            next_run_at = NOW(),
            locked_at = NULL,
            last_error = NULL,
            dead_lettered_at = NULL,
            updated_at = NOW()
        WHERE job_id = $1 AND status = 'dead_letter'
        RETURNING job_id, job_type, payload, status, attempts, max_attempts, next_run_at,
                  locked_at, last_error, result, created_at, updated_at,
                  completed_at, dead_lettered_at
        "#,
    )
    .bind(job_id)
    .fetch_one(pool)
    .await
    .context("dead-letter job not found")
}

pub async fn run_once(state: AppState) -> Result<Option<Job>> {
    let Some(job) = claim_next(&state.pool).await? else {
        return Ok(None);
    };

    match process_job(&state, &job).await {
        Ok(result) => complete_job(&state.pool, job.job_id, result).await,
        Err(error) => fail_job(&state.pool, &job, error.to_string()).await,
    }
}

pub async fn run_worker(state: AppState) {
    let mut interval = time::interval(time::Duration::from_secs(state.config.job_poll_seconds));
    loop {
        interval.tick().await;
        match run_once(state.clone()).await {
            Ok(Some(job)) => {
                tracing::info!(job_id = %job.job_id, job_type = %job.job_type, "job processed");
            }
            Ok(None) => {}
            Err(error) => {
                tracing::error!(?error, "job worker iteration failed");
            }
        }
    }
}

async fn claim_next(pool: &PgPool) -> Result<Option<Job>> {
    sqlx::query_as::<_, Job>(
        r#"
        WITH picked AS (
            SELECT job_id
            FROM jobs
            WHERE status = 'pending' AND next_run_at <= NOW()
            ORDER BY created_at
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        )
        UPDATE jobs
        SET status = 'running',
            attempts = attempts + 1,
            locked_at = NOW(),
            updated_at = NOW()
        WHERE job_id IN (SELECT job_id FROM picked)
        RETURNING job_id, job_type, payload, status, attempts, max_attempts, next_run_at,
                  locked_at, last_error, result, created_at, updated_at,
                  completed_at, dead_lettered_at
        "#,
    )
    .fetch_optional(pool)
    .await
    .context("failed to claim next job")
}

async fn process_job(state: &AppState, job: &Job) -> Result<Value> {
    match job.job_type.as_str() {
        "generate_efiling" => {
            let tenant_code = job
                .payload
                .get("tenant_code")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("generate_efiling requires tenant_code"))?;
            let by_id = job
                .payload
                .get("by_id")
                .and_then(Value::as_i64)
                .ok_or_else(|| anyhow!("generate_efiling requires by_id"))?;
            let tenant = tenant::resolve_tenant(&state.pool, tenant_code).await?;
            let result = efiling::generate_efiling(&state.pool, &tenant, by_id).await?;
            Ok(serde_json::to_value(result)?)
        }
        "erp_import" => {
            erp::process_job(&state.pool, &job.payload, job.attempts, job.max_attempts).await
        }
        other => Err(anyhow!("unsupported job type {other}")),
    }
}

async fn complete_job(pool: &PgPool, job_id: Uuid, result: Value) -> Result<Option<Job>> {
    let job = sqlx::query_as::<_, Job>(
        r#"
        UPDATE jobs
        SET status = 'succeeded',
            result = $2,
            completed_at = NOW(),
            locked_at = NULL,
            updated_at = NOW()
        WHERE job_id = $1
        RETURNING job_id, job_type, payload, status, attempts, max_attempts, next_run_at,
                  locked_at, last_error, result, created_at, updated_at,
                  completed_at, dead_lettered_at
        "#,
    )
    .bind(job_id)
    .bind(result)
    .fetch_one(pool)
    .await
    .context("failed to complete job")?;
    Ok(Some(job))
}

async fn fail_job(pool: &PgPool, job: &Job, error: String) -> Result<Option<Job>> {
    let dead_letter = job.attempts >= job.max_attempts;
    let next_run_at = Utc::now() + retry_delay(job.attempts);
    let status = if dead_letter {
        "dead_letter"
    } else {
        "pending"
    };

    let updated = sqlx::query_as::<_, Job>(
        r#"
        UPDATE jobs
        SET status = $2,
            next_run_at = $3,
            locked_at = NULL,
            last_error = $4,
            dead_lettered_at = CASE WHEN $5 THEN NOW() ELSE NULL END,
            updated_at = NOW()
        WHERE job_id = $1
        RETURNING job_id, job_type, payload, status, attempts, max_attempts, next_run_at,
                  locked_at, last_error, result, created_at, updated_at,
                  completed_at, dead_lettered_at
        "#,
    )
    .bind(job.job_id)
    .bind(status)
    .bind(next_run_at)
    .bind(error)
    .bind(dead_letter)
    .fetch_one(pool)
    .await
    .context("failed to fail job")?;

    if dead_letter {
        tracing::warn!(job_id = %job.job_id, "job moved to dead-letter queue");
    }
    Ok(Some(updated))
}

fn retry_delay(attempts: i32) -> Duration {
    let bounded = attempts.clamp(1, 6);
    Duration::seconds(2_i64.pow(bounded as u32))
}

pub fn generate_efiling_payload(tenant_code: &str, by_id: i64) -> Value {
    json!({
        "tenant_code": tenant_code,
        "by_id": by_id
    })
}

#[cfg(test)]
mod tests {
    use super::retry_delay;

    #[test]
    fn retry_delay_uses_exponential_backoff() {
        assert_eq!(retry_delay(1).num_seconds(), 2);
        assert_eq!(retry_delay(3).num_seconds(), 8);
        assert_eq!(retry_delay(99).num_seconds(), 64);
    }
}
