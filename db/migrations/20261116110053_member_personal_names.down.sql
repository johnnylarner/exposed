ALTER TABLE exposed.members DROP CONSTRAINT members_complete_personal_name;
UPDATE exposed.members SET name = COALESCE(parliamentary_name, name);
ALTER TABLE exposed.members
    DROP COLUMN personal_first_name,
    DROP COLUMN personal_last_name,
    DROP COLUMN parliamentary_name,
    DROP COLUMN formal_name;
