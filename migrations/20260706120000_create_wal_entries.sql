-- Write-Ahead Log: records intent before applying DB writes.
CREATE TABLE IF NOT EXISTS wal_entries (
    id          BIGSERIAL PRIMARY KEY,
    event_type  TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id   INT,
    payload     JSONB NOT NULL DEFAULT '{}',
    status      TEXT NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending', 'committed', 'failed')),
    error_msg   TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    committed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS wal_entries_status_idx ON wal_entries (status);
CREATE INDEX IF NOT EXISTS wal_entries_event_type_idx ON wal_entries (event_type);
CREATE INDEX IF NOT EXISTS wal_entries_created_at_idx ON wal_entries (created_at DESC);
