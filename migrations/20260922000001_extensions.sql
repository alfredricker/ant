-- Extensions ant depends on. Runs in every environment via sqlx migrate.
--
-- Requires a privileged role: superuser locally, rds_superuser (or the cloud
-- provider's equivalent) in prod, where the extension must also be on the
-- provider's allow-list.
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
