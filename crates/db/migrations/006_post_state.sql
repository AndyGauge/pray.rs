-- Replace posts.kind ('prayer' | 'praise') with a lifecycle state:
--   prayer → thanksgiving, and either → released (terminal, hidden).
-- Existing praises become thanksgivings. The prayer_id link ("praise answers
-- prayer") is superseded by transitioning the prayer itself, so it is dropped.
-- SQLite can't alter a CHECK constraint, so the table is rebuilt.
CREATE TABLE posts_new (
    id         TEXT PRIMARY KEY,
    author_id  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    state      TEXT NOT NULL DEFAULT 'prayer'
               CHECK (state IN ('prayer', 'thanksgiving', 'released')),
    content    TEXT NOT NULL,
    visibility TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT INTO posts_new (id, author_id, state, content, visibility, created_at, updated_at)
SELECT id, author_id,
       CASE kind WHEN 'praise' THEN 'thanksgiving' ELSE 'prayer' END,
       content, visibility, created_at, updated_at
FROM posts;

DROP TABLE posts;
ALTER TABLE posts_new RENAME TO posts;

CREATE INDEX IF NOT EXISTS idx_posts_author  ON posts(author_id);
CREATE INDEX IF NOT EXISTS idx_posts_vis     ON posts(visibility);
CREATE INDEX IF NOT EXISTS idx_posts_created ON posts(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_posts_state   ON posts(state);
