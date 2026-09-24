-- Users. Mirrors ant_common::models::user::User, plus password_hash, which
-- stays server-side and is never serialized to the browser.
--
-- TIMESTAMPTZ (not TIMESTAMP) so sqlx maps it to chrono::DateTime<Utc>.

-- Shared by every table with an updated_at column.
CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE users (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    username      TEXT,
    email         TEXT        NOT NULL,
    password_hash TEXT        NOT NULL,
    is_superuser  BOOLEAN     NOT NULL DEFAULT false,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Case-insensitive uniqueness: "Alfred@x.com" and "alfred@x.com" are one account.
-- NULL usernames don't collide, so any number of users can skip picking one.
CREATE UNIQUE INDEX users_email_key    ON users (lower(email));
CREATE UNIQUE INDEX users_username_key ON users (lower(username));

CREATE TRIGGER users_set_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
