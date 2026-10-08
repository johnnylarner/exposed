BEGIN;

ALTER TABLE exposed.funders
    DROP CONSTRAINT IF EXISTS funders_funder_name_key,
    ADD COLUMN IF NOT EXISTS resolution_identity_id TEXT;

DROP INDEX IF EXISTS exposed.funders_funder_name_key;
CREATE UNIQUE INDEX IF NOT EXISTS funders_resolution_identity_idx
    ON exposed.funders (resolution_identity_id);

ALTER TABLE exposed.funding_entries
    ADD COLUMN IF NOT EXISTS source_funding_entry_id TEXT,
    ADD COLUMN IF NOT EXISTS selected_observation_id TEXT,
    ADD COLUMN IF NOT EXISTS attribution_basis TEXT,
    ADD COLUMN IF NOT EXISTS selected_parent_declaration_id INTEGER,
    ADD COLUMN IF NOT EXISTS unavailable_reason TEXT,
    ADD COLUMN IF NOT EXISTS attribution_issues JSONB NOT NULL DEFAULT '[]'::JSONB;

CREATE UNIQUE INDEX IF NOT EXISTS funding_entries_source_occurrence_idx
    ON exposed.funding_entries (source_funding_entry_id);

CREATE TABLE IF NOT EXISTS exposed.funder_aliases (
    funder_id UUID NOT NULL REFERENCES exposed.funders (id),
    funder_alias TEXT NOT NULL CHECK (length(trim(funder_alias)) > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (funder_id, funder_alias)
);

CREATE OR REPLACE TRIGGER funder_aliases_set_updated_at
    BEFORE UPDATE ON exposed.funder_aliases
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();

CREATE TABLE IF NOT EXISTS exposed.declaration_load_runs (
    ingestion_key UUID PRIMARY KEY,
    fingerprint TEXT NOT NULL,
    declarations BIGINT NOT NULL CHECK (declarations >= 0),
    funders BIGINT NOT NULL CHECK (funders >= 0),
    funding_entries BIGINT NOT NULL CHECK (funding_entries >= 0),
    loaded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMIT;
