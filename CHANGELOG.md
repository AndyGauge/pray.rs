# Changelog

All notable changes to pray.rs are recorded here. Dates are in `YYYY-MM-DD`.

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
