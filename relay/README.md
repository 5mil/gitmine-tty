# Relay — GitMine Self-Hosted Real-Time Bridge

Relay replaces Cloudflare Workers entirely. It is a lightweight Python WebSocket
broadcaster that runs on your own hardware — your mining rig, home server, Raspberry Pi,
or any always-on Linux box. Zero cloud accounts. Zero external dependencies.

## Architecture

```
GitHub Action (pool operator)
    ↓ POST /webhook
Relay HTTP Listener (:8766)
    ↓ parse + classify event
Relay WebSocket Server (:8765)
    ↓ broadcast to all clients
Dashboard PWA / TUI Command
```

## Setup

### 1. Install dependencies
```bash
pip install websockets aiohttp pyyaml
```

### 2. Configure pool.yaml
Add your GitHub webhook secret and Relay port:
```yaml
operator:
  relay_host: "0.0.0.0"
  relay_port: 8765
  webhook_secret: "your-github-webhook-secret"
```

### 3. Start Relay
```bash
python relay/bridge.py
```

### 4. Configure GitHub Webhook
- Go to: `https://github.com/5mil/gitmine-tty/settings/hooks`
- Payload URL: `http://<your-ip>:8766/webhook`
- Content type: `application/json`
- Secret: (same as `webhook_secret` in pool.yaml)
- Events: `push`, `workflow_run`, `create`

### 5. Install as systemd service (optional)
```bash
sudo cp relay/gitmine-relay.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now gitmine-relay
sudo journalctl -u gitmine-relay -f
```

## Client Connection

Dashboard PWA and TUI connect to:
```
ws://<your-ip>:8765
```

Set `RELAY_URL` in the dashboard or TUI config:
```yaml
# fleet/defaults.yaml
relay:
  enabled: true
  host: "192.168.1.100"  # Your Relay host
  port: 8765
```

## Events Broadcast

| Event subtype | Trigger | Payload fields |
|---|---|---|
| `state_update` | Action commits stats/template | `branch`, `commits` |
| `share_submitted` | Miner pushes to `shares-pending` | `branch`, `pusher`, `commits` |
| `heartbeat` | Rig pushes heartbeat commit | `rig_id`, `branch` |
| `new_template` | Action creates template tag | `ref`, `ref_type` |
| `workflow_run` | Action starts/completes | `workflow`, `status`, `conclusion` |

## Security Notes

- Relay never holds secrets or private keys
- Webhook signature (HMAC-SHA256) verified before broadcast
- WebSocket clients are read-only receivers (no write-back to pool)
- Run behind your router's firewall; only expose port if remote clients needed
- For remote access: SSH tunnel (`ssh -L 8765:localhost:8765 yourserver`)
