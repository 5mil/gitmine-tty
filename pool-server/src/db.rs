use sqlx::postgres::PgPoolOptions;

#[derive(Clone)]
pub struct DbPool(pub sqlx::Pool<sqlx::Postgres>);

pub async fn create_pool(database_url: &str) -> anyhow::Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(32)
        .connect(database_url)
        .await?;

    Ok(DbPool(pool))
}
