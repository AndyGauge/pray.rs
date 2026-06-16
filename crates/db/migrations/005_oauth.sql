-- OAuth 2.1 authorization server state for the MCP endpoint.
-- Clients are public (PKCE, no secret) and register dynamically (RFC 7591).

CREATE TABLE IF NOT EXISTS oauth_clients (
    client_id     TEXT PRIMARY KEY,
    client_name   TEXT,
    redirect_uris TEXT NOT NULL,           -- JSON array of allowed redirect URIs
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Short-lived authorization codes, bound to a PKCE challenge (RFC 7636).
CREATE TABLE IF NOT EXISTS oauth_auth_codes (
    code_hash      TEXT PRIMARY KEY,
    client_id      TEXT NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    user_id        TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    redirect_uri   TEXT NOT NULL,
    code_challenge TEXT NOT NULL,          -- S256 challenge
    scope          TEXT,
    resource       TEXT,
    expires_at     TEXT NOT NULL,
    used           INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Issued access + refresh tokens (hashed, like api_keys).
CREATE TABLE IF NOT EXISTS oauth_access_tokens (
    token_hash    TEXT PRIMARY KEY,
    refresh_hash  TEXT UNIQUE,
    client_id     TEXT NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope         TEXT,
    expires_at    TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_oauth_tokens_user ON oauth_access_tokens(user_id);
