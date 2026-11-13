ALTER TABLE exposed.members DISABLE TRIGGER members_set_updated_at;
CREATE TABLE exposed.chambers (
    id SMALLINT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);
INSERT INTO exposed.chambers VALUES (1, 'Commons'), (2, 'Lords');
CREATE TABLE exposed.membership_kinds (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    chamber_id SMALLINT REFERENCES exposed.chambers (id)
);
INSERT INTO exposed.membership_kinds VALUES
    ('elected_mp', 'Elected MP', 1),
    ('life_peer', 'Life peer', 2),
    ('hereditary_peer', 'Hereditary peer', 2),
    ('excepted_hereditary_peer', 'Excepted hereditary peer', 2),
    ('hereditary_life_peer', 'Hereditary peer given life peerage', 2),
    ('bishop', 'Bishop', 2),
    ('judicial_peer', 'Judicial peer', 2),
    ('other_lords', 'Other Lords membership', 2),
    ('unknown', 'Unknown membership', NULL);
ALTER TABLE exposed.members
    RENAME COLUMN latest_membership_from TO membership_from;
ALTER TABLE exposed.members
    ADD COLUMN membership_kind TEXT REFERENCES exposed.membership_kinds (id),
    ADD COLUMN membership_start_date TEXT,
    ADD COLUMN membership_end_date TEXT,
    ADD COLUMN membership_status TEXT,
    ADD COLUMN membership_status_provenance TEXT NOT NULL DEFAULT 'source',
    ADD COLUMN membership_status_active BOOLEAN,
    ADD COLUMN source_captured_at TIMESTAMPTZ,
    ADD COLUMN source_capture_key UUID,
    ADD COLUMN commons_imported_at TIMESTAMPTZ,
    ALTER COLUMN party_id DROP NOT NULL,
    ALTER COLUMN party_name DROP NOT NULL,
    ALTER COLUMN membership_from DROP NOT NULL;
UPDATE exposed.members SET
    membership_kind = CASE latest_house WHEN 1 THEN 'elected_mp' ELSE
        CASE lower(trim(membership_from))
            WHEN 'life peer' THEN 'life_peer'
            WHEN 'hereditary' THEN 'hereditary_peer'
            WHEN 'hereditary peer' THEN 'hereditary_peer'
            WHEN 'bishops' THEN 'bishop'
            WHEN 'bishop' THEN 'bishop'
            WHEN 'life peer (judicial)' THEN 'judicial_peer'
            WHEN 'judicial peer' THEN 'judicial_peer'
            WHEN 'hereds given lps' THEN 'hereditary_life_peer'
            WHEN 'hereditary peer given life peerage' THEN 'hereditary_life_peer'
            WHEN 'excepted hereditary' THEN 'excepted_hereditary_peer'
            WHEN 'excepted hereditary peer' THEN 'excepted_hereditary_peer'
            ELSE 'other_lords'
        END END,
    membership_status_provenance = CASE WHEN latest_house = 1 THEN CASE WHEN is_current_commons THEN 'legacy_current' ELSE 'legacy_former' END ELSE 'source' END,
    membership_status = CASE WHEN latest_house = 1 THEN
        CASE WHEN is_current_commons THEN 'Legacy Commons current classification' ELSE 'Legacy Commons former classification' END END,
    commons_imported_at = created_at;
ALTER TABLE exposed.members
    ALTER COLUMN membership_kind SET NOT NULL,
    DROP COLUMN latest_house,
    DROP COLUMN is_current_commons,
    ADD CONSTRAINT members_source_clock CHECK ((source_captured_at IS NULL) = (source_capture_key IS NULL));
CREATE TABLE exposed.lords_imports (
    ingestion_key UUID PRIMARY KEY,
    capture_digest TEXT NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL,
    member_count BIGINT NOT NULL CHECK (member_count >= 0),
    loaded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE exposed.simple_declaration_registers (
    member_id UUID PRIMARY KEY REFERENCES exposed.members (id) ON DELETE CASCADE,
    source_document JSONB NOT NULL,
    empty_categories JSONB NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL,
    ingestion_key UUID NOT NULL REFERENCES exposed.lords_imports (ingestion_key)
);
CREATE TABLE exposed.simple_declarations (
    member_id UUID NOT NULL REFERENCES exposed.simple_declaration_registers (member_id) ON DELETE CASCADE,
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
    PRIMARY KEY (member_id, category_position, node_position),
    FOREIGN KEY (member_id, category_position, parent_position)
        REFERENCES exposed.simple_declarations (member_id, category_position, node_position),
    UNIQUE NULLS NOT DISTINCT (member_id, category_position, parent_position, sibling_position)
);

ALTER TABLE exposed.members ENABLE TRIGGER members_set_updated_at;
