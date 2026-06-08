# GitMine

> A Git-native dual-coin mining pool. GitHub Actions is the pool operator. Git commits are shares. Supabase is the live database + realtime layer. GitHub Pages is the dashboard.

**Dashboard:** https://5mil.github.io/gitmine-tty

## Coins

| Coin | Ticker | Algorithm | RPC Port |
|------|--------|-----------|----------|
| Fennec | FNNC | YescryptR16 | 8339 |
| Trinity | TTY | SHA256d | 12345 |

## Architecture

```
Miners (cgminer / sgminer / XMRig)
  │  Stratum TCP  pool.gitmine.io:3333
  ▼
Stratum Server  ──►  Supabase (shares, workers, balances, pool_stats)
                          │
                     Realtime WS
                          │
                     GitHub Pages Dashboard  (docs/index.html)
                          │
                     GitHub Actions  (validate · stats · payouts)
```

## Stack

| Component | Technology | Status |
|-----------|-----------|--------|
| Pool Operator | GitHub Actions | ✅ Live |
| Dashboard | GitHub Pages (PWA) | ✅ Live |
| Database + Realtime | Supabase | ✅ Connected |
| Auth | GitHub OAuth (Device Flow) | ✅ Live |
| Realtime Bridge | Fennec (self-hosted WS) | Optional |
| Config / State | Git branches | ✅ Live |
| Miner Client | cgminer · sgminer · XMRig · bfgminer | Point & mine |

## Quick Start — Mine in 2 Minutes

### 1. Sign in

Open the dashboard and click **Sign in with GitHub**. No account creation or config needed — your GitHub login becomes your pool identity.

### 2. Point your miner

The dashboard shows you the exact command for your software. Example with cgminer:

```bash
# Fennec (YescryptR16 — CPU/GPU)
cgminer --url stratum+tcp://pool.gitmine.io:3333 \
  --user yourgithub.gitmine --pass x \
  --algorithm yescryptr16

# Trinity (SHA256d — ASIC/GPU)
cgminer --url stratum+tcp://pool.gitmine.io:3333 \
  --user yourgithub.gitmine --pass x \
  --algorithm sha256d
```

Username format: `githublogin.workername` — e.g. `5mil.rig1`, `5mil.asic`

### 3. Watch your stats

Shares appear live on the dashboard. The **• DB live** badge confirms your realtime connection to Supabase.

## Database Schema (Supabase)

Five tables power the pool. All writes from the Actions workflow; reads via anon key with RLS.

| Table | Purpose |
|-------|---------|
| `users` | GitHub identity (id, login, avatar) |
| `workers` | Registered worker names + coin |
| `shares` | Every submitted share (accepted/rejected, difficulty, algo) |
| `balances` | Miner payout balances per coin |
| `pool_stats` | Snapshot metrics per coin (hashrate, shares, block height) |

> **Setup:** Run the migration in `scripts/supabase_schema.sql` against your project, or let `scripts/update_stats.py` auto-create tables on first run.

## GitHub Secrets Required

In repo **Settings → Secrets and variables → Actions**:

| Secret | Description |
|--------|-------------|
| `FNNC_POOL_ADDRESS` | Fennec payout address (starts with F) |
| `FNNC_RPC_PASS` | Fennec daemon RPC password |
| `TTY_POOL_ADDRESS` | Trinity payout address |
| `TTY_RPC_PASS` | Trinity daemon RPC password |
| `SUPABASE_URL` | `https://wfabhxfpzqrdhlkidkcl.supabase.co` |
| `SUPABASE_SERVICE_KEY` | Supabase service role key (for Actions writes) |
| `FENNEC_SECRET` | Relay bridge shared secret (optional) |
| `FENNEC_URL` | Relay bridge URL (optional) |

## Pool Operator Workflow

GitHub Actions (`.github/workflows/pool.yml`) runs every 2 minutes and on share pushes:

1. **Fetch templates** — queries Fennec / Trinity RPC for block templates
2. **Select template** — picks active template per algo
3. **Validate shares** — verifies PoW + Ed25519 signatures from `shares-pending`
4. **Update stats** — writes pool metrics to Supabase `pool_stats`
5. **Process payouts** — computes balances, writes to `payouts/pending/`
6. **Broadcast** — signs and sends via RPC, tags txid
7. **Deploy** — pushes `docs/` to GitHub Pages

## Share Protocol (Git-native path)

For Git-native miners that push commits directly (advanced):

```
SHARE{"coin":"FNNC","algo":"yescryptr16","nonce":"a1b2c3d4",
      "hash":"f00d...","height":123456,"miner":"FAddr...","pubkey":"ed25519hex..."}
|SIG{ed25519signaturehex}
```

Push to `shares-pending` branch → Actions picks up within 2 minutes.

## Payouts

1. `process_payouts.py` computes balances → commits unsigned tx to `payouts/pending/`
2. Pool operator signs via WebAuthn in the dashboard → moves to `payouts/signed/`
3. `broadcast_payouts.py` sends via RPC → tags txid under `payouts/broadcast/`

## Difficulty Presets

| Preset | Best For |
|--------|----------|
| `d=1` (auto / vardiff) | CPUs, entry GPUs |
| `d=512` | Mid-range GPUs |
| `d=8192` | High-end GPUs |
| `d=65536` | ASICs (TTY only) |

Pass as the Stratum password field: `--pass d=512`

## Fennec Relay (optional real-time bridge)

For sub-second dashboard updates without polling:

```bash
pip install websockets aiohttp pyyaml
python fennec/bridge.py --config config/pool.yaml
# Configure GitHub webhook → http://<your-ip>:8766/webhook
# Or use the systemd unit: fennec/gitmine-fennec.service
```

## Fleet Config (GitOps)

Each rig's config lives in `fleet/<rig-id>/config.yaml`. Hot-reload via `git pull` — no restart required.

```bash
git clone git@github.com:5mil/gitmine-tty.git
cp fleet/rig-example/config.yaml fleet/rig-001/config.yaml
vim fleet/rig-001/config.yaml
git add fleet/rig-001/ && git commit -m "fleet: register rig-001" && git push
```

## Algorithms

| Coin | Algorithm | Difficulty | Target Hardware |
|------|-----------|------------|----------------|
| FNNC | YescryptR16 | 8,000 | CPU / GPU |
| TTY | SHA256d | 500,000 | ASIC / GPU |

## License

MIT
