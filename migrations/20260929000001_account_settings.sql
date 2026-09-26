-- Account page: avatars, email changes and per-user settings.

-- Whether the email is known to belong to the account's owner. Google
-- vouches for the addresses it hands us; password sign-ups and email changes
-- don't verify theirs (yet). Linking a Google identity by email only trusts
-- an account whose email is verified (see db::users).
ALTER TABLE users ADD COLUMN email_verified BOOLEAN NOT NULL DEFAULT false;
UPDATE users SET email_verified = true
WHERE id IN (SELECT user_id FROM user_identities);

-- Where the avatar lives: the Google profile photo, or our own
-- /api/users/{id}/avatar?v=… after an upload. NULL shows a default insect.
ALTER TABLE users ADD COLUMN avatar_url TEXT;

-- Uploaded avatars, already re-encoded to a small square by the server.
-- In postgres rather than on disk so no volume is needed; they're ~20 KB.
CREATE TABLE user_avatars (
    user_id      UUID        PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    content_type TEXT        NOT NULL,
    data         BYTEA       NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Preferences, one row per user who has changed one; no row means defaults.
-- Kept off users so User stays a plain wire type read with SELECT *.
CREATE TYPE theme AS ENUM ('system', 'light', 'dark');

CREATE TABLE user_settings (
    user_id    UUID        PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    theme      theme       NOT NULL DEFAULT 'system',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER user_settings_set_updated_at
    BEFORE UPDATE ON user_settings
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
