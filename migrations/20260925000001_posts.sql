-- Posts looking for collaborators, and what people do with them: public
-- comments and likes, and private responses that only the post's author sees.
--
-- Enum values mirror the Rust enums in src/models (serde and sqlx both use
-- the lowercase names). Adding a value is `ALTER TYPE ... ADD VALUE`.

CREATE TYPE post_kind AS ENUM ('passion', 'school', 'art', 'game', 'startup', 'other');

-- `closed`: the author found who they needed, or stopped looking. Closed
-- posts stay readable but drop out of browsing and stop taking responses.
CREATE TYPE post_status AS ENUM ('open', 'closed');

CREATE TABLE posts (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    author_id   UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind        post_kind   NOT NULL,
    status      post_status NOT NULL DEFAULT 'open',
    title       TEXT        NOT NULL CHECK (char_length(title) BETWEEN 1 AND 120),
    body        TEXT        NOT NULL CHECK (char_length(body) BETWEEN 1 AND 10000),
    -- Skills or roles wanted ("Rust", "Pixel art"), shown as tags.
    looking_for TEXT[]      NOT NULL DEFAULT '{}' CHECK (cardinality(looking_for) <= 10),
    -- Has money behind it; set by the author, not verified.
    funded      BOOLEAN     NOT NULL DEFAULT false,
    -- CDN URL of the optional attached image.
    image_url   TEXT,
    -- Keyword half of search. The semantic half (a pgvector embedding) gets
    -- its own migration once the model, and so the dimension, is chosen.
    search      TSVECTOR    GENERATED ALWAYS AS (
                    setweight(to_tsvector('english', title), 'A') ||
                    setweight(to_tsvector('english', body), 'B')
                ) STORED,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Browsing: newest open posts first.
CREATE INDEX posts_open_created_at_idx ON posts (created_at DESC) WHERE status = 'open';
CREATE INDEX posts_author_id_idx       ON posts (author_id, created_at DESC);
CREATE INDEX posts_search_idx          ON posts USING GIN (search);
CREATE INDEX posts_looking_for_idx     ON posts USING GIN (looking_for);

CREATE TRIGGER posts_set_updated_at
    BEFORE UPDATE ON posts
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Private "I'm interested" replies, one per person per post. Only the post's
-- author and the responder can see one. Accepting one opens a conversation;
-- the responder withdraws by deleting it.
CREATE TYPE response_status AS ENUM ('pending', 'accepted', 'declined');

CREATE TABLE post_responses (
    id             UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id        UUID            NOT NULL REFERENCES posts (id) ON DELETE CASCADE,
    responder_id   UUID            NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    message        TEXT            NOT NULL CHECK (char_length(message) BETWEEN 1 AND 4000),
    -- CDN URL of an optional attachment (portfolio piece, sketch, ...).
    attachment_url TEXT,
    status         response_status NOT NULL DEFAULT 'pending',
    -- Lets the author filter responses by what they say.
    search         TSVECTOR        GENERATED ALWAYS AS (to_tsvector('english', message)) STORED,
    created_at     TIMESTAMPTZ     NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ     NOT NULL DEFAULT now(),
    UNIQUE (post_id, responder_id)
);

CREATE INDEX post_responses_post_id_idx      ON post_responses (post_id, status, created_at DESC);
CREATE INDEX post_responses_responder_id_idx ON post_responses (responder_id, created_at DESC);
CREATE INDEX post_responses_search_idx       ON post_responses USING GIN (search);

CREATE TRIGGER post_responses_set_updated_at
    BEFORE UPDATE ON post_responses
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Public discussion under a post. Flat for now; replies would add a
-- nullable parent_id.
CREATE TABLE post_comments (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id    UUID        NOT NULL REFERENCES posts (id) ON DELETE CASCADE,
    author_id  UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    body       TEXT        NOT NULL CHECK (char_length(body) BETWEEN 1 AND 2000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX post_comments_post_id_idx ON post_comments (post_id, created_at);

CREATE TRIGGER post_comments_set_updated_at
    BEFORE UPDATE ON post_comments
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE post_likes (
    post_id    UUID        NOT NULL REFERENCES posts (id) ON DELETE CASCADE,
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (post_id, user_id)
);

CREATE INDEX post_likes_user_id_idx ON post_likes (user_id);
