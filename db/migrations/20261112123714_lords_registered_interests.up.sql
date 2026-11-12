CREATE TABLE exposed.commons_member_profiles (
    member_id UUID PRIMARY KEY REFERENCES exposed.members (id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    party_id INTEGER NOT NULL,
    party_name TEXT NOT NULL,
    latest_house SMALLINT NOT NULL CHECK (latest_house IN (1, 2)),
    latest_membership_from TEXT NOT NULL,
    is_current_commons BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (NOT is_current_commons OR latest_house = 1)
);
INSERT INTO exposed.commons_member_profiles (member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons, created_at, updated_at)
SELECT id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons, created_at, updated_at FROM exposed.members;
ALTER TABLE exposed.declarations ADD CONSTRAINT declarations_commons_profile_fkey
    FOREIGN KEY (member_id) REFERENCES exposed.commons_member_profiles (member_id);
DROP TRIGGER members_set_updated_at ON exposed.members;
DROP INDEX exposed.members_name_search_trigram_idx;
DROP INDEX exposed.members_person_name_codes;
ALTER TABLE exposed.members
    DROP COLUMN name, DROP COLUMN party_id, DROP COLUMN party_name,
    DROP COLUMN latest_house, DROP COLUMN latest_membership_from,
    DROP COLUMN is_current_commons, DROP COLUMN updated_at;
CREATE INDEX members_name_search_trigram_idx
    ON exposed.commons_member_profiles USING gist (name exposed.gist_trgm_ops);
CREATE INDEX members_person_name_codes
    ON exposed.commons_member_profiles USING gin (exposed.person_name_codes(name));

CREATE TRIGGER commons_member_profiles_set_updated_at
    BEFORE UPDATE ON exposed.commons_member_profiles
    FOR EACH ROW WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();

CREATE TABLE exposed.lords_imports (
    ingestion_key UUID PRIMARY KEY,
    capture_digest TEXT NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL,
    member_count BIGINT NOT NULL CHECK (member_count >= 0),
    loaded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE exposed.lords_member_profiles (
    member_id UUID PRIMARY KEY REFERENCES exposed.members (id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    party_name TEXT,
    membership_from TEXT,
    membership_status TEXT,
    membership_end_date TEXT,
    source_document JSONB NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL,
    ingestion_key UUID NOT NULL REFERENCES exposed.lords_imports (ingestion_key)
);
CREATE INDEX lords_members_name_trgm_idx
    ON exposed.lords_member_profiles USING gin (name exposed.gin_trgm_ops);
CREATE TABLE exposed.lords_categories (
    member_id UUID NOT NULL REFERENCES exposed.lords_member_profiles (member_id) ON DELETE CASCADE,
    category_position BIGINT NOT NULL CHECK (category_position >= 0),
    source_id BIGINT NOT NULL CHECK (source_id BETWEEN 1 AND 4294967295),
    name TEXT,
    sort_order INTEGER NOT NULL,
    PRIMARY KEY (member_id, category_position)
);
CREATE TABLE exposed.lords_interests (
    member_id UUID NOT NULL,
    category_position BIGINT NOT NULL CHECK (category_position >= 0),
    node_position BIGINT NOT NULL CHECK (node_position >= 0),
    parent_position BIGINT CHECK (parent_position >= 0 AND parent_position < node_position),
    sibling_position BIGINT NOT NULL CHECK (sibling_position >= 0),
    source_id BIGINT NOT NULL CHECK (source_id BETWEEN 1 AND 4294967295),
    interest_text TEXT,
    created_when TEXT,
    last_amended_when TEXT,
    deleted_when TEXT,
    is_correction BOOLEAN NOT NULL,
    PRIMARY KEY (member_id, category_position, node_position),
    FOREIGN KEY (member_id, category_position)
        REFERENCES exposed.lords_categories (member_id, category_position) ON DELETE CASCADE,
    FOREIGN KEY (member_id, category_position, parent_position)
        REFERENCES exposed.lords_interests (member_id, category_position, node_position),
    UNIQUE NULLS NOT DISTINCT (member_id, category_position, parent_position, sibling_position)
);
