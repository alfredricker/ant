-- Research and travel posts, for the wider "people with big goals" framing.
-- BEFORE keeps the enum's order matching PostKind, as in post_kinds.
ALTER TYPE post_kind ADD VALUE IF NOT EXISTS 'research' BEFORE 'art';
ALTER TYPE post_kind ADD VALUE IF NOT EXISTS 'travel' BEFORE 'startup';
