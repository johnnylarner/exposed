CREATE TABLE exposed.declaration_sync_runs (
    ingestion_key uuid PRIMARY KEY,
    fingerprint text NOT NULL,
    content_sha256 text NOT NULL,
    revision bigint NOT NULL UNIQUE CHECK (revision > 0),
    manifest_path text NOT NULL,
    manifest_sha256 text NOT NULL,
    coverage_date date NOT NULL,
    committed_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE exposed.declaration_sync_head (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    ingestion_key uuid NOT NULL REFERENCES exposed.declaration_sync_runs(ingestion_key)
);
