ALTER TABLE exposed.parliament_terms
    ADD COLUMN term_end date,
    ADD CONSTRAINT parliament_terms_valid_dates
        CHECK (term_end IS NULL OR term_end >= term_start);

ALTER TABLE exposed.members
    DROP COLUMN first_seen_run_id,
    DROP COLUMN last_seen_run_id;
ALTER TABLE exposed.member_terms DROP COLUMN last_seen_run_id;
DROP TABLE exposed.api_responses;
DROP TABLE exposed.import_runs;
