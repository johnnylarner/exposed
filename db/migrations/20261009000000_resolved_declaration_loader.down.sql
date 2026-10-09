DROP TABLE exposed.declaration_load_runs;
DROP TABLE exposed.funder_aliases;

ALTER TABLE exposed.funding_entries
    DROP COLUMN attribution_issues,
    DROP COLUMN unavailable_reason,
    DROP COLUMN selected_parent_declaration_id,
    DROP COLUMN attribution_basis,
    DROP COLUMN selected_observation_id,
    DROP COLUMN source_funding_entry_id;

ALTER TABLE exposed.funders
    DROP COLUMN resolution_identity_id,
    ADD CONSTRAINT funders_funder_name_key UNIQUE (funder_name);
