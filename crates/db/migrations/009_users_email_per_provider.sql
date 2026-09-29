-- Email is unique per provider, not globally: the same address may sign in
-- with Google and with Facebook, and each becomes its own account. Accounts
-- stay keyed by (provider, provider_id).
--
-- SQLite can't drop a column's UNIQUE, so `users` is rebuilt. Almost every
-- table references users(id) ON DELETE CASCADE, so this MUST run with foreign
-- keys OFF, or dropping the old table would delete everyone's posts, groups,
-- keys and prayers. `Db::open` runs migrations that way; this guard aborts
-- the migration (CHECK fails) if anything else runs it with them on.
CREATE TEMP TABLE _fk_off_guard (foreign_keys INTEGER CHECK (foreign_keys = 0));
INSERT INTO _fk_off_guard SELECT foreign_keys FROM pragma_foreign_keys;
DROP TABLE _fk_off_guard;

CREATE TABLE users_new (
    id           TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    email        TEXT NOT NULL,
    avatar_url   TEXT,
    provider     TEXT NOT NULL CHECK (provider IN ('google', 'facebook')),
    provider_id  TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (provider, provider_id),
    UNIQUE (provider, email)
);

INSERT INTO users_new (id, display_name, email, avatar_url, provider, provider_id, created_at)
SELECT id, display_name, email, avatar_url, provider, provider_id, created_at FROM users;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;
