CREATE TABLE IF NOT EXISTS group_join_tokens (
    id         TEXT PRIMARY KEY NOT NULL,
    group_id   TEXT NOT NULL UNIQUE REFERENCES groups(id) ON DELETE CASCADE,
    created_by TEXT NOT NULL REFERENCES users(id),
    token      TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);
