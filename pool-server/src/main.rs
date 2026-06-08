mod config;
mod db;
mod server;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = config::load()?;
    let db = db::create_pool(&cfg.database_url).await?;

    tracing::info!("connected to database, listening on {}", cfg.listen_addr);

    server::run(cfg, db).await?;

    Ok(())
}
