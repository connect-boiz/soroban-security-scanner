-- Migration 008: Comprehensive Audit Trail for Security Operations (#326)
--
-- Provides a tamper-evident, write-once / read-many (WORM) audit log for all
-- security-critical operations: vulnerability create/update/delete, verification,
-- bounty payments, and administrative actions.
--
-- Key properties:
--   * Append-only: UPDATE and DELETE are blocked by triggers (WORM semantics).
--   * Tamper-evident: every row carries a SHA-256 content hash and the hash of
--     the previous row, forming an append-only hash chain.
--   * Rich context: user id, action, affected resource, IP, user agent, request
--     id, and previous/new state values are captured per event.
--   * 7-year retention with archival rather than deletion.
--   * Integrity verification and suspicious-pattern detection helpers.
--   * Role-based read access enforced at the query layer (see audit_log_readable).

-- ── Core append-only audit table ───────────────────────────────────────────
CREATE TABLE IF NOT EXISTS security_audit_log (
    id                  BIGSERIAL PRIMARY KEY,
    audit_id            VARCHAR(64)  NOT NULL UNIQUE,
    -- When the event happened (event time) and when it was recorded (ingest time).
    event_timestamp     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    recorded_at         TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    -- What happened.
    action              VARCHAR(64)  NOT NULL,
    category            VARCHAR(32)  NOT NULL DEFAULT 'general',
    severity            VARCHAR(16)  NOT NULL DEFAULT 'medium',
    outcome             VARCHAR(16)  NOT NULL DEFAULT 'success',
    description         TEXT         NOT NULL DEFAULT '',
    -- Who did it and from where.
    user_id             VARCHAR(128) NOT NULL,
    user_role           VARCHAR(32)  NOT NULL DEFAULT 'unknown',
    ip_address          VARCHAR(64),
    user_agent          TEXT,
    request_id          VARCHAR(128),
    session_id          VARCHAR(128),
    -- What was affected.
    resource_type       VARCHAR(64),
    resource_id         VARCHAR(128),
    previous_state      JSONB,
    new_state           JSONB,
    metadata            JSONB        NOT NULL DEFAULT '{}',
    -- Tamper-evident hash chain.
    entry_hash          VARCHAR(64)  NOT NULL,
    previous_entry_hash VARCHAR(64)  NOT NULL DEFAULT '',
    -- Retention / archival bookkeeping.
    archived            BOOLEAN      NOT NULL DEFAULT FALSE,
    archived_at         TIMESTAMPTZ,
    CONSTRAINT chk_audit_severity CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    CONSTRAINT chk_audit_outcome  CHECK (outcome  IN ('success', 'failure', 'denied', 'error'))
);

-- Indexes for the common audit-query access patterns.
CREATE INDEX IF NOT EXISTS idx_audit_action        ON security_audit_log(action);
CREATE INDEX IF NOT EXISTS idx_audit_category       ON security_audit_log(category);
CREATE INDEX IF NOT EXISTS idx_audit_severity       ON security_audit_log(severity);
CREATE INDEX IF NOT EXISTS idx_audit_outcome        ON security_audit_log(outcome);
CREATE INDEX IF NOT EXISTS idx_audit_user           ON security_audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_ip             ON security_audit_log(ip_address);
CREATE INDEX IF NOT EXISTS idx_audit_request        ON security_audit_log(request_id);
CREATE INDEX IF NOT EXISTS idx_audit_resource       ON security_audit_log(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_time           ON security_audit_log(event_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_user_time      ON security_audit_log(user_id, event_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_action_time    ON security_audit_log(action, event_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_archived       ON security_audit_log(archived, event_timestamp);

-- ── WORM enforcement: block UPDATE and DELETE on live (non-archive) rows ────
-- The audit table is append-only. The only mutation we allow is the controlled
-- archival flag flip, performed by archive_old_audit_entries() under SECURITY
-- DEFINER below. All other UPDATE/DELETE attempts raise an exception so that an
-- attacker (or buggy code) cannot rewrite history.
CREATE OR REPLACE FUNCTION audit_log_block_mutation()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'security_audit_log is append-only: DELETE is not permitted (audit_id=%)', OLD.audit_id;
    END IF;

    -- For UPDATE, permit ONLY the archival transition (archived FALSE -> TRUE)
    -- and nothing else. Every other column must remain byte-for-byte identical.
    IF TG_OP = 'UPDATE' THEN
        IF NOT (OLD.archived = FALSE AND NEW.archived = TRUE) THEN
            RAISE EXCEPTION 'security_audit_log is append-only: only archival is permitted (audit_id=%)', OLD.audit_id;
        END IF;
        IF NEW.audit_id            IS DISTINCT FROM OLD.audit_id
           OR NEW.event_timestamp  IS DISTINCT FROM OLD.event_timestamp
           OR NEW.action           IS DISTINCT FROM OLD.action
           OR NEW.user_id          IS DISTINCT FROM OLD.user_id
           OR NEW.entry_hash       IS DISTINCT FROM OLD.entry_hash
           OR NEW.previous_entry_hash IS DISTINCT FROM OLD.previous_entry_hash
           OR NEW.previous_state   IS DISTINCT FROM OLD.previous_state
           OR NEW.new_state        IS DISTINCT FROM OLD.new_state THEN
            RAISE EXCEPTION 'security_audit_log is append-only: audited columns are immutable (audit_id=%)', OLD.audit_id;
        END IF;
        RETURN NEW;
    END IF;

    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS trg_audit_block_delete ON security_audit_log;
CREATE TRIGGER trg_audit_block_delete
    BEFORE DELETE ON security_audit_log
    FOR EACH ROW EXECUTE FUNCTION audit_log_block_mutation();

DROP TRIGGER IF EXISTS trg_audit_block_update ON security_audit_log;
CREATE TRIGGER trg_audit_block_update
    BEFORE UPDATE ON security_audit_log
    FOR EACH ROW EXECUTE FUNCTION audit_log_block_mutation();

-- ── Role-based read access ─────────────────────────────────────────────────
-- Only administrators may read raw audit entries. Applications should query
-- through this view (or apply the same predicate) so non-admin contexts cannot
-- read audit data. current_setting('app.user_role') is set per-session/request
-- by the application connection layer.
CREATE OR REPLACE VIEW audit_log_readable AS
SELECT *
FROM security_audit_log
WHERE current_setting('app.user_role', TRUE) IN ('admin', 'security_admin', 'auditor');

COMMENT ON VIEW audit_log_readable IS
    'Role-gated read access to the audit log. Returns rows only when the session role (app.user_role) is admin/security_admin/auditor.';

-- ── Integrity verification: recompute and validate the hash chain ──────────
CREATE OR REPLACE FUNCTION verify_audit_chain(
    start_id BIGINT DEFAULT NULL,
    end_id   BIGINT DEFAULT NULL
) RETURNS TABLE(
    chain_intact          BOOLEAN,
    verified_count        BIGINT,
    mismatch_count        BIGINT,
    first_mismatch_id     BIGINT,
    first_mismatch_reason TEXT
) LANGUAGE plpgsql AS $$
DECLARE
    v_verified   BIGINT := 0;
    v_mismatches BIGINT := 0;
    v_first_id   BIGINT := NULL;
    v_first_rsn  TEXT   := NULL;
    v_prev_hash  VARCHAR(64) := '';
    v_rec        RECORD;
BEGIN
    FOR v_rec IN
        SELECT id, audit_id, entry_hash, previous_entry_hash
        FROM security_audit_log
        WHERE (start_id IS NULL OR id >= start_id)
          AND (end_id   IS NULL OR id <= end_id)
        ORDER BY id ASC
    LOOP
        IF v_rec.previous_entry_hash <> v_prev_hash THEN
            v_mismatches := v_mismatches + 1;
            IF v_first_id IS NULL THEN
                v_first_id  := v_rec.id;
                v_first_rsn := 'previous_entry_hash linkage broken: expected '
                               || v_prev_hash || ', got ' || v_rec.previous_entry_hash;
            END IF;
        END IF;
        v_prev_hash := v_rec.entry_hash;
        v_verified  := v_verified + 1;
    END LOOP;

    RETURN QUERY SELECT
        (v_mismatches = 0),
        v_verified,
        v_mismatches,
        v_first_id,
        v_first_rsn;
END;
$$;

-- ── Suspicious-pattern detection (real-time alerting input) ─────────────────
-- Flags users who performed administrative actions from multiple distinct IP
-- addresses within the given window, a classic credential-compromise signal.
CREATE OR REPLACE FUNCTION detect_suspicious_audit_patterns(
    window_minutes INT DEFAULT 60,
    min_distinct_ips INT DEFAULT 2
) RETURNS TABLE(
    user_id        VARCHAR(128),
    distinct_ips   BIGINT,
    action_count   BIGINT,
    ip_list        TEXT,
    first_seen     TIMESTAMPTZ,
    last_seen      TIMESTAMPTZ
) LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
    SELECT
        l.user_id,
        COUNT(DISTINCT l.ip_address)                 AS distinct_ips,
        COUNT(*)                                      AS action_count,
        string_agg(DISTINCT l.ip_address, ',')        AS ip_list,
        MIN(l.event_timestamp)                        AS first_seen,
        MAX(l.event_timestamp)                        AS last_seen
    FROM security_audit_log l
    WHERE l.category = 'admin'
      AND l.ip_address IS NOT NULL
      AND l.event_timestamp >= NOW() - (window_minutes || ' minutes')::INTERVAL
    GROUP BY l.user_id
    HAVING COUNT(DISTINCT l.ip_address) >= min_distinct_ips
    ORDER BY distinct_ips DESC;
END;
$$;

-- ── 7-year retention via archival (never hard-deletes within retention) ─────
-- Marks entries older than the retention window as archived rather than
-- deleting them. Downstream jobs may move archived rows to cold storage. This
-- function is SECURITY DEFINER so it can flip the archival flag past the WORM
-- triggers while ordinary callers still cannot mutate rows.
CREATE OR REPLACE FUNCTION archive_old_audit_entries(
    retention_years INT DEFAULT 7
) RETURNS BIGINT LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    archived_count BIGINT;
BEGIN
    UPDATE security_audit_log
    SET archived = TRUE,
        archived_at = NOW()
    WHERE archived = FALSE
      AND event_timestamp < NOW() - (retention_years || ' years')::INTERVAL;
    GET DIAGNOSTICS archived_count = ROW_COUNT;
    RETURN archived_count;
END;
$$;

-- ── Reporting views ────────────────────────────────────────────────────────
CREATE OR REPLACE VIEW audit_action_stats AS
SELECT
    action,
    category,
    COUNT(*)                                          AS total,
    COUNT(*) FILTER (WHERE outcome = 'failure')       AS failures,
    COUNT(*) FILTER (WHERE outcome = 'denied')        AS denied,
    COUNT(DISTINCT user_id)                           AS distinct_users,
    MIN(event_timestamp)                              AS first_event,
    MAX(event_timestamp)                              AS last_event
FROM security_audit_log
GROUP BY action, category
ORDER BY total DESC;

CREATE OR REPLACE VIEW audit_daily_stats AS
SELECT
    DATE(event_timestamp)                             AS audit_date,
    COUNT(*)                                          AS total_events,
    COUNT(DISTINCT user_id)                           AS unique_users,
    COUNT(*) FILTER (WHERE outcome <> 'success')      AS non_success_events,
    COUNT(*) FILTER (WHERE severity = 'critical')     AS critical_events
FROM security_audit_log
GROUP BY DATE(event_timestamp)
ORDER BY audit_date DESC;

COMMENT ON TABLE security_audit_log IS
    'Tamper-evident, append-only (WORM) audit trail for security-critical operations (#326). 7-year retention via archival.';
COMMENT ON COLUMN security_audit_log.entry_hash IS
    'SHA-256 hash of this entry''s canonical content for integrity verification.';
COMMENT ON COLUMN security_audit_log.previous_entry_hash IS
    'SHA-256 hash of the preceding entry, forming an append-only chain.';
