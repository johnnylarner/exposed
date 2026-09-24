-- Add up migration script here

BEGIN;

CREATE TABLE exposed.funders (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    funder_name TEXT NOT NULL UNIQUE,
    funder_kind TEXT,
    company_number TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT funders_company_number_status
        CHECK (company_number IS NULL OR funder_kind IS NOT DISTINCT FROM 'Company')
);

CREATE TRIGGER funders_set_updated_at
    BEFORE UPDATE ON exposed.funders
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();


ALTER TABLE exposed.funding_entries
    DROP COLUMN funder,
    DROP COLUMN donor_status,
    DROP COLUMN company_number,
    ADD funder_id UUID NOT NULL REFERENCES exposed.funders (id);


COMMIT;
