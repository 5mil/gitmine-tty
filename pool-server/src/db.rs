use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

pub type DbPool = Pool<Postgres>;

pub async fn create_pool(database_url: &str) -> Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(32)
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Ensure all required tables exist. Safe to run on every startup.
pub async fn migrate(pool: &DbPool) -> Result<()> {
    sqlx::query("
        CREATE TABLE IF NOT EXISTS users (
            id              BIGSERIAL PRIMARY KEY,
            github_id       BIGINT UNIQUE NOT NULL,
            github_login    TEXT UNIQUE NOT NULL,
            avatar_url      TEXT,
            last_seen_at    TIMESTAMPTZ DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS workers (
            id              BIGSERIAL PRIMARY KEY,
            worker_name     TEXT UNIQUE NOT NULL,
            coin            TEXT NOT NULL DEFAULT 'FNNC',
            created_at      TIMESTAMPTZ DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS shares (
            id              BIGSERIAL PRIMARY KEY,
            worker_name     TEXT NOT NULL,
            coin            TEXT NOT NULL,
            accepted        BOOLEAN NOT NULL DEFAULT TRUE,
            difficulty      DOUBLE PRECISION NOT NULL DEFAULT 1.0,
            submitted_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            block_height    BIGINT,
            nonce           TEXT,
            hash            TEXT,
            is_block        BOOLEAN NOT NULL DEFAULT FALSE
        );

        CREATE INDEX IF NOT EXISTS shares_worker_submitted
            ON shares(worker_name, submitted_at DESC);

        CREATE TABLE IF NOT EXISTS balances (
            id              BIGSERIAL PRIMARY KEY,
            worker_name     TEXT NOT NULL,
            coin            TEXT NOT NULL,
            amount          DOUBLE PRECISION NOT NULL DEFAULT 0.0,
            updated_at      TIMESTAMPTZ DEFAULT NOW(),
            UNIQUE(worker_name, coin)
        );

        CREATE TABLE IF NOT EXISTS pool_stats (
            id              BIGSERIAL PRIMARY KEY,
            coin            TEXT NOT NULL,
            block_height    BIGINT,
            template_algo   TEXT,
            total_shares    BIGINT NOT NULL DEFAULT 0,
            hashrate_mhs    DOUBLE PRECISION NOT NULL DEFAULT 0.0,
            active_workers  INT NOT NULL DEFAULT 0,
            snapshot_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
    ")
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct ShareRow {
    pub worker_name:  String,
    pub coin:         String,
    pub accepted:     bool,
    pub difficulty:   f64,
    pub submitted_at: DateTime<Utc>,
    pub block_height: Option<i64>,
    pub nonce:        Option<String>,
    pub hash:         Option<String>,
    pub is_block:     bool,
}

pub async fn insert_share(pool: &DbPool, r: &ShareRow) -> Result<()> {
    sqlx::query("
        INSERT INTO shares
            (worker_name, coin, accepted, difficulty, submitted_at, block_height, nonce, hash, is_block)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
    ")
    .bind(&r.worker_name)
    .bind(&r.coin)
    .bind(r.accepted)
    .bind(r.difficulty)
    .bind(r.submitted_at)
    .bind(r.block_height)
    .bind(&r.nonce)
    .bind(&r.hash)
    .bind(r.is_block)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn ensure_worker(pool: &DbPool, worker_name: &str, coin: &str) -> Result<()> {
    sqlx::query("
        INSERT INTO workers (worker_name, coin)
        VALUES ($1, $2)
        ON CONFLICT (worker_name) DO NOTHING
    ")
    .bind(worker_name)
    .bind(coin)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_pool_stats(
    pool: &DbPool,
    coin: &str,
    height: i64,
    algo: &str,
    total_shares: i64,
    hashrate_mhs: f64,
    active_workers: i32,
) -> Result<()> {
    sqlx::query("
        INSERT INTO pool_stats (coin, block_height, template_algo, total_shares, hashrate_mhs, active_workers, snapshot_at)
        VALUES ($1,$2,$3,$4,$5,$6,NOW())
    ")
    .bind(coin)
    .bind(height)
    .bind(algo)
    .bind(total_shares)
    .bind(hashrate_mhs)
    .bind(active_workers)
    .execute(pool)
    .await?;
    Ok(())
}
