mod config;
mod db;
mod jobs;
mod protocol;
mod server;
mod shares;
mod vardiff;

use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = config::load()?;
    tracing::info!("GitMine Pool Server starting up");

    let db = db::create_pool(&cfg.database_url).await?;
    tracing::info!("Postgres connected (max_connections=32)");

    // Run schema migrations so tables exist on first start
    db::migrate(&db).await?;
    tracing::info!("Schema ready");

    let job_manager = Arc::new(jobs::JobManager::new(cfg.clone()));
    let jm = job_manager.clone();
    tokio::spawn(async move { jm.run().await });

    let vd = Arc::new(vardiff::VarDiffManager::new(cfg.vardiff.clone()));

    server::run(cfg, db, job_manager, vd).await?;

    Ok(())
}
