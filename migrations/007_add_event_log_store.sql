-- Migration 007: Add Event Log Store for Compliance Auditing
-- Supports paginated queries, CSV/JSON export, and tamper-evident hash chaining.

-- Event log table with hash chaining support
CREATE TABLE IF NOT EXISTS event_log_store (
    id              BIGSERIAL PRIMARY KEY,
    event_id        VARCHAR(64) NOT NULL UNIQUE,
    timestamp       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    operation       VARCHAR(64) NOT NULL,
    severity        VARCHAR(16) NOT NULL DEFAULT 'Medium',
    status          VARCHAR(16) NOT NULL DEFAULT 'Started',
    description     TEXT NOT NULL DEFAULT '',
    actor           VARCHAR(128) NOT NULL,
    target          VARCHAR(256),
    metadata        JSONB DEFAULT '{}',
    previous_state  TEXT,
    new_state       TEXT,
    error_message   TEXT,
    execution_duration_ms BIGINT,
    gas_consumed    BIGINT,
    transaction_hash VARCHAR(128),
    ledger_sequence BIGINT,
    -- Hash chain fields (tamper-evident)
    event_hash      VARCHAR(64) NOT NULL,
    previous_event_hash VARCHAR(64) NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for paginated queries and exports
CREATE INDEX IF NOT EXISTS idx_event_log_operation ON event_log_store(operation);
CREATE INDEX IF NOT EXISTS idx_event_log_actor ON event_log_store(actor);
CREATE INDEX IF NOT EXISTS idx_event_log_timestamp ON event_log_store(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_event_log_severity ON event_log_store(severity);
CREATE INDEX IF NOT EXISTS idx_event_log_status ON event_log_store(status);
CREATE INDEX IF NOT EXISTS idx_event_log_hash ON event_log_store(event_hash);
CREATE INDEX IF NOT EXISTS idx_event_log_prev_hash ON event_log_store(previous_event_hash);
-- Compound index for common query pattern (actor + time range)
CREATE INDEX IF NOT EXISTS idx_event_log_actor_time ON event_log_store(actor, timestamp DESC);
-- Compound index for operation + time range queries
CREATE INDEX IF NOT EXISTS idx_event_log_operation_time ON event_log_store(operation, timestamp DESC);

-- View: Recent events for dashboard
CREATE OR REPLACE VIEW recent_events AS
SELECT
    event_id,
    timestamp,
    operation,
    severity,
    status,
    description,
    actor,
    target,
    event_hash,
    previous_event_hash
FROM event_log_store
ORDER BY timestamp DESC
LIMIT 100;

-- View: Events grouped by operation with counts
CREATE OR REPLACE VIEW event_operation_stats AS
SELECT
    operation,
    COUNT(*) AS total_events,
    COUNT(*) FILTER (WHERE severity = 'Critical') AS critical_count,
    COUNT(*) FILTER (WHERE severity = 'High') AS high_count,
    COUNT(*) FILTER (WHERE severity = 'Medium') AS medium_count,
    COUNT(*) FILTER (WHERE severity = 'Low') AS low_count,
    MIN(timestamp) AS first_event,
    MAX(timestamp) AS last_event
FROM event_log_store
GROUP BY operation
ORDER BY total_events DESC;

-- View: Daily event counts for compliance reports
CREATE OR REPLACE VIEW daily_event_stats AS
SELECT
    DATE(timestamp) AS event_date,
    COUNT(*) AS total_events,
    COUNT(DISTINCT actor) AS unique_actors,
    COUNT(*) FILTER (WHERE status = 'Failed') AS failed_events,
    COUNT(*) FILTER (WHERE severity = 'Critical') AS critical_events
FROM event_log_store
GROUP BY DATE(timestamp)
ORDER BY event_date DESC;

-- Function: Verify hash chain integrity for a range of events
CREATE OR REPLACE FUNCTION verify_event_chain(
    start_id BIGINT DEFAULT NULL,
    end_id BIGINT DEFAULT NULL
) RETURNS TABLE(
    chain_integrity BOOLEAN,
    verified_count BIGINT,
    mismatch_count BIGINT,
    first_mismatch_id BIGINT,
    first_mismatch_reason TEXT
) LANGUAGE plpgsql AS $$
DECLARE
    v_verified BIGINT := 0;
    v_mismatches BIGINT := 0;
    v_first_mismatch_id BIGINT := NULL;
    v_first_mismatch_reason TEXT := NULL;
    v_prev_hash VARCHAR(64) := '';
    v_rec RECORD;
BEGIN
    FOR v_rec IN
        SELECT id, event_id, event_hash, previous_event_hash
        FROM event_log_store
        WHERE (start_id IS NULL OR id >= start_id)
          AND (end_id IS NULL OR id <= end_id)
        ORDER BY id ASC
    LOOP
        -- Check previous hash linkage
        IF v_rec.previous_event_hash != v_prev_hash THEN
            v_mismatches := v_mismatches + 1;
            IF v_first_mismatch_id IS NULL THEN
                v_first_mismatch_id := v_rec.id;
                v_first_mismatch_reason := 'Previous event hash mismatch: expected ' || v_prev_hash || ', got ' || v_rec.previous_event_hash;
            END IF;
        END IF;

        v_prev_hash := v_rec.event_hash;
        v_verified := v_verified + 1;
    END LOOP;

    RETURN QUERY SELECT
        (v_mismatches = 0) AS chain_integrity,
        v_verified AS verified_count,
        v_mismatches AS mismatch_count,
        v_first_mismatch_id,
        v_first_mismatch_reason;
END;
$$;

-- Function: Export events as CSV-compatible format for a time range
CREATE OR REPLACE FUNCTION export_events_csv(
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ,
    max_rows INT DEFAULT 10000
) RETURNS TABLE(csv_line TEXT) LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
    SELECT
        format('%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s',
            event_id,
            EXTRACT(EPOCH FROM timestamp)::BIGINT,
            operation,
            severity,
            status,
            regexp_replace(description, '["\\n]', ' ', 'g'),
            actor,
            COALESCE(target, ''),
            COALESCE(event_hash, ''),
            COALESCE(previous_event_hash, ''),
            COALESCE(error_message, ''),
            COALESCE(execution_duration_ms::TEXT, '0'),
            COALESCE(gas_consumed::TEXT, '0')
        ) AS csv_line
    FROM event_log_store
    WHERE timestamp >= start_time
      AND timestamp <= end_time
    ORDER BY timestamp ASC
    LIMIT max_rows;
END;
$$;

-- Procedure: Clean up old event records beyond retention period
CREATE OR REPLACE FUNCTION cleanup_old_events(
    retention_days INT DEFAULT 90
) RETURNS BIGINT LANGUAGE plpgsql AS $$
DECLARE
    deleted_count BIGINT;
BEGIN
    DELETE FROM event_log_store
    WHERE timestamp < NOW() - (retention_days || ' days')::INTERVAL;
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$;

COMMENT ON TABLE event_log_store IS 'Stores audit events with tamper-evident SHA-256 hash chaining for compliance';
COMMENT ON COLUMN event_log_store.event_hash IS 'SHA-256 hash of this events content for integrity verification';
COMMENT ON COLUMN event_log_store.previous_event_hash IS 'SHA-256 hash of the previous event, forming a chain';
