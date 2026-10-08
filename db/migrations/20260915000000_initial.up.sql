-- Date-only source values are stored at midnight UTC.
CREATE SCHEMA IF NOT EXISTS exposed;
CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA exposed;

CREATE TABLE IF NOT EXISTS exposed.members (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    parliament_member_id INTEGER NOT NULL UNIQUE CHECK (parliament_member_id > 0),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    party_id INTEGER NOT NULL,
    party_name TEXT NOT NULL,
    latest_house SMALLINT NOT NULL CHECK (latest_house IN (1, 2)),
    latest_membership_from TEXT NOT NULL,
    is_current_commons BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (NOT is_current_commons OR latest_house = 1)
);

CREATE TABLE IF NOT EXISTS exposed.declarations (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    source_declaration_id INTEGER NOT NULL UNIQUE CHECK (source_declaration_id > 0),
    member_id UUID NOT NULL REFERENCES exposed.members (id),
    category_id INTEGER NOT NULL CHECK (category_id > 0),
    category_name TEXT NOT NULL CHECK (length(trim(category_name)) > 0),
    fetched_at TIMESTAMPTZ NOT NULL,
    registration_date TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS declarations_member_idx
    ON exposed.declarations (member_id);

COMMENT ON COLUMN exposed.declarations.registration_date IS
    'Parliament registrationDate at midnight UTC from the latest selected register version; NULL when unavailable';

CREATE TABLE IF NOT EXISTS exposed.funders (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    funder_name TEXT NOT NULL,
    funder_kind TEXT,
    company_number TEXT,
    resolution_identity_id TEXT UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT funders_company_number_status
        CHECK (company_number IS NULL OR funder_kind IS NOT DISTINCT FROM 'Company')
);

CREATE TABLE IF NOT EXISTS exposed.funder_aliases (
    funder_id UUID NOT NULL REFERENCES exposed.funders (id),
    funder_alias TEXT NOT NULL CHECK (length(trim(funder_alias)) > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (funder_id, funder_alias)
);

COMMENT ON COLUMN exposed.funders.funder_kind IS
    'Explicit Parliament DonorStatus for the attributed donor; NULL when unavailable';
COMMENT ON COLUMN exposed.funders.company_number IS
    'Parliament DonorCompanyIdentifier when DonorStatus is Company; text preserves leading zeros';

CREATE TABLE IF NOT EXISTS exposed.funding_entries (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    source_declaration_id INTEGER NOT NULL REFERENCES exposed.declarations (source_declaration_id),
    funder_id UUID REFERENCES exposed.funders (id),
    amount NUMERIC,
    currency TEXT,
    payment_type TEXT,
    source_funding_entry_id TEXT UNIQUE,
    selected_observation_id TEXT,
    attribution_basis TEXT,
    selected_parent_declaration_id INTEGER,
    unavailable_reason TEXT,
    attribution_issues JSONB NOT NULL DEFAULT '[]'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS funding_entries_declaration_idx
    ON exposed.funding_entries (source_declaration_id);
CREATE INDEX IF NOT EXISTS funding_entries_funder_idx
    ON exposed.funding_entries (funder_id);

COMMENT ON COLUMN exposed.funding_entries.funder_id IS
    'Resolved identity selected by the reporting attribution policy; NULL when unavailable';

CREATE TABLE IF NOT EXISTS exposed.declaration_load_runs (
    ingestion_key UUID PRIMARY KEY,
    fingerprint TEXT NOT NULL,
    declarations BIGINT NOT NULL CHECK (declarations >= 0),
    funders BIGINT NOT NULL CHECK (funders >= 0),
    funding_entries BIGINT NOT NULL CHECK (funding_entries >= 0),
    loaded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE OR REPLACE FUNCTION exposed.set_updated_at()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at := now();

    RETURN NEW;
END;
$$;

CREATE OR REPLACE TRIGGER members_set_updated_at
    BEFORE UPDATE ON exposed.members
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();

CREATE OR REPLACE TRIGGER declarations_set_updated_at
    BEFORE UPDATE ON exposed.declarations
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();

CREATE OR REPLACE TRIGGER funding_entries_set_updated_at
    BEFORE UPDATE ON exposed.funding_entries
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();

CREATE OR REPLACE TRIGGER funders_set_updated_at
    BEFORE UPDATE ON exposed.funders
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();

CREATE OR REPLACE TRIGGER funder_aliases_set_updated_at
    BEFORE UPDATE ON exposed.funder_aliases
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();
