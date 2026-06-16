#!/usr/bin/env bash
# Cross-compile and deploy to droplet.
# Usage: bash deploy.sh [host]
set -euo pipefail

DEPLOY_HOST="${1:-root@pray.rs}"
APP_DIR=/opt/thanksgivings
SERVICE=thanksgivings

# ── Ensure zig toolchain is ready ─────────────────────────────────────────────
if ! command -v zig &> /dev/null; then
  echo "→ Installing zig via brew..."
  brew install zig
fi
if ! cargo zigbuild --version &> /dev/null 2>&1; then
  echo "→ Installing cargo-zigbuild..."
  cargo install cargo-zigbuild
fi
if ! rustup target list --installed | grep -q x86_64-unknown-linux-gnu; then
  echo "→ Adding Linux target..."
  rustup target add x86_64-unknown-linux-gnu
fi

# ── Build WASM + CSS ──────────────────────────────────────────────────────────
# cargo-leptos builds WASM and CSS before linking the native server binary.
# The native link fails on macOS (long symbol names) but we use zigbuild for
# that step anyway, so we ignore the exit code and just need the pkg/ output.
echo "→ Building WASM and CSS..."
cargo leptos build --release || true
[ -f target/site/pkg/thanksgivings.wasm ] || { echo "✗ WASM build failed"; exit 1; }

# ── Cross-compile server binary for Linux x86_64 ──────────────────────────────
echo "→ Cross-compiling server binary for Linux..."
cargo zigbuild --release --target x86_64-unknown-linux-gnu -p app --features ssr --bin app

# ── Upload ────────────────────────────────────────────────────────────────────
BINARY="target/x86_64-unknown-linux-gnu/release/app"
echo "→ Uploading binary..."
scp "$BINARY" "$DEPLOY_HOST:$APP_DIR/thanksgivings.new"

echo "→ Uploading site assets..."
scp -r target/site/pkg "$DEPLOY_HOST:$APP_DIR/site/"

# Upload public assets (logo, etc.) from site root
for f in target/site/logo.png target/site/favicon.ico target/site/manifest.json; do
  [ -f "$f" ] && scp "$f" "$DEPLOY_HOST:$APP_DIR/site/$(basename $f)"
done

# ── Restart service ───────────────────────────────────────────────────────────
echo "→ Deploying and restarting..."
ssh "$DEPLOY_HOST" bash -s <<'REMOTE'
set -euo pipefail
# Fix WASM filename — HTML requests thanksgivings_bg.wasm, leptos outputs thanksgivings.wasm
if [ -f /opt/thanksgivings/site/pkg/thanksgivings.wasm ]; then
  cp /opt/thanksgivings/site/pkg/thanksgivings.wasm \
     /opt/thanksgivings/site/pkg/thanksgivings_bg.wasm
fi
mv /opt/thanksgivings/thanksgivings.new /opt/thanksgivings/thanksgivings
chmod +x /opt/thanksgivings/thanksgivings
chown -R thanksgivings:thanksgivings /opt/thanksgivings/site
systemctl restart thanksgivings
sleep 2
systemctl is-active thanksgivings && echo "✓ thanksgivings running"
systemctl is-active caddy         && echo "✓ caddy running"
REMOTE

echo "✓ Deployed. https://pray.rs"
