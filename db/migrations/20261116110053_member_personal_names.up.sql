ALTER TABLE exposed.members
    ADD COLUMN personal_first_name text,
    ADD COLUMN personal_last_name text,
    ADD COLUMN parliamentary_name text,
    ADD COLUMN formal_name text,
    ADD CONSTRAINT members_complete_personal_name CHECK (
        (personal_first_name IS NULL AND personal_last_name IS NULL)
        OR (personal_first_name IS NOT NULL AND personal_last_name IS NOT NULL
            AND personal_first_name = btrim(personal_first_name) AND personal_first_name <> ''
            AND personal_last_name = btrim(personal_last_name) AND personal_last_name <> ''
            AND name = personal_first_name || ' ' || personal_last_name)
    );

UPDATE exposed.members SET parliamentary_name = name;

CREATE INDEX members_parliamentary_name_trigram
    ON exposed.members USING gin (parliamentary_name exposed.gin_trgm_ops);
CREATE INDEX members_formal_name_trigram
    ON exposed.members USING gin (formal_name exposed.gin_trgm_ops);
CREATE INDEX members_parliamentary_name_codes
    ON exposed.members USING gin (exposed.person_name_codes(parliamentary_name));
CREATE INDEX members_formal_name_codes
    ON exposed.members USING gin (exposed.person_name_codes(formal_name));
