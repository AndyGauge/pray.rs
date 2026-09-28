# pray.rs (Thanksgivings)

A Rust full-stack prayer book web app: write prayers/praises, share them privately,
to a group, or publicly. Intentional paged ("book") navigation — no feed, no algorithm.
Bring-your-own-AI via an authenticated MCP server.

## Stack

- **Leptos** (SSR + WASM hydrate) on **Axum**
- **SQLite via SQLx** — every query uses the compile-time-checked `query!` /
  `query_as!` macros, verified offline against committed `.sqlx/` metadata (see
  "Database queries" below). No database is needed to build.
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

All common commands are `make` targets (`make` lists them). Run from the repo root.

| Command | What it does |
|---|---|
| `make setup` | Install wasm target, `cargo-leptos`, `cargo-zigbuild`, zig; create `.env` |
| `make dev` | `cargo leptos watch` on `127.0.0.1:3000`, hot-reloads |
| `make check` | Type-check **both** targets (SSR + hydrate/WASM) |
| `make test` | db + app tests |
| `make release` | Release-compile the Linux server (see the depth-limit gotcha below) |
| `make sqlx-prepare` / `make sqlx-check` | Regenerate / verify `.sqlx/` query metadata |
| `make verify` | `sqlx-check` + `check` + `test` + `release` — run before committing or deploying |
| `make deploy` | `deploy/deploy.sh` to the droplet (`HOST=user@host` to override). See the `infra` skill |
| `make project-mcp` | Build the stdio `project-mcp` server that `.mcp.json` points at |

- `make check` passing is **not** enough for UI changes; use `make verify` (it includes
  the release compile).
- `DEV_AUTH_BYPASS=1` (in `.env`) enables a dev login that skips real OAuth.
- When adding a new routine command, add a Makefile target (with a `## description`
  so it shows in `make help`) rather than documenting a raw command here.

## Database queries (compile-time checked)

All SQL in `crates/db/src/repository/` goes through `sqlx::query!` / `sqlx::query_as!`,
so a wrong column, table, parameter count or result type is a **compile error**.

- **Builds are offline.** `.cargo/config.toml` sets `SQLX_OFFLINE=true`, so the macros
  check against the committed `.sqlx/` folder (one `query-<hash>.json` per statement),
  never a live database. `.env`'s `DATABASE_URL` is only used at runtime. Run cargo
  from the repo root, or the config (and offline mode) won't apply.
- **After changing any query's SQL or adding a migration, run `make sqlx-prepare`**
  and commit `.sqlx/`. It builds a throwaway schema from `crates/db/migrations/` and
  recompiles the db crate against it to regenerate the metadata. A changed query with
  no cached entry fails to build (`SQLX_OFFLINE=true but there is no cached data`);
  that error means "run `make sqlx-prepare`", not `cargo sqlx prepare` (the installed sqlx-cli
  is 0.9, the library is 0.8.6).
- **Stale metadata** (schema changed, SQL didn't) still compiles. `make sqlx-check`
  catches it, and `deploy/deploy.sh` runs that check before building.
- **Write SQL as literals.** The macros can't take `format!`-built SQL, so each query
  spells out its columns (no shared `SELECT_…` constants). For a variable-length
  `IN (...)`, bind a JSON array and use `IN (SELECT value FROM json_each(?))`
  (see `posts::with_history`).
- **Nullability overrides.** SQLite lets a non-`INTEGER` `PRIMARY KEY` hold NULL, so
  sqlx infers `Option<String>` for `id`-style columns. Force non-null with
  `id AS "id!"` in a raw string (`r#"..."#`).
- Runtime `sqlx::query(...)` is only for tests that deliberately send invalid data
  (`crates/db/tests/post_state_schema.rs`).

## Post lifecycle (state machine)

Entries are a `PostState` (`crates/core/src/lib.rs`), not a fixed kind:

```
Prayer ──► Thanksgiving
   │            │
   └──► Released ◄┘      (Released is terminal and hidden from every listing)
```

An entry may *start* as Prayer or Thanksgiving (`is_initial`). The legal moves are
defined once, in `PostState::predecessors`. Everything else derives from it:

- **DB:** `posts::transition(pool, post_id, author, to, note)` runs in one transaction:
  it reads the current state, checks `can_transition_to` in Rust, updates with
  `WHERE state = <the state it read>` (so a concurrent move matches no row), and
  appends a row to the **`post_transitions` log** (`from_state`, `to_state`, optional
  trimmed `note`, `created_at`). All list queries exclude `released`.
- **Transition log / notes:** a move never rewrites the entry's `content`. The note
  ("how was it answered?", "why release it?") is recorded in the log and shown *after*
  the entry. `Post.history: Vec<PostTransition>` carries the log (oldest first);
  `posts::with_history` loads it for every listed post in one query.
- **UI:** `offer(from, to)` in `crates/app/src/components/post_page.rs` maps each legal
  move to a button: label, optional `note_prompt` (opens a panel with a textarea), and
  optional `confirm` warning. `StateActions` renders one button per `PostState::ALL`
  entry that `offer` returns, so buttons are never hard-coded per state. `History`
  renders the log under the entry: first when it was written (from `created_at`,
  worded by `created_label` of its starting state, e.g. "Prayed · <date>"), then
  each move worded by `history_label(to)`.
- **MCP:** `transition_tool(to)` in `crates/app/src/mcp/tools.rs` maps each target
  state to a `TransitionTool` (`give_thanks`, `release_prayer`), each taking `post_id`
  and an optional `note`. The tool list, input schema and dispatch are all generated
  from it. `list_my_prayers` / `list_public_prayers` / `list_group_prayers` append each
  entry's history as `→ <state> (<when>): <note>` lines.
- **Server fn:** `set_post_state(post_id, state, note)` in `pages/book.rs`.

### Actions (`PostAction`) — what people can *do* to an entry

States say what an entry *is*; `PostAction` (core) lists what someone can do to it:
`MoveTo(PostState)` (the lifecycle moves above, author only) and `PrayingNow` (+1:
anyone who can see a prayer, unlimited presses). `PostAction::allowed(state, viewer)`
is the single rule, where `Viewer` is `Author` or `Other`; `fetch_posts` returns
`ViewedPost { post, viewer }` so the UI knows which applies.

- **DB:** `posts::pray` checks visibility (private → author; group → members;
  public → anyone) and `allowed`, then upserts `post_prayers` (one row per
  entry+person, `count` bumped every press). `Post.prayers: PrayerCount { total,
  people }` is loaded with the history.
- **UI:** `action_ui(state, action)` in `post_page.rs` maps each action to a control
  (`Move` → `OfferButton`, `PrayingNow` → `PrayButton`, optimistic and mash-safe).
  `PostActions` renders one per allowed action, on any tab.
- **MCP:** `action_tool(action)` maps each action to a tool (`give_thanks`,
  `release_prayer`, `pray_for`); tool list and dispatch come from
  `PostAction::all()`, each behind its capability (`WriteOwn` for moves, `Pray` for
  +1; anonymous callers get neither).
- **Server fn:** `pray_for(post_id)` in `pages/book.rs`.

**Adding an action:** add the variant; `PostAction::all()`'s guard and `allowed`
fail to compile until you list it and say who may do it in which states. Then
`action_ui` (UI) and `action_tool` plus the handler's dispatch `match` (MCP) fail
until both expose it (or return `None` deliberately). The tests
`every_allowed_action_has_a_control` (UI) and `every_allowed_action_has_a_tool`
(MCP) catch an action that's allowed but has no control or tool.

### Adding or changing a state — follow the errors

Every layer is guarded so a new state can't be half-handled. Don't add `_ =>`
wildcard arms to `match`es on `PostState`: they turn off these guards.

1. **Add the variant** to `PostState`. Compile errors (non-exhaustive `match`) then
   point at each place to update: `is_initial`, `predecessors`, `as_str`, `Display`,
   and the `_ALL_IS_COMPLETE` guard, which reminds you to add it to `PostState::ALL`
   (and bump the array length).
2. **Write a migration** that rebuilds **both** `posts` (`state`) and `post_transitions`
   (`from_state`, `to_state`) with the new value in their `CHECK (<col> IN (...))`
   lists (SQLite can't alter a CHECK; copy the approach in `006_post_state.sql`).
   `crates/db/build.rs` **fails the build** if the last CHECK list for any of those
   three columns differs from `PostState::ALL`. Keep constraints written exactly as
   `CHECK (<col> IN ('a', 'b', ...))`, since the build script parses that; a new
   state-holding column must be added to its `STATE_COLUMNS`.
   Then run `make sqlx-prepare` and commit `.sqlx/`.
3. **Decide the UI:** `offer` fails to compile until the new state has an arm
   (a button with its note prompt/confirm, or `None` if the UI shouldn't offer it),
   and `history_label` / `created_label` until you word how the state reads in an
   entry's history.
4. **Decide MCP:** `transition_tool` fails to compile likewise.
5. **Run the tests.** Adding a move between *existing* states compiles fine, so tests
   cover that case: `every_legal_move_has_an_offer` (UI) and
   `every_transition_target_has_a_tool` (MCP) fail if the state machine allows a move
   with no button/tool. `crates/db/tests/post_state_schema.rs` checks that SQLite
   really accepts every state and rejects unknown ones, and that `posts::transition`
   allows exactly the moves `can_transition_to` does, logs each one with its note,
   and never changes the entry's text.
   - `make test`

## Gotchas

### Forms inside the book must lock the page

The book (`Folio` from `leptosbook`) turns pages on keys (Space/arrows), swipes and
drags anywhere inside it. leptosbook leaves input in form fields alone, but a form
that would lose its contents on a page turn must also call
`leptosbook::use_folio_lock(open_signal)` while it's open (see `OfferButton` in
`components/post_page.rs`). `use_folio_lock` needs leptosbook ≥ 0.2.

### `cargo check` passing is NOT enough — release builds compute view-type layout

A change can pass `cargo check` (SSR and WASM) yet **fail the release build** with:

```
error: queries overflow the depth limit!
```

`view!` macros build one deeply-nested, statically-typed view; release codegen computes
that type's full layout, and a large page can blow the default recursion limit. `cargo
check` doesn't do this work, so it won't catch it. **Verify deep UI changes with a
release compile** (`make release`, included in `make verify`) before deploying.

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
