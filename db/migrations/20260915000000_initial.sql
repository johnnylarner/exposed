-- migrate:up
CREATE SCHEMA IF NOT EXISTS exposed;

CREATE TABLE exposed.parliament_terms (
    id uuid PRIMARY KEY,
    term_start date NOT NULL UNIQUE,
    term_end date,
    CONSTRAINT parliament_terms_valid_dates CHECK (term_end IS NULL OR term_end >= term_start)
);

CREATE TABLE exposed.members (
    id uuid PRIMARY KEY,
    parliament_member_id integer NOT NULL UNIQUE CHECK (parliament_member_id > 0),
    name text NOT NULL CHECK (length(trim(name)) > 0),
    party_id integer,
    party_name text,
    latest_house smallint NOT NULL CHECK (latest_house IN (1, 2)),
    latest_membership_from text,
    is_current_commons boolean NOT NULL,
    CHECK (NOT is_current_commons OR latest_house = 1)
);

CREATE TABLE exposed.member_terms (
    id uuid PRIMARY KEY,
    member_id uuid NOT NULL REFERENCES exposed.members(id),
    term_id uuid NOT NULL REFERENCES exposed.parliament_terms(id),
    house smallint NOT NULL CHECK (house IN (1, 2)),
    source_start_date date NOT NULL,
    source_end_date date,
    served_from date NOT NULL,
    served_until date,
    UNIQUE (member_id, term_id, house, source_start_date),
    CHECK (served_from >= source_start_date),
    CHECK (served_until IS NULL OR served_until >= served_from),
    CHECK (served_until IS NOT DISTINCT FROM source_end_date)
);
CREATE INDEX member_terms_term_idx ON exposed.member_terms(term_id);

CREATE FUNCTION exposed.check_term_service_start() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.served_from IS DISTINCT FROM (
        SELECT greatest(NEW.source_start_date, term_start)
        FROM exposed.parliament_terms WHERE id = NEW.term_id
    ) THEN
        RAISE EXCEPTION 'Service start must be the later of source start and term start'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER member_terms_check_start
BEFORE INSERT OR UPDATE ON exposed.member_terms
FOR EACH ROW EXECUTE FUNCTION exposed.check_term_service_start();

CREATE TABLE exposed.declarations (
    id uuid PRIMARY KEY,
    source_declaration_id integer NOT NULL UNIQUE CHECK (source_declaration_id > 0),
    member_id uuid NOT NULL REFERENCES exposed.members(id),
    category_id integer NOT NULL CHECK (category_id > 0),
    category_name text NOT NULL CHECK (length(trim(category_name)) > 0),
    registration_date date,
    fetched_at timestamptz NOT NULL
);
CREATE INDEX declarations_member_idx ON exposed.declarations(member_id);

COMMENT ON COLUMN exposed.declarations.registration_date IS
    'Parliament registrationDate from the latest selected register version; NULL when unavailable';

CREATE TABLE exposed.funding_entries (
    id uuid PRIMARY KEY,
    source_declaration_id integer NOT NULL REFERENCES exposed.declarations(source_declaration_id),
    funder text,
    amount numeric,
    currency text,
    payment_type text,
    donor_status text,
    company_number text,
    CONSTRAINT funding_entries_company_number_status
        CHECK (company_number IS NULL OR donor_status IS NOT DISTINCT FROM 'Company')
);
CREATE INDEX funding_entries_declaration_idx ON exposed.funding_entries(source_declaration_id);

COMMENT ON COLUMN exposed.funding_entries.donor_status IS
    'Explicit Parliament DonorStatus for the attributed donor; NULL when unavailable';
COMMENT ON COLUMN exposed.funding_entries.company_number IS
    'Parliament DonorCompanyIdentifier when DonorStatus is Company; text preserves leading zeros';

-- migrate:down
DROP TABLE exposed.funding_entries;
DROP TABLE exposed.declarations;
DROP TABLE exposed.member_terms;
DROP FUNCTION exposed.check_term_service_start();
DROP TABLE exposed.members;
DROP TABLE exposed.parliament_terms;
