# Changelog

All notable changes to pray.rs are recorded here. Dates are in `YYYY-MM-DD`.

## 2026-09-29

### Added
- **Website** (`website/`, Hugo, published to GitHub Pages by
  `.github/workflows/website.yml`): a feature walkthrough with generated SVG
  illustrations of people using pray.rs (joining a group by QR code in person,
  inviting, sharing, praying now, answered prayers, releasing, bringing your own
  AI), and the public prayers.
- The public prayers are fetched from the app **over MCP at build time**
  (`website/scripts/fetch_public_prayers.py`) and never committed, so a prayer
  made private leaves the site at the next (daily) build.
- `list_public_prayers` now also returns MCP `structuredContent`: the page of
  prayers as JSON (content, state, RFC 3339 dates, history, tally, and
  `next_offset`), without author ids.
- `make site-dev`, `make site`, `make site-prayers`, `make site-illustrations`;
  `make setup` installs Hugo.

### Changed
- **One account per provider.** Email is now unique per provider
  (`UNIQUE (provider, email)`) instead of globally, so the same address can sign
  in with Google and with Facebook as two separate accounts. Previously signing
  in with a second provider failed with a 500 (`UNIQUE constraint failed:
  users.email`). Migration `009` rebuilds `users`.

### Fixed
- Migrations now run with foreign keys off, then check for dangling
  references before startup continues. Table rebuilds would otherwise
  cascade-delete every row referencing the rebuilt table. Rebuild migrations
  carry a guard that aborts if foreign keys are on.

## 2026-09-28

### Changed
- **`#[authed]`** (new `crates/macros`): server functions declare the signed-in
  context they need (`#[authed(user, pool)]`, `session`, `state`, or `ctx`)
  instead of repeating the AppState/Session/`user_id` preamble; 17 functions
  converted. Built on an `Authed` Axum extractor (`server.rs`), with compile
  errors for unknown/duplicate bindings, shadowed parameters, non-async
  functions, and wrong placement relative to `#[server]`. No behaviour change:
  signed-out calls still fail with "not authenticated".
- `Viewer::of` / `ViewedPost::new` in core are the single "author vs other"
  rule; `fetch_posts` uses them instead of an inline comparison.

### Added
- **Log out** (Settings → Account), backed by a `/logout` server route that ends
  the session and sends `Clear-Site-Data: "cache", "cookies", "storage"`, so the
  browser drops everything it holds for pray.rs, including a stale app bundle.
  Because it's a server route, opening https://pray.rs/logout works from any
  client version, and on Android it also resets the installed home-screen app
  (which shares Chrome's storage; clearing the app's own data in Android
  settings doesn't touch it).

### Fixed
- After a deploy, browsers could keep running the previous front-end bundle (same
  file names every release, and no `Cache-Control`, so they cached it by guesswork).
  The old bundle couldn't read the new `fetch_posts` response, so the book showed
  as empty. Caddy now serves `/pkg/*` directly with `Cache-Control: no-cache` and an
  ETag: browsers revalidate on every load (`304`, no re-download, when unchanged)
  and pick up new bundles immediately. The bundle is also compressed (zstd/gzip).
- `deploy/Caddyfile` previously marked `/pkg/*` `immutable` for a year, which
  would have made this permanent; it now matches production, and `setup.sh`
  writes the same config.

## 2026-09-27

### Changed
- **Entries are now a lifecycle, not a fixed kind.** `PostKind { Prayer, Praise }`
  is replaced by `PostState { Prayer, Thanksgiving, Released }`: a prayer can
  become a thanksgiving when answered, and either can be released. Entries can
  still start as a thanksgiving. Released is terminal and hidden from every
  listing (book tabs and MCP). Transitions are enforced in the `UPDATE` itself
  (`posts::transition`), so illegal or racing moves match no row.
- **All SQL is now compile-time checked.** Every repository query moved from
  runtime `sqlx::query` to the `query!` / `query_as!` macros, verified offline
  against committed `.sqlx/` metadata (`SQLX_OFFLINE=true` in
  `.cargo/config.toml`), so builds need no database. `scripts/sqlx-prepare.sh`
  regenerates the metadata from the migrations; `--check` (run by
  `deploy/deploy.sh`) refuses to ship stale metadata.
- A root `Makefile` for the routine commands (`make` lists them): `setup`, `dev`,
  `check`, `test`, `release`, `sqlx-prepare`, `sqlx-check`, `verify` (everything
  before a commit or deploy), `deploy`, `project-mcp`. README and CLAUDE.md now
  point at these targets.
- Migration `006_post_state` rebuilds `posts` with a `state` column (existing
  praises → thanksgivings) and drops the unused `prayer_id` link.

### Fixed
- Typing a note in the answer/release panel no longer turns the page (Space was
  being read as "next page"). The panel now holds the book on its page with
  leptosbook's new `use_folio_lock` while it's open, and leptosbook ignores keys
  and drags aimed at form fields. Requires leptosbook 0.2.0 (bumped from 0.1).

### Added
- **"Praying now" (+1).** Anyone who can see a prayer (the author, group members,
  anyone on a public prayer) can tap 🙏 as often as they pray; each press counts
  and the entry shows "12 prayers from 3 people". Stored per person in
  `post_prayers` (migration `008`), counted with `ON CONFLICT` upserts. MCP tool
  `pray_for` (new `pray` capability, granted to signed-in users). Thanksgivings
  and released entries can't be prayed for.
- **`PostAction`** in core: every action on an entry (`MoveTo(state)`,
  `PrayingNow`) with one `allowed(state, viewer)` rule. The UI (`action_ui`) and
  MCP (`action_tool` + dispatch) match on it exhaustively, so a new action fails
  to compile until both expose it.
- Author-only actions (give thanks, release) now appear on your own entries on
  every book tab, not only "Mine", so shared prayers can be answered too.
- "Answered — give thanks" and "Release" (with confirm) buttons under your own
  entries on the Mine tab, backed by a `set_post_state` server fn. Each opens a
  panel for an optional note ("How was it answered?", "Why are you releasing it?").
- **Transition log** (migration `007_post_transitions`): every move is recorded
  with its from/to state, note and time, in the same transaction as the state
  change. The entry's own text is never rewritten; its history is shown after
  it, starting with when it was first written ("Prayed · September 27, 2026",
  or "Gave thanks · …" for an entry written as a thanksgiving), then each move
  ("Answered · …" and the note).
- MCP tools `give_thanks` and `release_prayer`, each with an optional `note`;
  `create_prayer` takes an optional `state` (`prayer` | `thanksgiving`), and
  listings show each entry's state and history.
- Compile-time guards for new states: exhaustive matches in the UI (`offer`) and
  MCP (`transition_tool`) fail to compile until a new `PostState` is handled, and
  `crates/db/build.rs` fails the build if the migrations' `posts.state` CHECK list
  differs from `PostState::ALL`. Tests cover new transitions between existing
  states and that SQLite really enforces the constraint.

## 2026-06-17

### Security
- Post creation now verifies group membership before allowing a prayer to be
  addressed to a group, closing a gap where a crafted request could inject a post
  into a group the author didn't belong to. Empty content is also rejected
  server-side.

### Added
- **Share a prayer with a group** from the app UI: the "Mine" book tab shows a
  group picker + Share button under each prayer, and the book now has a tab per
  group to view what's been shared. Backed by a `share_prayer_to_group` server fn
  that checks group membership and reuses `posts::set_visibility`. (feature
  `2026-06-17-0d3bae18`)
- Group selection when composing a prayer, shown as a dropdown on mobile.
- Links to the Privacy, Terms, and Delete-My-Data pages from Settings (previously
  reachable only by URL).
- Claude Code setup instructions now lead with OAuth sign-in (`/mcp` →
  Authenticate / Reauthenticate) instead of a static API key.
- `project-mcp` `list_features` surfaces priority and can filter by it
  (`low`/`medium`/`high`), rendering `[status] [priority] id — title`. (feature
  `2026-06-17-1aac9148`)

### Changed
- Rebranded remaining "Thanksgivings" references to **pray.rs** across the legal
  pages, invitation SMS/email copy, and the email sender name.
- Extracted the compose visibility control into a `VisibilityField` component
  (returning `AnyView`) so release builds no longer overflow the view-type
  recursion limit — preferred over raising `recursion_limit`.

### Fixed
- Added explicit timeouts to all outbound HTTP: the OAuth userinfo fetch and the
  Resend mailer use `Client::builder()` with a 10s request / 5s connect timeout,
  and the Google/Facebook code exchanges are wrapped in a 15s ceiling, preventing
  handler tasks from parking on a hung upstream. (feature `2026-06-17-2d5bf428`)
- Group detail and other long pages now scroll instead of clipping content.
- The "Create group" button is full-width and padded to match its input.

## 2026-06-05

### Added
- Project scaffolded: Rust workspace with a Leptos SSR app, SQLite database,
  OAuth2 (Google + Facebook), book-shaped page navigation, and the project MCP
  server.
