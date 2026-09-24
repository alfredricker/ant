-- Passwordless auth: sign-in goes through external identities (Google for
-- now) and server-side sessions. With the hash gone, every users column is
-- safe to expose, so the server reads rows straight into the shared User.
ALTER TABLE users DROP COLUMN password_hash;

-- One row per linked login. `subject` is the provider's stable user id (the
-- OIDC `sub` claim); emails can change, subjects don't.
CREATE TABLE user_identities (
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    provider   TEXT        NOT NULL,
    subject    TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (provider, subject)
);

CREATE INDEX user_identities_user_id_idx ON user_identities (user_id);

-- The cookie holds a random token; only its SHA-256 is stored, so a leaked
-- table can't be replayed as live sessions.
CREATE TABLE sessions (
    token_hash BYTEA       PRIMARY KEY,
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX sessions_user_id_idx ON sessions (user_id);
