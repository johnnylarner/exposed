DROP TABLE exposed.lords_interests;
DROP TABLE exposed.lords_categories;
DROP TABLE exposed.lords_member_profiles;
DROP TABLE exposed.lords_imports;
ALTER TABLE exposed.declarations DROP CONSTRAINT declarations_commons_profile_fkey;
DELETE FROM exposed.members m
WHERE NOT EXISTS (SELECT 1 FROM exposed.commons_member_profiles p WHERE p.member_id = m.id);
ALTER TABLE exposed.members
    ADD COLUMN name TEXT,
    ADD COLUMN party_id INTEGER,
    ADD COLUMN party_name TEXT,
    ADD COLUMN latest_house SMALLINT,
    ADD COLUMN latest_membership_from TEXT,
    ADD COLUMN is_current_commons BOOLEAN,
    ADD COLUMN updated_at TIMESTAMPTZ DEFAULT now();
UPDATE exposed.members m SET
    name = p.name,
    party_id = p.party_id,
    party_name = p.party_name,
    latest_house = p.latest_house,
    latest_membership_from = p.latest_membership_from,
    is_current_commons = p.is_current_commons,
    created_at = p.created_at,
    updated_at = p.updated_at
FROM exposed.commons_member_profiles p WHERE p.member_id = m.id;
DROP TABLE exposed.commons_member_profiles;
ALTER TABLE exposed.members
    ALTER COLUMN name SET NOT NULL,
    ALTER COLUMN party_id SET NOT NULL,
    ALTER COLUMN party_name SET NOT NULL,
    ALTER COLUMN latest_house SET NOT NULL,
    ALTER COLUMN latest_membership_from SET NOT NULL,
    ALTER COLUMN is_current_commons SET NOT NULL,
    ALTER COLUMN updated_at SET NOT NULL,
    ADD CONSTRAINT members_name_check CHECK (length(trim(name)) > 0),
    ADD CONSTRAINT members_latest_house_check CHECK (latest_house IN (1, 2)),
    ADD CONSTRAINT members_check CHECK (NOT is_current_commons OR latest_house = 1);
CREATE INDEX members_name_search_trigram_idx
    ON exposed.members USING gist (name exposed.gist_trgm_ops);
CREATE INDEX members_person_name_codes
    ON exposed.members USING gin (exposed.person_name_codes(name));

CREATE TRIGGER members_set_updated_at
    BEFORE UPDATE ON exposed.members
    FOR EACH ROW WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();
