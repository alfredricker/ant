-- Direct messages. A conversation has members rather than two fixed user
-- columns, so group chats stay possible, but for now every conversation is
-- between exactly two people and there's at most one per pair.

CREATE TABLE conversations (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- "<lower user id>:<higher user id>" for a two-person conversation, so a
    -- pair can't end up with two; NULL would mean a group.
    direct_key      TEXT        UNIQUE,
    -- The post it started from, if any (accepting a response starts one).
    post_id         UUID        REFERENCES posts (id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Bumped with each message, for sorting the inbox.
    last_message_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE conversation_members (
    conversation_id UUID        NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    user_id         UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Everything after this is unread; NULL means nothing has been read.
    last_read_at    TIMESTAMPTZ,
    PRIMARY KEY (conversation_id, user_id)
);

CREATE INDEX conversation_members_user_id_idx ON conversation_members (user_id);

CREATE TABLE messages (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID        NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    sender_id       UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    body            TEXT        NOT NULL CHECK (char_length(body) BETWEEN 1 AND 4000),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX messages_conversation_id_idx ON messages (conversation_id, created_at DESC);
