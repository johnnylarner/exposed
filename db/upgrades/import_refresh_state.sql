-- Add application refresh state without changing imported records.
-- Run only when imports/migrations are stopped. Verify against the baseline
-- before adopting its checksum (db/README.md).
BEGIN;
CREATE TABLE IF NOT EXISTS exposed.import_refresh_state (
    term_start TIMESTAMPTZ PRIMARY KEY,
    last_completed TIMESTAMPTZ,
    last_attempt TIMESTAMPTZ,
    next_attempt TIMESTAMPTZ,
    outcome TEXT CHECK (outcome IN ('succeeded', 'warnings', 'failed')),
    rejected INTEGER NOT NULL DEFAULT 0 CHECK (rejected >= 0),
    failures INTEGER NOT NULL DEFAULT 0 CHECK (failures >= 0),
    new_members INTEGER[] NOT NULL DEFAULT '{}'
);
CREATE TABLE IF NOT EXISTS exposed.import_notifications (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    event_key TEXT NOT NULL UNIQUE,
    message TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    next_attempt TIMESTAMPTZ NOT NULL DEFAULT now(),
    delivered_at TIMESTAMPTZ
);
COMMIT;
