-- "Praying now" (+1) presses. One row per (entry, person); every press bumps
-- `count`, so someone can pray for an entry as many times as they like.
CREATE TABLE post_prayers (
    post_id         TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    count           INTEGER NOT NULL DEFAULT 1 CHECK (count > 0),
    first_prayed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_prayed_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (post_id, user_id)
);
