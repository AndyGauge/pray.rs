# Infra — pray.rs deployment helper

You are helping manage the Thanksgivings app (pray.rs) running on a DigitalOcean droplet.

## Context

- **Droplet**: `root@143.198.58.74` (hostname: pray-rs, Ubuntu 24.04)
- **Domain**: pray.rs (DNS already pointed at droplet)
- **App binary**: `/opt/thanksgivings/thanksgivings`
- **Site assets**: `/opt/thanksgivings/site/pkg/` (WASM, JS, CSS)
- **Database**: `/opt/thanksgivings/data/thanksgivings.db` (SQLite)
- **Service**: `thanksgivings.service` managed by systemd
- **Reverse proxy**: Caddy on port 443 → localhost:3000
- **Service file**: `deploy/thanksgivings.service` (source of truth — upload if changed)
- **Build tool**: `cargo zigbuild --release --target x86_64-unknown-linux-gnu -p app --features ssr --bin app`
- **Binary output**: `target/x86_64-unknown-linux-gnu/release/app`

## Common operations

### Deploy a new release
```sh
cargo zigbuild --release --target x86_64-unknown-linux-gnu -p app --features ssr --bin app
scp target/x86_64-unknown-linux-gnu/release/app root@143.198.58.74:/opt/thanksgivings/thanksgivings.new
scp -r target/site/pkg root@143.198.58.74:/opt/thanksgivings/site/
ssh root@143.198.58.74 'mv /opt/thanksgivings/thanksgivings.new /opt/thanksgivings/thanksgivings && chmod +x /opt/thanksgivings/thanksgivings && chown -R thanksgivings:thanksgivings /opt/thanksgivings/site && systemctl restart thanksgivings && systemctl is-active thanksgivings'
```

Or just run: `bash deploy/deploy.sh`

### Check service status
```sh
ssh root@143.198.58.74 'systemctl status thanksgivings --no-pager'
```

### Tail logs
```sh
ssh root@143.198.58.74 'journalctl -u thanksgivings -f'
```

### Check recent errors
```sh
ssh root@143.198.58.74 'journalctl -u thanksgivings -n 50 --no-pager'
```

### Restart without redeploy
```sh
ssh root@143.198.58.74 'systemctl restart thanksgivings'
```

### Update service env vars (OAuth keys, etc.)
Edit `deploy/thanksgivings.service`, then:
```sh
scp deploy/thanksgivings.service root@143.198.58.74:/etc/systemd/system/thanksgivings.service
ssh root@143.198.58.74 'systemctl daemon-reload && systemctl restart thanksgivings'
```

### Check Caddy
```sh
ssh root@143.198.58.74 'systemctl status caddy --no-pager && caddy version'
```

### Fresh droplet setup (run once)
```sh
scp deploy/setup.sh root@143.198.58.74:/tmp/setup.sh
ssh root@143.198.58.74 'bash /tmp/setup.sh'
```

### Smoke test
```sh
ssh root@143.198.58.74 'curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:3000/'
curl -s -o /dev/null -w "%{http_code}" https://pray.rs/
```

## Things to watch out for

- The binary must be named `thanksgivings` (not `app`) — the service file expects this.
- Site assets go to `/opt/thanksgivings/site/pkg/`, owned by `thanksgivings:thanksgivings`.
- `ProtectSystem=strict` is on — only `/opt/thanksgivings/data` is writable by the service.
- OAuth env vars in the service file are empty until filled in — `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `FACEBOOK_CLIENT_ID`, `FACEBOOK_CLIENT_SECRET`.
- Cargo zigbuild requires `zig` (`brew install zig`) and `cargo-zigbuild` (`cargo install cargo-zigbuild`).
- The `reqwest` dependency must use `rustls-tls` (not default OpenSSL) for cross-compilation to work.

## Now

Address whatever the user asked about infra. Use the Bash tool to SSH into the droplet or run local build commands as needed. Be direct — run the commands, show the output.
