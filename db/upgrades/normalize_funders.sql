-- Run after inspecting the existing schema; see db/README.md.
-- Supports the previous baseline and the feature branch's already-normalized schema.
-- SQLx history is deliberately left untouched until schema/data verification.
BEGIN;

LOCK TABLE exposed.funding_entries IN ACCESS EXCLUSIVE MODE;

DO $$
BEGIN
    IF to_regclass('exposed.funders') IS NULL THEN
        -- A shared name cannot retain contradictory source identities. Resolve
        -- these explicitly instead of silently discarding previously imported data.
        IF EXISTS (
            SELECT funder FROM exposed.funding_entries
            GROUP BY funder
            HAVING count(DISTINCT donor_status) > 1
                OR count(DISTINCT company_number) > 1
        ) OR EXISTS (
            SELECT 1 FROM exposed.funding_entries
            WHERE funder IS NULL AND (donor_status IS NOT NULL OR company_number IS NOT NULL)
        ) THEN
            RAISE EXCEPTION 'Resolve conflicting or unnamed funder metadata before upgrading';
        END IF;

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

        INSERT INTO exposed.funders (funder_name, funder_kind, company_number)
        SELECT funder, max(donor_status), max(company_number)
        FROM exposed.funding_entries
        WHERE funder IS NOT NULL
        GROUP BY funder;

        ALTER TABLE exposed.funding_entries
            ADD COLUMN funder_id UUID REFERENCES exposed.funders (id);
        -- Relinking storage does not change the source payment or its audit times.
        ALTER TABLE exposed.funding_entries DISABLE TRIGGER funding_entries_set_updated_at;
        UPDATE exposed.funding_entries fe
        SET funder_id = f.id
        FROM exposed.funders f
        WHERE f.funder_name = fe.funder;
        ALTER TABLE exposed.funding_entries ENABLE TRIGGER funding_entries_set_updated_at;
        ALTER TABLE exposed.funding_entries
            DROP COLUMN funder,
            DROP COLUMN donor_status,
            DROP COLUMN company_number;
    END IF;
END;
$$;

ALTER TABLE exposed.funding_entries
    ALTER COLUMN funder_id DROP NOT NULL,
    ALTER COLUMN currency DROP NOT NULL,
    ALTER COLUMN payment_type DROP NOT NULL;

CREATE INDEX IF NOT EXISTS funding_entries_funder_idx
    ON exposed.funding_entries (funder_id);

CREATE OR REPLACE TRIGGER funders_set_updated_at
    BEFORE UPDATE ON exposed.funders
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();

COMMENT ON COLUMN exposed.funders.funder_kind IS
    'Explicit Parliament DonorStatus for the attributed donor; NULL when unavailable';
COMMENT ON COLUMN exposed.funders.company_number IS
    'Parliament DonorCompanyIdentifier when DonorStatus is Company; text preserves leading zeros';
COMMENT ON COLUMN exposed.funding_entries.funder_id IS
    'Shared exact-name funder; NULL when the source does not identify a funder';

COMMIT;
