CREATE TABLE exposed.funder_group_names (
    group_id UUID PRIMARY KEY REFERENCES exposed.funders(id) ON DELETE CASCADE,
    preferred_name TEXT NOT NULL CHECK (btrim(preferred_name) <> ''),
    family_key TEXT NOT NULL CHECK (btrim(family_key) <> '')
);
CREATE INDEX funder_group_names_preferred_name_trgm
    ON exposed.funder_group_names USING GIN (preferred_name exposed.gin_trgm_ops);
