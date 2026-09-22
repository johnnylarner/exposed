-- migrate:up
ALTER TABLE exposed.funding_entries 
    ALTER COLUMN funder SET NOT NULL;
ALTER TABLE exposed.funding_entries 
    ALTER COLUMN currency SET NOT NULL;
ALTER TABLE exposed.funding_entries 
    ALTER COLUMN payment_type SET NOT NULL;

-- migrate:down
ALTER TABLE exposed.funding_entries 
    ALTER COLUMN funder DROP NOT NULL;
ALTER TABLE exposed.funding_entries 
    ALTER COLUMN currency DROP NOT NULL;
ALTER TABLE exposed.funding_entries 
    ALTER COLUMN payment_type DROP NOT NULL;

