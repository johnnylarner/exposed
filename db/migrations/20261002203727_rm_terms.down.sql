-- Add down migration script here

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

