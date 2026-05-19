use anyhow::{anyhow, Result};
use cit_system::{db, seed};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let options = parse_args()?;
    let config = cit_system::Config::from_env()?;
    let pool = db::connect(&config.database_url).await?;
    db::migrate(&pool).await?;
    let result = seed::run_demo_seed(&pool, options).await?;
    println!(
        "seed-demo completed tenant={} main_by_id={} filed_by_id={} customers={} business_years={} users={} menus={} efiling_id={} validation_issues={} validation_errors={}",
        result.tenant_code,
        result.main_by_id,
        result.filed_by_id,
        result.customer_count,
        result.business_year_count,
        result.user_count,
        result.menu_node_count,
        result.efiling_id,
        result.validation_issue_count,
        result.validation_error_count
    );
    Ok(())
}

fn parse_args() -> Result<seed::DemoSeedOptions> {
    let mut options = seed::DemoSeedOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--reset" => options.reset = true,
            "--tenant" => {
                options.tenant_code = args
                    .next()
                    .ok_or_else(|| anyhow!("--tenant requires a value"))?;
            }
            "--admin-password" => {
                options.admin_password = args
                    .next()
                    .ok_or_else(|| anyhow!("--admin-password requires a value"))?;
            }
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run --bin seed-demo -- [--reset] [--tenant demo] [--admin-password <password>]"
                );
                std::process::exit(0);
            }
            other => return Err(anyhow!("unknown argument {other}")),
        }
    }
    Ok(options)
}
