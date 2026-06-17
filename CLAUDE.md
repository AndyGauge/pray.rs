# pray.rs (Thanksgivings)

A Rust full-stack prayer book web app: write prayers/praises, share them privately,
to a group, or publicly. Intentional paged ("book") navigation — no feed, no algorithm.
Bring-your-own-AI via an authenticated MCP server.

## Stack

- **Leptos** (SSR + WASM hydrate) on **Axum**
- **SQLite via SQLx** — uses `sqlx::query_as` / runtime queries (NOT the `query!` macros),
  so no `DATABASE_URL` is needed at compile time
- **OAuth2** (Google + Facebook) with `tower-sessions` (SQLite store)
- **MCP server** at `/mcp` with an OAuth 2.1 authorization server (DCR + PKCE) so
  Claude Code / claude.ai can connect; also supports static API keys (`prs_…`)

## Workspace layout

```
crates/core          — shared types (Post, User, Group, Visibility, …)
crates/db            — SQLx SQLite layer, migrations, repository pattern
crates/app           — Leptos full-stack app (SSR bin + WASM hydrate lib)
crates/project-mcp   — separate stdio MCP server for feature requests + changelog
```

## Build & run

- **Dev:** `cargo leptos watch` (serves on `127.0.0.1:3000`, hot-reloads). Requires
  `cargo-leptos` and the `wasm32-unknown-unknown` target.
- **Type-check both targets** before assuming a change is good:
  - SSR: `cargo check -p app --features ssr`
  - Hydrate/WASM: `cargo check -p app --features hydrate --target wasm32-unknown-unknown`
- **Deploy:** `bash deploy/deploy.sh` — cross-compiles the Linux binary with
  `cargo zigbuild`, builds WASM/CSS via `cargo leptos build --release`, uploads to the
  droplet, and restarts the systemd service. See the `infra` skill for details.
- `DEV_AUTH_BYPASS=1` (in `.env`) enables a dev login that skips real OAuth.

## Gotchas

### `cargo check` passing is NOT enough — release builds compute view-type layout

A change can pass `cargo check` (SSR and WASM) yet **fail the release build** with:

```
error: queries overflow the depth limit!
```

`view!` macros build one deeply-nested, statically-typed view; release codegen computes
that type's full layout, and a large page can blow the default recursion limit. `cargo
check` doesn't do this work, so it won't catch it. **Verify deep UI changes with a
release compile** (`cargo zigbuild --release --target x86_64-unknown-linux-gnu -p app
--features ssr --bin app`) before deploying.

### Fix it by promoting components, not by raising `recursion_limit`

Do **not** reach for `#![recursion_limit = "…"]` — that just hides a growing type behind
a bigger global budget and slows compiles for everyone.

Instead, **extract the deep subtree into its own `#[component]` and type-erase the
boundary with `.into_any()`**. Returning `AnyView` truncates the static type tree at that
component, so the parent sees a single shallow leaf instead of inheriting the whole
nested type. Each piece then has its own modest depth.

Example: the compose page's visibility control (Private/Public + a button per group +
a mobile dropdown) tripped the limit. The fix was `VisibilityField` in
`crates/app/src/pages/compose.rs` — a component whose body ends in `.into_any()`. Prefer
this pattern whenever a page's `view!` grows several levels of nesting.
