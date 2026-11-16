CREATE TABLE exposed.chambers (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    name TEXT NOT NULL UNIQUE
);
INSERT INTO exposed.chambers (name) VALUES ('Commons'), ('Lords');
CREATE TABLE exposed.membership_kinds (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    name TEXT NOT NULL UNIQUE,
    chamber_id UUID NOT NULL REFERENCES exposed.chambers (id)
);
INSERT INTO exposed.membership_kinds (name, chamber_id)
SELECT kind.name, chamber.id
FROM (VALUES
    ('Elected MP', 'Commons'),
    ('Life peer', 'Lords'),
    ('Hereditary peer', 'Lords'),
    ('Excepted hereditary peer', 'Lords'),
    ('Hereditary peer given life peerage', 'Lords'),
    ('Bishop', 'Lords'),
    ('Judicial peer', 'Lords')
) AS kind(name, chamber_name)
JOIN exposed.chambers chamber ON chamber.name = kind.chamber_name;
ALTER TABLE exposed.members RENAME COLUMN latest_membership_from TO membership_from;
ALTER TABLE exposed.members
    ADD COLUMN membership_kind UUID REFERENCES exposed.membership_kinds (id),
    ADD COLUMN membership_start_date TEXT,
    ADD COLUMN membership_end_date TEXT,
    ADD COLUMN membership_status TEXT;
-- The existing import contains only MPs, so every existing row is an elected MP.
-- Preserve the member modification times while assigning the new classification.
ALTER TABLE exposed.members DISABLE TRIGGER members_set_updated_at;
UPDATE exposed.members SET membership_kind = (SELECT id FROM exposed.membership_kinds WHERE name = 'Elected MP');
ALTER TABLE exposed.members ENABLE TRIGGER members_set_updated_at;
ALTER TABLE exposed.members
    ALTER COLUMN membership_kind SET NOT NULL,
    DROP COLUMN latest_house,
    DROP COLUMN is_current_commons;
CREATE TABLE exposed.simple_declarations (
    member_id UUID NOT NULL REFERENCES exposed.members (id) ON DELETE CASCADE,
    category_position BIGINT NOT NULL CHECK (category_position >= 0),
    category_source_id BIGINT NOT NULL CHECK (category_source_id BETWEEN 1 AND 4294967295),
    category_name TEXT,
    category_sort_order INTEGER NOT NULL,
    node_position BIGINT NOT NULL CHECK (node_position >= 0),
    parent_position BIGINT CHECK (parent_position >= 0 AND parent_position < node_position),
    sibling_position BIGINT NOT NULL CHECK (sibling_position >= 0),
    source_id BIGINT NOT NULL CHECK (source_id BETWEEN 1 AND 4294967295),
    declaration_text TEXT,
    created_when TEXT,
    last_amended_when TEXT,
    deleted_when TEXT,
    is_correction BOOLEAN NOT NULL,
    fetched_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (member_id, category_position, node_position),
    FOREIGN KEY (member_id, category_position, parent_position)
        REFERENCES exposed.simple_declarations (member_id, category_position, node_position),
    UNIQUE NULLS NOT DISTINCT (member_id, category_position, parent_position, sibling_position)
);
