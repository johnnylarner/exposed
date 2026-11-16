DROP TABLE exposed.simple_declarations;
ALTER TABLE exposed.members
    ADD COLUMN latest_house SMALLINT,
    ADD COLUMN is_current_commons BOOLEAN;
ALTER TABLE exposed.members DISABLE TRIGGER members_set_updated_at;
UPDATE exposed.members m SET
    latest_house = CASE k.chamber_id
        WHEN '019a7a90-0000-7000-8000-000000000001'::uuid THEN 1
        WHEN '019a7a90-0000-7000-8000-000000000002'::uuid THEN 2
    END,
    is_current_commons = k.chamber_id = '019a7a90-0000-7000-8000-000000000001' AND m.membership_end_date IS NULL
FROM exposed.membership_kinds k WHERE k.id = m.membership_kind;
ALTER TABLE exposed.members ENABLE TRIGGER members_set_updated_at;
ALTER TABLE exposed.members
    DROP COLUMN membership_kind,
    DROP COLUMN membership_start_date,
    DROP COLUMN membership_end_date,
    DROP COLUMN membership_status,
    ALTER COLUMN latest_house SET NOT NULL,
    ALTER COLUMN is_current_commons SET NOT NULL,
    ADD CONSTRAINT members_latest_house_check CHECK (latest_house IN (1, 2)),
    ADD CONSTRAINT members_check CHECK (NOT is_current_commons OR latest_house = 1);
ALTER TABLE exposed.members RENAME COLUMN membership_from TO latest_membership_from;
DROP TABLE exposed.membership_kinds;
DROP TABLE exposed.chambers;
