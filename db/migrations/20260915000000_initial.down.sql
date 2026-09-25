-- Drop dependent tables first; their triggers and indexes are removed with them.
DROP TABLE exposed.funding_entries;
DROP TABLE exposed.funders;
DROP TABLE exposed.declarations;
DROP TABLE exposed.member_terms;
DROP TABLE exposed.members;
DROP TABLE exposed.parliament_terms;

DROP FUNCTION exposed.set_updated_at();
DROP FUNCTION exposed.check_term_service_start();

DROP EXTENSION pg_trgm;
DROP SCHEMA exposed;
