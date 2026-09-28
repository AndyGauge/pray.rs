# pray.rs — common commands. Run `make` (or `make help`) to list them.
# Run from the repo root: .cargo/config.toml (SQLX_OFFLINE=true) only applies here.

HOST ?= root@pray.rs
LINUX_TARGET := x86_64-unknown-linux-gnu

.DEFAULT_GOAL := help
.PHONY: help setup dev check check-ssr check-wasm test release sqlx-prepare sqlx-check verify deploy project-mcp

help: ## List available commands
	@grep -E '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "} {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

# ── Setup ──────────────────────────────────────────────────────────────────────

setup: ## Install the toolchain pieces (wasm target, cargo-leptos, cargo-zigbuild, zig)
	rustup target add wasm32-unknown-unknown
	cargo install --locked cargo-leptos
	cargo install --locked cargo-zigbuild
	@command -v zig >/dev/null || brew install zig
	@test -f .env || { cp .env.example .env; echo "created .env from .env.example; fill it in"; }

# ── Develop ────────────────────────────────────────────────────────────────────

dev: ## Run the app with hot reload on http://127.0.0.1:3000
	cargo leptos watch

check: check-ssr check-wasm ## Type-check both build targets (server + WASM)

check-ssr:
	cargo check -p app --features ssr

check-wasm:
	cargo check -p app --features hydrate --target wasm32-unknown-unknown

test: ## Run the db and app tests
	cargo test -p thanksgivings-db -p app --features app/ssr

release: ## Release-compile the server (catches view-depth overflow `check` misses)
	cargo zigbuild --release --target $(LINUX_TARGET) -p app --features ssr --bin app

# ── Database queries ───────────────────────────────────────────────────────────

sqlx-prepare: ## Regenerate .sqlx/ after changing a query or adding a migration
	scripts/sqlx-prepare.sh

sqlx-check: ## Fail if .sqlx/ is stale
	scripts/sqlx-prepare.sh --check

# ── Ship ───────────────────────────────────────────────────────────────────────

verify: sqlx-check check test release ## Everything to run before committing or deploying

deploy: ## Build and deploy to the droplet (override with HOST=user@host)
	bash deploy/deploy.sh $(HOST)

# ── Tools ──────────────────────────────────────────────────────────────────────

project-mcp: ## Build the stdio feature-request/changelog MCP server (.mcp.json uses it)
	cargo build -p project-mcp
