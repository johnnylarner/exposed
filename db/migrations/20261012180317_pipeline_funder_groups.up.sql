CREATE TABLE exposed.funder_alias_groups (
    funder_id UUID PRIMARY KEY REFERENCES exposed.funders(id) ON DELETE CASCADE,
    group_id UUID NOT NULL REFERENCES exposed.funders(id) ON DELETE CASCADE
);
CREATE INDEX funder_alias_groups_group_id_idx ON exposed.funder_alias_groups(group_id);

CREATE TABLE exposed.funder_alias_links (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    left_funder_id UUID NOT NULL REFERENCES exposed.funders(id) ON DELETE CASCADE,
    right_funder_id UUID NOT NULL REFERENCES exposed.funders(id) ON DELETE CASCADE,
    normalized_alias TEXT NOT NULL,
    left_alias TEXT NOT NULL,
    right_alias TEXT NOT NULL,
    CHECK (left_funder_id < right_funder_id)
);
CREATE TABLE exposed.funder_family_links (
    left_funder_id UUID NOT NULL REFERENCES exposed.funders(id) ON DELETE CASCADE,
    right_funder_id UUID NOT NULL REFERENCES exposed.funders(id) ON DELETE CASCADE,
    family_key TEXT NOT NULL,
    left_name TEXT NOT NULL,
    right_name TEXT NOT NULL,
    PRIMARY KEY (left_funder_id, right_funder_id, family_key),
    CHECK (left_funder_id < right_funder_id)
);
