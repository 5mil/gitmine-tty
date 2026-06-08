use crate::config::VarDiffConfig;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct VarDiffManager {
    cfg:   VarDiffConfig,
    state: Arc<DashMap<String, VarDiffState>>,
}

struct VarDiffState {
    current_diff:    f64,
    last_retarget:   Instant,
    share_times:     Vec<Instant>,
}

impl VarDiffManager {
    pub fn new(cfg: VarDiffConfig) -> Self {
        Self { cfg, state: Arc::new(DashMap::new()) }
    }

    /// Record a share submission; return a new difficulty if retarget is due.
    pub fn on_share(&self, worker: &str) -> Option<f64> {
        let now = Instant::now();
        let mut entry = self.state.entry(worker.to_string()).or_insert_with(|| VarDiffState {
            current_diff:  self.cfg.min_diff,
            last_retarget: now,
            share_times:   Vec::new(),
        });

        entry.share_times.push(now);

        // Purge share times older than 10 * target_secs
        let window = std::time::Duration::from_secs_f64(self.cfg.target_secs * 10.0);
        entry.share_times.retain(|t| now.duration_since(*t) < window);

        let since_retarget = now.duration_since(entry.last_retarget).as_secs_f64();
        if since_retarget < self.cfg.retarget_secs {
            return None; // not time to retarget yet
        }

        // Compute average share interval over the window
        let n = entry.share_times.len();
        if n < 2 { return None; }
        let elapsed = now.duration_since(entry.share_times[0]).as_secs_f64();
        let avg_interval = elapsed / (n as f64 - 1.0);

        let target = self.cfg.target_secs;
        let variance = target * self.cfg.variance_pct;

        if (avg_interval - target).abs() < variance {
            return None; // within acceptable variance, no change
        }

        // New difficulty proportional to ratio of actual vs target
        let new_diff = (entry.current_diff * target / avg_interval)
            .clamp(self.cfg.min_diff, self.cfg.max_diff);

        // Round to nearest power-of-2 for cleaner values
        let new_diff = round_diff(new_diff);

        if (new_diff - entry.current_diff).abs() / entry.current_diff < 0.05 {
            return None; // <5% change, not worth notifying
        }

        entry.current_diff  = new_diff;
        entry.last_retarget = now;
        entry.share_times.clear();

        Some(new_diff)
    }

    pub fn current_diff(&self, worker: &str) -> f64 {
        self.state.get(worker).map(|s| s.current_diff).unwrap_or(self.cfg.min_diff)
    }
}

/// Round difficulty to the nearest power of 2.
fn round_diff(d: f64) -> f64 {
    if d <= 1.0 { return 1.0; }
    let exp = d.log2().round();
    2f64.powf(exp)
}
