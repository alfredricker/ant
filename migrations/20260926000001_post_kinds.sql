-- Music and contract posts. BEFORE keeps the enum's order matching
-- PostKind (it's the order `ORDER BY kind` would sort in).
--
-- A new enum value can't be used in the transaction that adds it; nothing
-- here does.
ALTER TYPE post_kind ADD VALUE IF NOT EXISTS 'music' BEFORE 'game';
ALTER TYPE post_kind ADD VALUE IF NOT EXISTS 'contract' BEFORE 'other';
