-- Dev-only example data: users, posts, comments, likes, responses and the
-- conversations that accepted responses opened.
--
-- Not a migration: sqlx would record it in _sqlx_migrations, the app's
-- embedded migrator (which only knows migrations/) would then refuse to
-- start, and every edit here would break its checksum. Instead the dev
-- entrypoint runs it after `sqlx migrate run` on every start. That's safe
-- because every row has a fixed id and inserts skip rows that exist, so edit
-- freely; to see an edit to an existing row, delete the row (or the whole
-- database volume) first.
--
-- Run by hand: docker exec ant_dev_app psql "$DATABASE_URL" -f /app/seed.sql
--
-- Ids are fixed so they're recognisable, and grouped by table:
--   users 5eed0000-…, posts 5eed0001-…, responses 5eed0002-…,
--   comments 5eed0003-…, conversations 5eed0004-…, messages 5eed0005-…
-- Times are relative to the first run, so the data looks recent.
--
-- The seeded users have no linked Google identity and can't sign in; they're
-- there to fill the pages you browse as yourself.

BEGIN;

INSERT INTO users (id, username, email, created_at) VALUES
    ('5eed0000-0000-4000-8000-000000000001', 'maya',  'maya@ant.test',  now() - interval '40 days'),
    ('5eed0000-0000-4000-8000-000000000002', 'jonah', 'jonah@ant.test', now() - interval '35 days'),
    ('5eed0000-0000-4000-8000-000000000003', 'priya', 'priya@ant.test', now() - interval '30 days'),
    ('5eed0000-0000-4000-8000-000000000004', 'leo',   'leo@ant.test',   now() - interval '25 days'),
    ('5eed0000-0000-4000-8000-000000000005', 'sam',   'sam@ant.test',   now() - interval '20 days'),
    -- Hasn't picked a username: shows as "someone".
    ('5eed0000-0000-4000-8000-000000000006', NULL,    'quiet@ant.test', now() - interval '10 days')
ON CONFLICT DO NOTHING;

INSERT INTO posts (id, author_id, kind, status, title, body, looking_for, funded, image_url, created_at, updated_at) VALUES
    ('5eed0001-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000001', 'game', 'open',
     'Co-op puzzle game for two',
     'Small asymmetric puzzler: one player sees the room, the other sees the rules. The prototype plays well '
     'in Godot and friends keep asking for "one more level". I need someone who enjoys pixel art to give it '
     'a look, and ideally a second brain for level design.',
     '{"Pixel art","Game design"}', false, NULL,
     now() - interval '2 days', now() - interval '2 days'),
    ('5eed0001-0000-4000-8000-000000000002', '5eed0000-0000-4000-8000-000000000002', 'school', 'open',
     'Senior capstone: transit delay model',
     'Predicting bus bunching from open GTFS-realtime feeds for my senior capstone. The data is cleaned and '
     'I have a baseline gradient-boosted model. Looking for a second pair of hands on modelling and on '
     'making the results readable for the city transit folks.',
     '{"Python","ML","Data viz"}', false, NULL,
     now() - interval '5 days', now() - interval '5 days'),
    ('5eed0001-0000-4000-8000-000000000003', '5eed0000-0000-4000-8000-000000000003', 'startup', 'open',
     'Local-first invoicing for freelancers',
     'Bootstrapped invoicing app that works offline and syncs when it can. We have a small, paying user base '
     'and budget set aside for a design-minded collaborator to rework onboarding and the invoice editor.',
     '{"Design","Frontend"}', true, 'https://picsum.photos/seed/ant-invoicing/1200/630',
     now() - interval '1 day', now() - interval '1 day'),
    ('5eed0001-0000-4000-8000-000000000004', '5eed0000-0000-4000-8000-000000000004', 'passion', 'open',
     'Field recording archive',
     'Mapping a year of city soundscapes: one recording a week from the same twelve corners. I have the '
     'recordings and a rough map; I need someone who enjoys audio tooling to build the playback and '
     'comparison views.',
     '{"Rust","Audio","Maps"}', false, 'https://picsum.photos/seed/ant-field-recording/1200/630',
     now() - interval '8 days', now() - interval '8 days'),
    ('5eed0001-0000-4000-8000-000000000005', '5eed0000-0000-4000-8000-000000000005', 'art', 'open',
     'Zine about neighbourhood corner shops',
     'Twenty-four pages of drawings and short interviews with the people who run corner shops around here. '
     'The drawings are half done. Looking for a writer who likes talking to strangers.',
     '{"Writing","Interviews"}', false, NULL,
     now() - interval '3 days', now() - interval '3 days'),
    -- Closed: out of browse, still on leo's profile.
    ('5eed0001-0000-4000-8000-000000000006', '5eed0000-0000-4000-8000-000000000004', 'music', 'closed',
     'EP needs a mixer',
     'Four tracks recorded at home, ambient-ish with field recordings layered in. Looking for someone who '
     'enjoys mixing and can explain what they changed and why.',
     '{"Mixing","Mastering"}', false, NULL,
     now() - interval '20 days', now() - interval '6 days'),
    ('5eed0001-0000-4000-8000-000000000007', '5eed0000-0000-4000-8000-000000000003', 'contract', 'open',
     'Two-week Flutter contract',
     'Paid, fixed scope: finish the mobile companion app for the invoicing product (receipt capture and '
     'sync status). Designs are done; the API exists.',
     '{"Flutter","Mobile"}', true, NULL,
     now() - interval '6 hours', now() - interval '6 hours')
ON CONFLICT DO NOTHING;

INSERT INTO post_comments (id, post_id, author_id, body, created_at, updated_at) VALUES
    ('5eed0003-0000-4000-8000-000000000001', '5eed0001-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000002',
     'The one-sees-the-room idea is great. Is it local co-op or online?',
     now() - interval '47 hours', now() - interval '47 hours'),
    ('5eed0003-0000-4000-8000-000000000002', '5eed0001-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000001',
     'Online for now, local split-screen is on the list.',
     now() - interval '46 hours', now() - interval '46 hours'),
    ('5eed0003-0000-4000-8000-000000000003', '5eed0001-0000-4000-8000-000000000002', '5eed0000-0000-4000-8000-000000000004',
     'Our city publishes GTFS-realtime too, happy to share a scraper if it helps.',
     now() - interval '4 days', now() - interval '4 days'),
    ('5eed0003-0000-4000-8000-000000000004', '5eed0001-0000-4000-8000-000000000004', '5eed0000-0000-4000-8000-000000000005',
     'Would love to draw the twelve corners for the map.',
     now() - interval '7 days', now() - interval '7 days'),
    ('5eed0003-0000-4000-8000-000000000005', '5eed0001-0000-4000-8000-000000000004', '5eed0000-0000-4000-8000-000000000006',
     'Following this one.',
     now() - interval '6 days', now() - interval '6 days')
ON CONFLICT DO NOTHING;

INSERT INTO post_likes (post_id, user_id, created_at) VALUES
    ('5eed0001-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000002', now() - interval '47 hours'),
    ('5eed0001-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000003', now() - interval '40 hours'),
    ('5eed0001-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000005', now() - interval '30 hours'),
    ('5eed0001-0000-4000-8000-000000000002', '5eed0000-0000-4000-8000-000000000004', now() - interval '4 days'),
    ('5eed0001-0000-4000-8000-000000000003', '5eed0000-0000-4000-8000-000000000001', now() - interval '20 hours'),
    ('5eed0001-0000-4000-8000-000000000004', '5eed0000-0000-4000-8000-000000000005', now() - interval '7 days'),
    ('5eed0001-0000-4000-8000-000000000004', '5eed0000-0000-4000-8000-000000000006', now() - interval '6 days'),
    ('5eed0001-0000-4000-8000-000000000004', '5eed0000-0000-4000-8000-000000000001', now() - interval '5 days'),
    ('5eed0001-0000-4000-8000-000000000005', '5eed0000-0000-4000-8000-000000000002', now() - interval '2 days')
ON CONFLICT DO NOTHING;

-- One of each state: pending with and without attachments, accepted (each
-- has a conversation below), declined.
INSERT INTO post_responses (id, post_id, responder_id, message, attachment_url, status, created_at, updated_at) VALUES
    ('5eed0002-0000-4000-8000-000000000001', '5eed0001-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000005',
     'I do pixel art for fun and would love a game to aim it at. A few sprites and tilesets attached.',
     'https://picsum.photos/seed/ant-sam-pixels/800/600', 'pending',
     now() - interval '36 hours', now() - interval '36 hours'),
    ('5eed0002-0000-4000-8000-000000000002', '5eed0001-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000004',
     'Not an artist, but I could do the sound design if that is ever on the list.',
     NULL, 'pending',
     now() - interval '30 hours', now() - interval '30 hours'),
    ('5eed0002-0000-4000-8000-000000000003', '5eed0001-0000-4000-8000-000000000003', '5eed0000-0000-4000-8000-000000000001',
     'Product designer by day. I redid onboarding for a budgeting app last year; case study attached.',
     'https://picsum.photos/seed/ant-maya-case-study/800/600', 'accepted',
     now() - interval '22 hours', now() - interval '18 hours'),
    ('5eed0002-0000-4000-8000-000000000004', '5eed0001-0000-4000-8000-000000000003', '5eed0000-0000-4000-8000-000000000002',
     'I can help with the frontend, mostly React though.',
     NULL, 'declined',
     now() - interval '21 hours', now() - interval '17 hours'),
    ('5eed0002-0000-4000-8000-000000000005', '5eed0001-0000-4000-8000-000000000004', '5eed0000-0000-4000-8000-000000000003',
     'I have built a couple of map-heavy apps in Rust and would enjoy an audio one.',
     NULL, 'pending',
     now() - interval '6 days', now() - interval '6 days'),
    ('5eed0002-0000-4000-8000-000000000006', '5eed0001-0000-4000-8000-000000000006', '5eed0000-0000-4000-8000-000000000001',
     'I mix my own band''s stuff and would love to hear these.',
     NULL, 'accepted',
     now() - interval '15 days', now() - interval '14 days'),
    ('5eed0002-0000-4000-8000-000000000007', '5eed0001-0000-4000-8000-000000000002', '5eed0000-0000-4000-8000-000000000006',
     'Stats minor, could help with the modelling.',
     NULL, 'pending',
     now() - interval '3 days', now() - interval '3 days')
ON CONFLICT DO NOTHING;

-- The conversations the two accepted responses opened: priya and maya about
-- the invoicing app, leo and maya about the EP. direct_key is built the way
-- db::conversations::find_or_create_direct builds it.
INSERT INTO conversations (id, direct_key, post_id, created_at, last_message_at)
SELECT c.id::uuid,
       least(c.a::uuid, c.b::uuid)::text || ':' || greatest(c.a::uuid, c.b::uuid)::text,
       c.post_id::uuid, now() - c.age, now() - c.age
FROM (VALUES
    ('5eed0004-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000003', '5eed0000-0000-4000-8000-000000000001',
     '5eed0001-0000-4000-8000-000000000003', interval '18 hours'),
    ('5eed0004-0000-4000-8000-000000000002', '5eed0000-0000-4000-8000-000000000004', '5eed0000-0000-4000-8000-000000000001',
     '5eed0001-0000-4000-8000-000000000006', interval '14 days')
) AS c (id, a, b, post_id, age)
ON CONFLICT DO NOTHING;

INSERT INTO messages (id, conversation_id, sender_id, body, created_at) VALUES
    ('5eed0005-0000-4000-8000-000000000001', '5eed0004-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000003',
     'Loved the case study. Want to do a short call this week?', now() - interval '18 hours'),
    ('5eed0005-0000-4000-8000-000000000002', '5eed0004-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000001',
     'Sure! Thursday evening works for me.', now() - interval '16 hours'),
    ('5eed0005-0000-4000-8000-000000000003', '5eed0004-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000003',
     'Thursday at 7 then. I''ll send over the current onboarding flow before.', now() - interval '15 hours'),
    ('5eed0005-0000-4000-8000-000000000004', '5eed0004-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000003',
     'Here it is: five screens, the third one is where people drop off.', now() - interval '2 hours'),
    ('5eed0005-0000-4000-8000-000000000005', '5eed0004-0000-4000-8000-000000000002', '5eed0000-0000-4000-8000-000000000004',
     'Stems are in the shared folder. Track two is the messy one.', now() - interval '14 days'),
    ('5eed0005-0000-4000-8000-000000000006', '5eed0004-0000-4000-8000-000000000002', '5eed0000-0000-4000-8000-000000000001',
     'First pass on all four is up. Pulled the field recordings back a bit on two.', now() - interval '9 days'),
    ('5eed0005-0000-4000-8000-000000000007', '5eed0004-0000-4000-8000-000000000002', '5eed0000-0000-4000-8000-000000000004',
     'These sound great. Closing the post, thank you!', now() - interval '6 days')
ON CONFLICT DO NOTHING;

-- Maya hasn't read priya's latest message, so her inbox shows one unread.
INSERT INTO conversation_members (conversation_id, user_id, joined_at, last_read_at) VALUES
    ('5eed0004-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000003', now() - interval '18 hours', now() - interval '2 hours'),
    ('5eed0004-0000-4000-8000-000000000001', '5eed0000-0000-4000-8000-000000000001', now() - interval '18 hours', now() - interval '15 hours'),
    ('5eed0004-0000-4000-8000-000000000002', '5eed0000-0000-4000-8000-000000000004', now() - interval '14 days', now() - interval '6 days'),
    ('5eed0004-0000-4000-8000-000000000002', '5eed0000-0000-4000-8000-000000000001', now() - interval '14 days', now() - interval '6 days')
ON CONFLICT DO NOTHING;

-- Inbox order follows the newest message, as db::conversations::send keeps it.
UPDATE conversations c
SET last_message_at = m.latest
FROM (SELECT conversation_id, max(created_at) AS latest FROM messages GROUP BY conversation_id) m
WHERE m.conversation_id = c.id
  AND c.id::text LIKE '5eed0004-%'
  AND c.last_message_at IS DISTINCT FROM m.latest;

COMMIT;
