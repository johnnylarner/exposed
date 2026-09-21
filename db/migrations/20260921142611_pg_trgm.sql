-- migrate:up

CREATE EXTENSION pg_trgm;

-- migrate:down

DROP EXTENSION pg_trgm;
