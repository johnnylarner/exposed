-- migrate:up
ALTER TABLE exposed.members DROP COLUMN latest_membership_from_id;

-- migrate:down
ALTER TABLE exposed.members ADD COLUMN latest_membership_from_id integer;
