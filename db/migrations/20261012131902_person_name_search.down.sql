DROP INDEX exposed.funder_aliases_person_name_codes;
DROP INDEX exposed.funders_person_name_codes;
DROP INDEX exposed.members_person_name_codes;
DROP FUNCTION exposed.person_name_distance(text, text);
DROP FUNCTION exposed.person_name_codes(text);
DROP FUNCTION exposed.person_name_words(text);
DROP EXTENSION fuzzystrmatch;
