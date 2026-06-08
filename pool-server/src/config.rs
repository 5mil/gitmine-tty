use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    #[serde(default)]
    pub database_url: String,
}

fn default_listen_addr() -> String {
    "0.0.0.0:3333".to_string()
}

pub fn load() -> anyhow::Result<Config> {
    let builder = config::Config::builder()
        .add_source(config::File::with_name("config/server").required(false))
        .add_source(config::Environment::with_prefix("GITMINE_POOL").separator("__"));

    let mut cfg: Config = builder.build()?.try_deserialize()?;

    if cfg.database_url.is_empty() {
        cfg.database_url = std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("GITMINE_POOL__DATABASE_URL"))
            .expect("DATABASE_URL or GITMINE_POOL__DATABASE_URL must be set");
    }

    Ok(cfg)
}
