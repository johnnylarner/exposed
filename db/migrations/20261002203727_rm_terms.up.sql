-- Add up migration script here

DROP TABLE IF EXISTS exposed.member_terms;
DROP TABLE IF EXISTS exposed.parliament_terms;
DROP FUNCTION IF EXISTS exposed.check_term_service_start();

