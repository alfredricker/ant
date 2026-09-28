-- Pictures attached to posts. The composer uploads one as soon as it's
-- picked, before the post exists, and puts the URL it gets back
-- (/api/post-images/{id}) in posts.image_url; so there's no post_id here,
-- and a picture whose post was never sent stays behind.
-- TODO: sweep images no post points at once they're a day old.
--
-- Like avatars, re-encoded by the server and kept in postgres; at most
-- 1600px on the long side, they're a few hundred KB.
CREATE TABLE post_images (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    uploader_id  UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    content_type TEXT        NOT NULL,
    data         BYTEA       NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Counting someone's recent uploads, for the hourly limit.
CREATE INDEX post_images_uploader_id_idx ON post_images (uploader_id, created_at DESC);
