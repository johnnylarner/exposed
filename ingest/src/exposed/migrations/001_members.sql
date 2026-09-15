CREATE TABLE exposed.parliament_terms (
    id uuid PRIMARY KEY,
    term_start date NOT NULL UNIQUE
);

CREATE TABLE exposed.import_runs (
    id uuid PRIMARY KEY,
    term_start date NOT NULL,
    as_of date NOT NULL,
    started_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    completed_at timestamptz,
    status text NOT NULL CHECK (status IN ('running', 'succeeded', 'failed')),
    summary jsonb,
    error text,
    CHECK ((status = 'running') = (completed_at IS NULL))
);

CREATE TABLE exposed.members (
    id uuid PRIMARY KEY,
    parliament_member_id integer NOT NULL UNIQUE CHECK (parliament_member_id > 0),
    name text NOT NULL CHECK (length(trim(name)) > 0),
    party_id integer,
    party_name text,
    latest_house smallint NOT NULL CHECK (latest_house IN (1, 2)),
    latest_membership_from text,
    latest_membership_from_id integer,
    is_current_commons boolean NOT NULL,
    first_seen_run_id uuid NOT NULL REFERENCES exposed.import_runs(id),
    last_seen_run_id uuid NOT NULL REFERENCES exposed.import_runs(id),
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
    last_seen_run_id uuid NOT NULL REFERENCES exposed.import_runs(id),
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

CREATE TABLE exposed.api_responses (
    id uuid PRIMARY KEY,
    run_id uuid NOT NULL REFERENCES exposed.import_runs(id),
    sequence integer NOT NULL CHECK (sequence > 0),
    url text NOT NULL,
    retrieved_at timestamptz NOT NULL,
    payload jsonb NOT NULL,
    UNIQUE (run_id, sequence)
);
