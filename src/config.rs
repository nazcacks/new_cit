use std::env;

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub enable_worker: bool,
    pub job_poll_seconds: u64,
    pub allowed_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
        let host = env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("APP_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .context("APP_PORT must be a valid u16")?;
        let enable_worker = env::var("ENABLE_WORKER")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(true);
        let job_poll_seconds = env::var("JOB_POLL_SECONDS")
            .unwrap_or_else(|_| "2".to_string())
            .parse()
            .context("JOB_POLL_SECONDS must be a valid integer")?;
        let allowed_origins = env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:8080,http://127.0.0.1:8080".to_string())
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect();

        Ok(Self {
            database_url,
            host,
            port,
            enable_worker,
            job_poll_seconds,
            allowed_origins,
        })
    }

    pub fn test(database_url: String) -> Self {
        Self {
            database_url,
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_worker: false,
            job_poll_seconds: 1,
            allowed_origins: vec!["http://127.0.0.1".to_string()],
        }
    }
}
