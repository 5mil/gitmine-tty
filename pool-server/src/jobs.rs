use crate::config::{CoinConfig, Config};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id:               String,
    pub coin:             String,
    pub algo:             String,
    pub height:           i64,
    pub version:          u32,
    pub prev_hash:        String,
    pub merkle_branches:  Vec<String>,
    pub ntime:            String,
    pub nbits:            String,
    pub target:           [u8; 32],
    pub coinbase1:        String,
    pub coinbase2:        String,
    pub clean_jobs:       bool,
}

impl Job {
    /// Build a mining.notify params array (Stratum V1 wire format).
    pub fn to_notify_params(&self) -> Value {
        serde_json::json!([
            self.id,
            self.prev_hash,
            self.coinbase1,
            self.coinbase2,
            self.merkle_branches,
            format!("{:08x}", self.version),
            self.nbits,
            self.ntime,
            self.clean_jobs,
        ])
    }
}

#[derive(Clone)]
pub struct JobManager {
    state: Arc<RwLock<JobState>>,
    cfg:   Config,
}

#[derive(Default)]
struct JobState {
    fnnc: Option<Job>,
    tty:  Option<Job>,
    // Subscribers: channel senders for new-job broadcast
    subs: Vec<tokio::sync::mpsc::UnboundedSender<Job>>,
}

impl JobManager {
    pub fn new(cfg: Config) -> Self {
        Self {
            state: Arc::new(RwLock::new(JobState::default())),
            cfg,
        }
    }

    /// Subscribe to new job notifications.
    pub async fn subscribe(&self) -> tokio::sync::mpsc::UnboundedReceiver<Job> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut s = self.state.write().await;
        s.subs.push(tx);
        rx
    }

    /// Get the current job for a coin.
    pub async fn current(&self, coin: &str) -> Option<Job> {
        let s = self.state.read().await;
        match coin {
            "FNNC" => s.fnnc.clone(),
            "TTY"  => s.tty.clone(),
            _      => None,
        }
    }

    /// Background task: poll both daemons for new block templates.
    pub async fn run(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = self.refresh_all().await {
                tracing::warn!("job refresh error: {:?}", e);
            }
        }
    }

    async fn refresh_all(&self) -> Result<()> {
        self.refresh_coin("TTY", &self.cfg.coins.tty.clone(), "sha256d").await?;
        self.refresh_coin("FNNC", &self.cfg.coins.fnnc.clone(), "yescryptr16").await?;
        Ok(())
    }

    async fn refresh_coin(&self, coin: &str, cfg: &CoinConfig, algo: &str) -> Result<()> {
        if cfg.rpc_url.is_empty() {
            // No daemon configured — build a stub job for testing
            let job = stub_job(coin, algo);
            self.update(coin, job).await;
            return Ok(());
        }

        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "jsonrpc": "1.0",
            "id": "gitmine",
            "method": "getblocktemplate",
            "params": [{"capabilities": ["coinbasetxn", "workid", "coinbase/append"]}]
        });

        let resp = client
            .post(&cfg.rpc_url)
            .basic_auth(&cfg.rpc_user, Some(&cfg.rpc_pass))
            .json(&body)
            .send()
            .await?;

        let json: Value = resp.json().await?;
        let tmpl = json.get("result").ok_or_else(|| anyhow::anyhow!("no result"))?;
        let job = template_to_job(coin, algo, tmpl)?;
        self.update(coin, job).await;
        Ok(())
    }

    async fn update(&self, coin: &str, job: Job) {
        let mut s = self.state.write().await;
        s.subs.retain(|tx| !tx.is_closed());
        let subs: Vec<_> = s.subs.clone();
        match coin {
            "TTY"  => s.tty  = Some(job.clone()),
            "FNNC" => s.fnnc = Some(job.clone()),
            _ => {}
        }
        drop(s);
        for tx in subs {
            let _ = tx.send(job.clone());
        }
    }
}

fn template_to_job(coin: &str, algo: &str, t: &Value) -> Result<Job> {
    let height     = t["height"].as_i64().unwrap_or(0);
    let version    = t["version"].as_u64().unwrap_or(0x20000000) as u32;
    let prev_hash  = t["previousblockhash"].as_str().unwrap_or("").to_string();
    let ntime      = format!("{:08x}", t["curtime"].as_u64().unwrap_or(0));
    let nbits      = t["bits"].as_str().unwrap_or("1d00ffff").to_string();
    let coinbaseaux= t["coinbaseaux"]["flags"].as_str().unwrap_or("").to_string();

    let merkle_branches: Vec<String> = t["transactions"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|tx| tx["hash"].as_str().map(str::to_string))
        .collect();

    // Minimal coinbase
    let coinbase1 = format!("01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff08{}", coinbaseaux);
    let coinbase2 = "ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000".to_string();

    let target = nbits_to_target(&nbits);
    let id = format!("{}-{}", coin.to_lowercase(), height);

    Ok(Job {
        id, coin: coin.to_string(), algo: algo.to_string(),
        height, version, prev_hash, merkle_branches,
        ntime, nbits, target, coinbase1, coinbase2, clean_jobs: true,
    })
}

/// Fallback stub job when no daemon is connected — lets us test the pipeline.
fn stub_job(coin: &str, algo: &str) -> Job {
    let height = 1;
    let id = format!("{}-stub-{}", coin.to_lowercase(), height);
    Job {
        id, coin: coin.to_string(), algo: algo.to_string(),
        height, version: 0x20000000,
        prev_hash: "0".repeat(64),
        merkle_branches: vec![],
        ntime: format!("{:08x}", chrono::Utc::now().timestamp()),
        nbits: "1d00ffff".to_string(),
        target: nbits_to_target("1d00ffff"),
        coinbase1: "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff".to_string(),
        coinbase2: "ffffffff0100f2052a0100000000000000".to_string(),
        clean_jobs: false,
    }
}

/// Convert compact nbits to a 32-byte target (big-endian).
pub fn nbits_to_target(nbits: &str) -> [u8; 32] {
    let n = u32::from_str_radix(nbits, 16).unwrap_or(0x1d00ffff);
    let exp   = ((n >> 24) as usize).saturating_sub(3);
    let coeff = (n & 0x00ff_ffff) as u64;
    let mut target = [0u8; 32];
    if exp + 3 <= 32 {
        let pos = 32 - exp - 3;
        let bytes = coeff.to_be_bytes();
        let src_start = 5usize; // 8 - 3
        let len = std::cmp::min(3, 32 - pos);
        target[pos..pos+len].copy_from_slice(&bytes[src_start..src_start+len]);
    }
    target
}

/// Double-SHA256.
pub fn sha256d(data: &[u8]) -> [u8; 32] {
    let first  = Sha256::digest(data);
    let second = Sha256::digest(&first);
    second.into()
}
