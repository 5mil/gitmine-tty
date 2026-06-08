use crate::config::Config;
use crate::db::DbPool;
use anyhow::Result;
use tokio::net::TcpListener;

pub async fn run(cfg: Config, _db: DbPool) -> Result<()> {
    let listener = TcpListener::bind(&cfg.listen_addr).await?;
    tracing::info!("gitmine-pool listening on {}", cfg.listen_addr);

    loop {
        let (_socket, addr) = listener.accept().await?;
        tracing::debug!("accepted connection from {}", addr);
        // TODO: spawn per-client handler
    }
}
