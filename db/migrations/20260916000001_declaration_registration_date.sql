-- migrate:up
ALTER TABLE exposed.declarations ADD COLUMN registration_date date;

COMMENT ON COLUMN exposed.declarations.registration_date IS
    'Parliament registrationDate from the latest selected register version; NULL when unavailable';

-- migrate:down
ALTER TABLE exposed.declarations DROP COLUMN registration_date;
