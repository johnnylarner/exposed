-- migrate:up
ALTER TABLE exposed.members 
    ALTER COLUMN party_id SET NOT NULL,
    ALTER COLUMN party_name SET NOT NULL,
    ALTER COLUMN latest_membership_from SET NOT NULL;


-- migrate:down
ALTER TABLE exposed.members 
    ALTER COLUMN party_id DROP NOT NULL,
    ALTER COLUMN party_name DROP NOT NULL,
    ALTER COLUMN latest_membership_from DROP NOT NULL;
