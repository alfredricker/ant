-- Email + password sign-in, alongside Google. The hash lives in its own
-- table, not on users, so every users column stays safe to expose and
-- `SELECT *` into the shared User keeps working. Going passwordless later is
-- dropping this table.
--
-- `hash` is a PHC string ($argon2id$v=19$m=...,t=...,p=...$salt$hash), so
-- the parameters travel with each hash and can be raised without a migration.
CREATE TABLE user_passwords (
    user_id    UUID        PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    hash       TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER user_passwords_set_updated_at
    BEFORE UPDATE ON user_passwords
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
