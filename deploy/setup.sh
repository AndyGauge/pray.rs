#!/usr/bin/env bash
# Run once on a fresh Ubuntu 24.04 droplet as root.
# Usage: bash setup.sh
set -euo pipefail

APP_USER=thanksgivings
APP_DIR=/opt/thanksgivings
APP_PORT=3000
DOMAIN="${1:-pray.rs}"

# ── System deps ───────────────────────────────────────────────────────────────
apt-get update -q
apt-get install -y --no-install-recommends debian-keyring debian-archive-keyring apt-transport-https curl

# Caddy
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' \
    | gpg --batch --no-tty --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' \
    | tee /etc/apt/sources.list.d/caddy-stable.list
apt-get update -q
apt-get install -y caddy

# ── App user + directories ────────────────────────────────────────────────────
useradd --system --no-create-home --shell /usr/sbin/nologin $APP_USER || true
mkdir -p $APP_DIR/data
chown -R $APP_USER:$APP_USER $APP_DIR

# ── Caddy configuration ───────────────────────────────────────────────────────
cat > /etc/caddy/Caddyfile << EOF
$DOMAIN {
  reverse_proxy localhost:$APP_PORT
}
EOF

systemctl enable caddy
systemctl restart caddy

# ── Systemd service for app ───────────────────────────────────────────────────
cat > /etc/systemd/system/thanksgivings.service << EOF
[Unit]
Description=Thanksgivings Prayer App
After=network.target

[Service]
Type=simple
User=$APP_USER
WorkingDirectory=$APP_DIR
ExecStart=$APP_DIR/app
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable thanksgivings

echo "✓ System ready for deployment."
echo "  Domain: $DOMAIN"
echo "  App port: $APP_PORT"
echo "  App dir: $APP_DIR"
echo ""
echo "Next: Upload your binary to $APP_DIR/app, then:"
echo "  ssh root@pray.rs systemctl start thanksgivings"
