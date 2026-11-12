CREATE TYPE exposed.profile_chamber AS ENUM ('commons', 'lords');
CREATE TABLE exposed.lords_imports (
    ingestion_key UUID PRIMARY KEY,
    capture_digest TEXT NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL,
    member_count BIGINT NOT NULL CHECK (member_count >= 0),
    loaded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE exposed.member_profiles (
    member_id UUID NOT NULL REFERENCES exposed.members (id) ON DELETE CASCADE,
    chamber exposed.profile_chamber NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    party_id INTEGER,
    party_name TEXT,
    latest_house SMALLINT CHECK (latest_house IN (1, 2)),
    latest_membership_from TEXT,
    is_current_commons BOOLEAN,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ,
    membership_from TEXT,
    membership_status TEXT,
    membership_end_date TEXT,
    source_document JSONB,
    captured_at TIMESTAMPTZ,
    ingestion_key UUID REFERENCES exposed.lords_imports (ingestion_key),
    PRIMARY KEY (member_id, chamber),
    CONSTRAINT member_profiles_source_shape CHECK (
        (chamber = 'commons'
            AND party_id IS NOT NULL AND party_name IS NOT NULL
            AND latest_house IS NOT NULL AND latest_membership_from IS NOT NULL
            AND is_current_commons IS NOT NULL
            AND created_at IS NOT NULL AND updated_at IS NOT NULL
            AND (NOT is_current_commons OR latest_house = 1)
            AND membership_from IS NULL AND membership_status IS NULL
            AND membership_end_date IS NULL AND source_document IS NULL
            AND captured_at IS NULL AND ingestion_key IS NULL)
        OR
        (chamber = 'lords'
            AND party_id IS NULL AND latest_house IS NULL
            AND latest_membership_from IS NULL AND is_current_commons IS NULL
            AND created_at IS NULL AND updated_at IS NULL
            AND source_document IS NOT NULL AND captured_at IS NOT NULL
            AND ingestion_key IS NOT NULL)
    )
);
INSERT INTO exposed.member_profiles (member_id, chamber, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons, created_at, updated_at)
SELECT id, 'commons', name, party_id, party_name, latest_house, latest_membership_from, is_current_commons, created_at, updated_at FROM exposed.members;
ALTER TABLE exposed.declarations
    ADD COLUMN profile_chamber exposed.profile_chamber
        GENERATED ALWAYS AS ('commons'::exposed.profile_chamber) STORED NOT NULL,
    ADD CONSTRAINT declarations_commons_profile_fkey
        FOREIGN KEY (member_id, profile_chamber) REFERENCES exposed.member_profiles (member_id, chamber);
DROP TRIGGER members_set_updated_at ON exposed.members;
DROP INDEX exposed.members_name_search_trigram_idx;
DROP INDEX exposed.members_person_name_codes;
ALTER TABLE exposed.members
    DROP COLUMN name, DROP COLUMN party_id, DROP COLUMN party_name,
    DROP COLUMN latest_house, DROP COLUMN latest_membership_from,
    DROP COLUMN is_current_commons, DROP COLUMN updated_at;
CREATE INDEX members_name_search_trigram_idx
    ON exposed.member_profiles USING gist (name exposed.gist_trgm_ops) WHERE chamber = 'commons';
CREATE INDEX members_person_name_codes
    ON exposed.member_profiles USING gin (exposed.person_name_codes(name)) WHERE chamber = 'commons';
CREATE INDEX lords_members_name_trgm_idx
    ON exposed.member_profiles USING gin (name exposed.gin_trgm_ops) WHERE chamber = 'lords';

CREATE TRIGGER member_profiles_set_updated_at
    BEFORE UPDATE ON exposed.member_profiles
    FOR EACH ROW WHEN (NEW.chamber = 'commons' AND OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();

CREATE TABLE exposed.lords_categories (
    member_id UUID NOT NULL,
    profile_chamber exposed.profile_chamber
        GENERATED ALWAYS AS ('lords'::exposed.profile_chamber) STORED NOT NULL,
    category_position BIGINT NOT NULL CHECK (category_position >= 0),
    source_id BIGINT NOT NULL CHECK (source_id BETWEEN 1 AND 4294967295),
    name TEXT,
    sort_order INTEGER NOT NULL,
    PRIMARY KEY (member_id, category_position),
    FOREIGN KEY (member_id, profile_chamber)
        REFERENCES exposed.member_profiles (member_id, chamber) ON DELETE CASCADE
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
