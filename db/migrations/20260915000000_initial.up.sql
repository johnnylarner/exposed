-- Date-only source values are stored at midnight UTC.
CREATE SCHEMA IF NOT EXISTS exposed;
CREATE EXTENSION pg_trgm WITH SCHEMA exposed;

CREATE TABLE exposed.parliament_terms (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    term_start TIMESTAMPTZ NOT NULL UNIQUE,
    term_end TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT parliament_terms_valid_dates
        CHECK (term_end IS NULL OR term_end >= term_start)
);

CREATE TABLE exposed.members (
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

CREATE TABLE exposed.member_terms (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    member_id UUID NOT NULL REFERENCES exposed.members (id),
    term_id UUID NOT NULL REFERENCES exposed.parliament_terms (id),
    house SMALLINT NOT NULL CHECK (house IN (1, 2)),
    source_start_date TIMESTAMPTZ NOT NULL,
    source_end_date TIMESTAMPTZ,
    served_from TIMESTAMPTZ NOT NULL,
    served_until TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (member_id, term_id, house, source_start_date),
    CHECK (served_from >= source_start_date),
    CHECK (served_until IS NULL OR served_until >= served_from),
    CHECK (served_until IS NOT DISTINCT FROM source_end_date)
);

CREATE INDEX member_terms_term_idx
    ON exposed.member_terms (term_id);

CREATE FUNCTION exposed.check_term_service_start()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.served_from IS DISTINCT FROM (
        SELECT greatest(NEW.source_start_date, term_start)
        FROM exposed.parliament_terms
        WHERE id = NEW.term_id
    ) THEN
        RAISE EXCEPTION 'Service start must be the later of source start and term start'
            USING ERRCODE = '23514';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER member_terms_check_start
    BEFORE INSERT OR UPDATE ON exposed.member_terms
    FOR EACH ROW
    EXECUTE FUNCTION exposed.check_term_service_start();

CREATE TABLE exposed.declarations (
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

CREATE INDEX declarations_member_idx
    ON exposed.declarations (member_id);

COMMENT ON COLUMN exposed.declarations.registration_date IS
    'Parliament registrationDate at midnight UTC from the latest selected register version; NULL when unavailable';

CREATE TABLE exposed.funding_entries (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    source_declaration_id INTEGER NOT NULL REFERENCES exposed.declarations (source_declaration_id),
    funder TEXT NOT NULL,
    amount NUMERIC,
    currency TEXT NOT NULL,
    payment_type TEXT NOT NULL,
    donor_status TEXT,
    company_number TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT funding_entries_company_number_status
        CHECK (company_number IS NULL OR donor_status IS NOT DISTINCT FROM 'Company')
);

CREATE INDEX funding_entries_declaration_idx
    ON exposed.funding_entries (source_declaration_id);

COMMENT ON COLUMN exposed.funding_entries.donor_status IS
    'Explicit Parliament DonorStatus for the attributed donor; NULL when unavailable';
COMMENT ON COLUMN exposed.funding_entries.company_number IS
    'Parliament DonorCompanyIdentifier when DonorStatus is Company; text preserves leading zeros';

CREATE FUNCTION exposed.set_updated_at()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at := now();

    RETURN NEW;
END;
$$;

CREATE TRIGGER parliament_terms_set_updated_at
    BEFORE UPDATE ON exposed.parliament_terms
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();

CREATE TRIGGER members_set_updated_at
    BEFORE UPDATE ON exposed.members
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();

CREATE TRIGGER member_terms_set_updated_at
    BEFORE UPDATE ON exposed.member_terms
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();

CREATE TRIGGER declarations_set_updated_at
    BEFORE UPDATE ON exposed.declarations
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();

CREATE TRIGGER funding_entries_set_updated_at
    BEFORE UPDATE ON exposed.funding_entries
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION exposed.set_updated_at();
