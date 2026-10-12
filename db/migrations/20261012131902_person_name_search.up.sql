CREATE EXTENSION fuzzystrmatch WITH SCHEMA exposed;

CREATE FUNCTION exposed.person_name_words(value text) RETURNS text[]
LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE AS $$
    SELECT COALESCE(array_agg(word), ARRAY[]::text[])
    FROM regexp_split_to_table(lower(value), '[^[:alnum:]]+') AS word
    WHERE word <> ''
$$;

CREATE FUNCTION exposed.person_name_codes(value text) RETURNS text[]
LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE AS $$
    SELECT COALESCE(array_agg(exposed.soundex(word)), ARRAY[]::text[])
    FROM unnest(exposed.person_name_words(value)) AS word
$$;

CREATE INDEX members_person_name_codes ON exposed.members USING gin (exposed.person_name_codes(name));
CREATE INDEX funders_person_name_codes ON exposed.funders USING gin (exposed.person_name_codes(funder_name));
CREATE INDEX funder_aliases_person_name_codes ON exposed.funder_aliases USING gin (exposed.person_name_codes(funder_alias));
