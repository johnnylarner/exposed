CREATE TABLE exposed.lords_imports (
    ingestion_key UUID PRIMARY KEY,
    capture_digest TEXT NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL,
    member_count BIGINT NOT NULL CHECK (member_count >= 0),
    loaded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE exposed.lords_members (
    parliament_member_id INTEGER PRIMARY KEY CHECK (parliament_member_id > 0),
    name TEXT NOT NULL,
    source_document JSONB NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL,
    ingestion_key UUID NOT NULL REFERENCES exposed.lords_imports (ingestion_key)
);

CREATE INDEX lords_members_name_trgm_idx
    ON exposed.lords_members USING gin (name exposed.gin_trgm_ops);
