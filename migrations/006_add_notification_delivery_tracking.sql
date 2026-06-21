-- Migration 006: Notification Delivery Tracking Persistence
-- Adds a table for persistent delivery status tracking of notifications

-- ============================================================
-- Delivery status enum type
-- ============================================================
DO $$ BEGIN
    CREATE TYPE delivery_status AS ENUM (
        'pending', 'processing', 'sent', 'delivered', 'failed', 'retrying'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- ============================================================
-- Notification delivery tracking table
-- ============================================================
CREATE TABLE notification_delivery_tracking (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    notification_id VARCHAR(255) NOT NULL,
    recipient_id VARCHAR(255) NOT NULL,
    channel VARCHAR(50) NOT NULL, -- email, sms, push, in_app
    status delivery_status NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    last_attempt_at TIMESTAMP WITH TIME ZONE,
    delivered_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    external_id VARCHAR(255), -- External provider tracking ID (e.g. SendGrid message ID)
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(notification_id, recipient_id, channel)
);

-- ============================================================
-- Indexes for efficient queries
-- ============================================================
CREATE INDEX idx_ntd_notification_id ON notification_delivery_tracking(notification_id);
CREATE INDEX idx_ntd_recipient_id ON notification_delivery_tracking(recipient_id);
CREATE INDEX idx_ntd_channel ON notification_delivery_tracking(channel);
CREATE INDEX idx_ntd_status ON notification_delivery_tracking(status);
CREATE INDEX idx_ntd_last_attempt_at ON notification_delivery_tracking(last_attempt_at);
CREATE INDEX idx_ntd_delivered_at ON notification_delivery_tracking(delivered_at);
CREATE INDEX idx_ntd_created_at ON notification_delivery_tracking(created_at);
CREATE INDEX idx_ntd_recipient_channel ON notification_delivery_tracking(recipient_id, channel);
CREATE INDEX idx_ntd_status_attempts ON notification_delivery_tracking(status, attempts) WHERE status = 'failed';
CREATE INDEX idx_ntd_cleanup ON notification_delivery_tracking(created_at) WHERE status IN ('delivered', 'failed');

-- ============================================================
-- Trigger for auto-updating updated_at
-- ============================================================
CREATE TRIGGER update_notification_delivery_tracking_updated_at
    BEFORE UPDATE ON notification_delivery_tracking
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================
-- Notification preferences per user table
-- ============================================================
CREATE TABLE notification_preferences (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel VARCHAR(50) NOT NULL, -- email, sms, push, in_app
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    quiet_hours_start INTEGER, -- Hour in 0-23 (recipient's timezone)
    quiet_hours_end INTEGER,   -- Hour in 0-23 (recipient's timezone)
    timezone VARCHAR(50) DEFAULT 'UTC',
    max_priority VARCHAR(20) DEFAULT 'normal', -- low, normal, high, critical
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id, channel)
);

CREATE INDEX idx_notification_preferences_user_id ON notification_preferences(user_id);
CREATE INDEX idx_notification_preferences_channel ON notification_preferences(channel);
CREATE INDEX idx_notification_preferences_enabled ON notification_preferences(enabled);

CREATE TRIGGER update_notification_preferences_updated_at
    BEFORE UPDATE ON notification_preferences
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================
-- View: recent delivery failures (for retry service)
-- ============================================================
CREATE VIEW recent_delivery_failures AS
SELECT *
FROM notification_delivery_tracking
WHERE status = 'failed'
  AND attempts < 3
  AND last_attempt_at >= NOW() - INTERVAL '24 hours'
ORDER BY last_attempt_at DESC;

-- ============================================================
-- View: daily delivery stats (for monitoring dashboard)
-- ============================================================
CREATE VIEW daily_delivery_stats AS
SELECT
    DATE(created_at) as delivery_date,
    channel,
    COUNT(*) as total_attempts,
    COUNT(*) FILTER (WHERE status = 'delivered') as delivered_count,
    COUNT(*) FILTER (WHERE status = 'failed') as failed_count,
    COUNT(*) FILTER (WHERE status = 'retrying') as retrying_count,
    COALESCE(AVG(CASE WHEN status = 'delivered' AND delivered_at IS NOT NULL
        THEN EXTRACT(EPOCH FROM (delivered_at - last_attempt_at)) END), 0)
        as avg_delivery_time_seconds
FROM notification_delivery_tracking
WHERE created_at >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY DATE(created_at), channel
ORDER BY delivery_date DESC, channel;

-- ============================================================
-- Stored procedure: retry failed deliveries
-- ============================================================
CREATE OR REPLACE FUNCTION retry_failed_deliveries(max_retries INTEGER DEFAULT 3)
RETURNS TABLE (
    tracking_id UUID,
    notification_id VARCHAR,
    recipient_id VARCHAR,
    channel VARCHAR,
    current_attempts INTEGER
) AS $$
BEGIN
    RETURN QUERY
    UPDATE notification_delivery_tracking
    SET
        status = 'retrying'::delivery_status,
        attempts = attempts + 1,
        last_attempt_at = NOW(),
        updated_at = NOW()
    WHERE id IN (
        SELECT id
        FROM notification_delivery_tracking
        WHERE status = 'failed'
          AND attempts < max_retries
          AND last_attempt_at <= NOW() - INTERVAL '5 minutes'
        ORDER BY last_attempt_at ASC
        LIMIT 100
        FOR UPDATE SKIP LOCKED
    )
    RETURNING
        id,
        notification_id,
        recipient_id,
        channel,
        attempts;
END;
$$ LANGUAGE plpgsql;

-- ============================================================
-- Stored procedure: cleanup old delivery records
-- ============================================================
CREATE OR REPLACE FUNCTION cleanup_old_delivery_records(retention_days INTEGER DEFAULT 90)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM notification_delivery_tracking
    WHERE created_at < NOW() - (retention_days || ' days')::INTERVAL
      AND status IN ('delivered', 'failed');
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;
