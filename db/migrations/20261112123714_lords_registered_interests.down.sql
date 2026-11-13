DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM exposed.members m
        JOIN exposed.membership_kinds k ON k.id = m.membership_kind
        WHERE m.commons_imported_at IS NOT NULL
          AND (k.chamber_id IS NULL OR m.party_id IS NULL OR m.party_name IS NULL OR m.membership_from IS NULL)
    ) THEN
        RAISE EXCEPTION 'Cannot revert member membership migration: retained Commons members contain unknown chamber, party or membership fields that the old schema cannot represent';
    END IF;
    IF EXISTS (
        SELECT 1 FROM exposed.members m
        JOIN exposed.declarations d ON d.member_id = m.id
        WHERE m.commons_imported_at IS NULL
    ) THEN
        RAISE EXCEPTION 'Cannot revert member membership migration: a member outside the imported Commons cohort owns financial declarations';
    END IF;
END
$$;
ALTER TABLE exposed.members DISABLE TRIGGER members_set_updated_at;
DROP TABLE exposed.simple_declarations;
DROP TABLE exposed.simple_declaration_registers;
DROP TABLE exposed.lords_imports;
DELETE FROM exposed.members WHERE commons_imported_at IS NULL;
ALTER TABLE exposed.members
    ADD COLUMN latest_house SMALLINT,
    ADD COLUMN is_current_commons BOOLEAN;
UPDATE exposed.members m SET
    latest_house = k.chamber_id,
    is_current_commons = k.chamber_id = 1 AND m.membership_end_date IS NULL AND m.membership_status_provenance <> 'legacy_former'
FROM exposed.membership_kinds k WHERE k.id = m.membership_kind;
ALTER TABLE exposed.members
    DROP COLUMN membership_kind,
    DROP COLUMN membership_start_date,
    DROP COLUMN membership_end_date,
    DROP COLUMN membership_status,
    DROP COLUMN membership_status_provenance,
    DROP COLUMN membership_status_active,
    DROP COLUMN source_captured_at,
    DROP COLUMN source_capture_key,
    DROP COLUMN commons_imported_at,
    ALTER COLUMN latest_house SET NOT NULL,
    ALTER COLUMN is_current_commons SET NOT NULL,
    ALTER COLUMN party_id SET NOT NULL,
    ALTER COLUMN party_name SET NOT NULL,
    ADD CONSTRAINT members_latest_house_check CHECK (latest_house IN (1, 2)),
    ADD CONSTRAINT members_check CHECK (NOT is_current_commons OR latest_house = 1);
ALTER TABLE exposed.members RENAME COLUMN membership_from TO latest_membership_from;
ALTER TABLE exposed.members ALTER COLUMN latest_membership_from SET NOT NULL;
DROP TABLE exposed.membership_kinds;
DROP TABLE exposed.chambers;

ALTER TABLE exposed.members ENABLE TRIGGER members_set_updated_at;
