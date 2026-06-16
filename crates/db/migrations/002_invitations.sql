CREATE TABLE IF NOT EXISTS invitations (
    id           TEXT PRIMARY KEY,
    group_id     TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    invited_by   TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    contact      TEXT NOT NULL,                -- email address or phone number
    contact_type TEXT NOT NULL CHECK (contact_type IN ('email', 'phone')),
    token        TEXT NOT NULL UNIQUE,         -- URL-safe random token
    status       TEXT NOT NULL DEFAULT 'pending'
                     CHECK (status IN ('pending', 'accepted', 'declined')),
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    expires_at   TEXT NOT NULL                 -- 7 days after creation
);

CREATE INDEX IF NOT EXISTS idx_inv_token    ON invitations(token);
CREATE INDEX IF NOT EXISTS idx_inv_group    ON invitations(group_id);
CREATE INDEX IF NOT EXISTS idx_inv_contact  ON invitations(contact);
