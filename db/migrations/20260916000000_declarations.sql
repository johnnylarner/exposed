-- migrate:up
CREATE TABLE exposed.declarations (
    id uuid PRIMARY KEY,
    source_declaration_id integer NOT NULL UNIQUE CHECK (source_declaration_id > 0),
    member_id uuid NOT NULL REFERENCES exposed.members(id),
    category_id integer NOT NULL CHECK (category_id > 0),
    category_name text NOT NULL CHECK (length(trim(category_name)) > 0),
    fetched_at timestamptz NOT NULL
);
CREATE INDEX declarations_member_idx ON exposed.declarations(member_id);

CREATE TABLE exposed.funding_entries (
    id uuid PRIMARY KEY,
    source_declaration_id integer NOT NULL REFERENCES exposed.declarations(source_declaration_id),
    funder text,
    amount numeric,
    currency text,
    payment_type text
);
CREATE INDEX funding_entries_declaration_idx ON exposed.funding_entries(source_declaration_id);

-- migrate:down
DROP TABLE exposed.funding_entries;
DROP TABLE exposed.declarations;
