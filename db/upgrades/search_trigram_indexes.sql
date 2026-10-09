BEGIN;

CREATE INDEX IF NOT EXISTS members_name_search_trigram_idx
    ON exposed.members USING GIST (name exposed.gist_trgm_ops);
CREATE INDEX IF NOT EXISTS funders_name_search_trigram_idx
    ON exposed.funders USING GIST (funder_name exposed.gist_trgm_ops);
CREATE INDEX IF NOT EXISTS funder_aliases_name_search_trigram_idx
    ON exposed.funder_aliases USING GIST (funder_alias exposed.gist_trgm_ops);

COMMIT;
