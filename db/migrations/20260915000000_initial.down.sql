-- Drop dependent tables first; their triggers and indexes are removed with them.
DROP TABLE exposed.funding_entries;
DROP TABLE exposed.funders;
DROP TABLE exposed.declarations;
DROP TABLE exposed.members;

DROP FUNCTION exposed.set_updated_at();

DROP EXTENSION pg_trgm;
DROP SCHEMA exposed;
