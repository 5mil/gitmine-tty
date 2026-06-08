use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    #[serde(default)]
    pub database_url: String,
    #[serde(default)]
    pub coins: CoinsConfig,
    #[serde(default)]
    pub vardiff: VarDiffConfig,
    #[serde(default)]
    pub pool: PoolConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CoinsConfig {
    pub fnnc: CoinConfig,
    pub tty: CoinConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CoinConfig {
    #[serde(default)]
    pub rpc_url: String,
    #[serde(default = "default_rpc_user")]
    pub rpc_user: String,
    #[serde(default)]
    pub rpc_pass: String,
    #[serde(default)]
    pub pool_address: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VarDiffConfig {
    #[serde(default = "default_target_secs")]
    pub target_secs: f64,
    #[serde(default = "default_retarget_secs")]
    pub retarget_secs: f64,
    #[serde(default = "default_min_diff")]
    pub min_diff: f64,
    #[serde(default = "default_max_diff")]
    pub max_diff: f64,
    #[serde(default = "default_variance_pct")]
    pub variance_pct: f64,
}

impl Default for VarDiffConfig {
    fn default() -> Self {
        Self {
            target_secs: default_target_secs(),
            retarget_secs: default_retarget_secs(),
            min_diff: default_min_diff(),
            max_diff: default_max_diff(),
            variance_pct: default_variance_pct(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PoolConfig {
    #[serde(default = "default_fee_pct")]
    pub fee_pct: f64,
    #[serde(default = "default_extranonce2_size")]
    pub extranonce2_size: u8,
}

fn default_listen_addr()    -> String  { "0.0.0.0:3333".to_string() }
fn default_rpc_user()       -> String  { "gitmine".to_string() }
fn default_target_secs()    -> f64     { 15.0 }
fn default_retarget_secs()  -> f64     { 30.0 }
fn default_min_diff()       -> f64     { 1.0 }
fn default_max_diff()       -> f64     { 1_000_000.0 }
fn default_variance_pct()   -> f64     { 0.25 }
fn default_fee_pct()        -> f64     { 1.0 }
fn default_extranonce2_size() -> u8    { 4 }

pub fn load() -> anyhow::Result<Config> {
    let builder = config::Config::builder()
        .add_source(config::File::with_name("config/server").required(false))
        .add_source(config::Environment::with_prefix("GITMINE_POOL").separator("__"));

    let mut cfg: Config = builder.build()?.try_deserialize()?;

    if cfg.database_url.is_empty() {
        cfg.database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL or GITMINE_POOL__DATABASE_URL must be set");
    }

    // Override RPC creds from env if present
    if let Ok(v) = std::env::var("FNNC_RPC_PASS")  { cfg.coins.fnnc.rpc_pass = v; }
    if let Ok(v) = std::env::var("TTY_RPC_PASS")   { cfg.coins.tty.rpc_pass = v; }
    if let Ok(v) = std::env::var("FNNC_POOL_ADDRESS") { cfg.coins.fnnc.pool_address = v; }
    if let Ok(v) = std::env::var("TTY_POOL_ADDRESS")  { cfg.coins.tty.pool_address = v; }

    Ok(cfg)
}
