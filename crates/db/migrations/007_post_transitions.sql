-- Append-only log of lifecycle moves. Each row records a transition and the
-- author's optional note about it ("how it was answered", "why released").
-- The entry's own content is never rewritten; notes are shown after it.
CREATE TABLE post_transitions (
    id         TEXT PRIMARY KEY NOT NULL,
    post_id    TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    from_state TEXT NOT NULL CHECK (from_state IN ('prayer', 'thanksgiving', 'released')),
    to_state   TEXT NOT NULL CHECK (to_state IN ('prayer', 'thanksgiving', 'released')),
    note       TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_post_transitions_post ON post_transitions(post_id, created_at);
