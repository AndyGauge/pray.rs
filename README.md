<div align="center">

# pray.rs

**A quiet prayer book for the web.**

Write prayers and thanksgivings, keep them private, share them with a group, or make them
public — read one page at a time, the way you'd turn through a book. No feed, no
algorithm, no performance.

[pray.rs](https://pray.rs)

</div>

---

## What it is

pray.rs is a small, intentional alternative to social media for prayer. It is built
around a few ideas:

- **A book, not a feed.** Prayers are paged through with arrow keys or swipe gestures —
  one at a time. There is no infinite scroll and nothing is ranked.
- **You choose who sees each prayer.** Every prayer is *private*, shared with a *group*
  you belong to, or *public*.
- **Prayers become thanksgivings.** When a prayer is answered, mark it so and write how;
  the original prayer is kept word for word, with the answer recorded after it. Either
  can be *released* when you're ready to let it go, and it leaves the book.
- **"Praying now."** Anyone who can see a prayer can tap 🙏 to let its author know
  they're praying, as often as they pray; the prayer shows the running tally.
- **Prayer groups.** Invite people by email, text, or a QR/link to pray together.
- **Bring your own AI.** A built-in [MCP](https://modelcontextprotocol.io) server lets
  you connect an AI assistant (Claude Code, claude.ai, Gemini CLI) to read and write
  your prayers on your behalf — authenticated as you, scoped to what you allow.

## Tech stack

| Concern        | Choice |
|----------------|--------|
| Frontend + SSR | [Leptos](https://leptos.dev) (server-rendered + WASM hydration) |
| Web server     | [Axum](https://github.com/tokio-rs/axum) |
| Database       | SQLite via [SQLx](https://github.com/launchbadge/sqlx) (runtime queries, no `DATABASE_URL` at compile time) |
| Auth           | OAuth2 (Google + Facebook) with `tower-sessions` |
| AI integration | Model Context Protocol server with an OAuth 2.1 authorization server (DCR + PKCE) and static API keys |
| Email          | [Resend](https://resend.com) for group invitations |
| Hosting        | DigitalOcean droplet behind [Caddy](https://caddyserver.com), `systemd`-managed |

## Repository layout

```
crates/
  core          Shared types (Post, User, Group, Visibility, …)
  db            SQLx SQLite layer — migrations + repositories
  app           The Leptos full-stack app (SSR binary + WASM hydrate lib)
  project-mcp   A separate stdio MCP server for tracking feature requests + changelog
deploy/         Deploy script, systemd unit, droplet setup
```

## Getting started

### Prerequisites

- Rust (stable) with the `wasm32-unknown-unknown` target:
  ```sh
  rustup target add wasm32-unknown-unknown
  ```
- [`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos):
  ```sh
  cargo install cargo-leptos
  ```

### Configuration

Copy the example env file and fill it in:

```sh
cp .env.example .env
```

| Variable | Purpose |
|----------|---------|
| `DATABASE_URL` | SQLite path, e.g. `sqlite://thanksgivings.db` |
| `BASE_URL` | Public base URL (used in invite links) |
| `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` | Google OAuth |
| `FACEBOOK_CLIENT_ID` / `FACEBOOK_CLIENT_SECRET` | Facebook OAuth |
| `RESEND_API_KEY` / `RESEND_FROM` | Outbound invitation email |
| `DEV_AUTH_BYPASS` | Set to `1` to enable a local dev login that skips real OAuth |

`make setup` installs the prerequisites below and creates `.env` for you. Run `make`
to list every command.

### Run it

```sh
make dev
```

Then open <http://127.0.0.1:3000>. With `DEV_AUTH_BYPASS=1`, use the dev login to sign
in without configuring OAuth.

### Check your work

`cargo check` alone won't catch everything. Before committing or deploying, run:

```sh
make verify   # sqlx metadata up to date, both build targets, tests, release compile
```

After changing any SQL query or adding a migration, run `make sqlx-prepare` and commit
the regenerated `.sqlx/` (queries are checked at compile time, offline).

See [CLAUDE.md](CLAUDE.md) for project conventions and gotchas (notably: prefer
extracting components over raising `recursion_limit`, and always bind SQL values rather
than interpolating them).

## Connect an AI assistant

In the app, go to **Settings → Connect your AI**. The recommended path signs in with
your account over OAuth — no API key to manage:

```sh
claude mcp add --transport http prayers https://pray.rs/mcp
```

Then run `/mcp` in Claude Code, select **prayers**, and **Authenticate** (or
**Reauthenticate** to move from public prayers to your own account). A static API key is
also available as an alternative.

Tools the assistant gets (depending on what you've authorized):

| Tool | What it does |
|------|--------------|
| `list_public_prayers`, `list_my_prayers`, `list_group_prayers` | Read entries, each with its state, history (answer/release notes) and prayer tally |
| `create_prayer` | Write a new entry; `state` is `prayer` (default) or `thanksgiving` |
| `give_thanks` | Mark a prayer answered, with an optional `note` on how |
| `release_prayer` | Release a prayer or thanksgiving (hides it), with an optional `note` on why |
| `pray_for` | "Praying now" (+1) on any prayer you can see; every call counts |
| `edit_prayer`, `delete_prayer`, `set_visibility` | Change the text, delete, or change who can see an entry |
| `share_to_group`, `list_my_groups`, `create_group`, `invite_to_group` | Groups |

## Deploying

```sh
make deploy               # or: make deploy HOST=user@host
```

This cross-compiles the Linux server binary with `cargo zigbuild`, builds the optimized
WASM/CSS with `cargo leptos build --release`, uploads to the droplet, and restarts the
service. Requires `zig` and `cargo-zigbuild`.

## License

© Andrew Gauger. Personal project — all rights reserved unless noted otherwise.
