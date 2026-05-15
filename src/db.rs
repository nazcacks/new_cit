use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, Executor, PgPool};
use std::time::Duration;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn connect(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await
        .context("failed to connect to PostgreSQL")
}

pub async fn migrate(pool: &PgPool) -> Result<()> {
    MIGRATOR.run(pool).await.context("failed to run migrations")
}

pub fn quote_ident(identifier: &str) -> Result<String> {
    if identifier.is_empty()
        || identifier.len() > 63
        || !identifier
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        || identifier
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
    {
        anyhow::bail!("invalid SQL identifier: {identifier}");
    }
    Ok(format!("\"{identifier}\""))
}

pub async fn execute_batch(pool: &PgPool, sql: &str) -> Result<()> {
    pool.execute(sql)
        .await
        .context("failed to execute SQL batch")?;
    Ok(())
}
