-- Migration 004: Add Analytics and Reporting Features
-- This migration adds tables for analytics, reporting, and metrics collection

-- Analytics event types
CREATE TYPE analytics_event_type AS ENUM (
    'user_login', 'user_logout', 'user_registration', 'wallet_created',
    'transaction_sent', 'transaction_received', 'bounty_created', 'bounty_submitted',
    'bounty_paid', 'scan_completed', 'security_alert_triggered', 'multi_sig_created',
    'multi_sig_signed', 'api_call', 'page_view', 'feature_used'
);

-- Analytics aggregation periods
CREATE TYPE aggregation_period AS ENUM ('hour', 'day', 'week', 'month', 'quarter', 'year');

-- Analytics events table (raw event data)
CREATE TABLE analytics_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    event_type analytics_event_type NOT NULL,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    session_id UUID REFERENCES user_sessions(id) ON DELETE SET NULL,
    wallet_id UUID REFERENCES wallets(id) ON DELETE SET NULL,
    transaction_id UUID REFERENCES transactions(id) ON DELETE SET NULL,
    bounty_id UUID REFERENCES bounties(id) ON DELETE SET NULL,
    project_id UUID REFERENCES projects(id) ON DELETE SET NULL,
    ip_address INET,
    user_agent TEXT,
    event_data JSONB DEFAULT '{}',
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    processed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Aggregated metrics table
CREATE TABLE aggregated_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    metric_name VARCHAR(100) NOT NULL,
    metric_type VARCHAR(50) NOT NULL, -- count, sum, average, min, max
    aggregation_period aggregation_period NOT NULL,
    period_start TIMESTAMP WITH TIME ZONE NOT NULL,
    period_end TIMESTAMP WITH TIME ZONE NOT NULL,
    value DECIMAL(19, 4) NOT NULL,
    dimensions JSONB DEFAULT '{}', -- Additional dimensions for filtering
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(metric_name, aggregation_period, period_start, dimensions)
);

-- User activity summary table
CREATE TABLE user_activity_summaries (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    login_count INTEGER DEFAULT 0,
    transaction_sent_count INTEGER DEFAULT 0,
    transaction_received_count INTEGER DEFAULT 0,
    bounty_submitted_count INTEGER DEFAULT 0,
    bounty_accepted_count INTEGER DEFAULT 0,
    scan_completed_count INTEGER DEFAULT 0,
    session_duration_seconds INTEGER DEFAULT 0,
    total_transaction_volume DECIMAL(19, 7) DEFAULT 0,
    total_earned DECIMAL(19, 7) DEFAULT 0,
    risk_score_avg DECIMAL(3, 2) DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id, date)
);

-- Platform metrics table
CREATE TABLE platform_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    metric_date DATE NOT NULL UNIQUE,
    total_users INTEGER DEFAULT 0,
    active_users INTEGER DEFAULT 0,
    new_users INTEGER DEFAULT 0,
    total_wallets INTEGER DEFAULT 0,
    active_wallets INTEGER DEFAULT 0,
    total_transactions INTEGER DEFAULT 0,
    transaction_volume DECIMAL(19, 7) DEFAULT 0,
    total_bounties INTEGER DEFAULT 0,
    open_bounties INTEGER DEFAULT 0,
    completed_bounties INTEGER DEFAULT 0,
    total_bounty_value DECIMAL(19, 7) DEFAULT 0,
    paid_bounty_value DECIMAL(19, 7) DEFAULT 0,
    total_scans INTEGER DEFAULT 0,
    completed_scans INTEGER DEFAULT 0,
    vulnerabilities_found INTEGER DEFAULT 0,
    critical_vulnerabilities INTEGER DEFAULT 0,
    security_alerts_count INTEGER DEFAULT 0,
    resolved_security_alerts INTEGER DEFAULT 0,
    avg_response_time_ms INTEGER DEFAULT 0,
    system_uptime_percentage DECIMAL(5, 2) DEFAULT 100,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Performance metrics table
CREATE TABLE performance_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    metric_timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    cpu_usage_percentage DECIMAL(5, 2),
    memory_usage_percentage DECIMAL(5, 2),
    disk_usage_percentage DECIMAL(5, 2),
    network_io_bytes BIGINT,
    database_connections_active INTEGER,
    database_connections_idle INTEGER,
    database_query_time_avg_ms DECIMAL(8, 2),
    api_requests_per_second DECIMAL(8, 2),
    error_rate_percentage DECIMAL(5, 2),
    response_time_p50_ms INTEGER,
    response_time_p95_ms INTEGER,
    response_time_p99_ms INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Reports table (for saved and scheduled reports)
CREATE TABLE reports (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    report_type VARCHAR(50) NOT NULL, -- daily, weekly, monthly, custom
    query_sql TEXT NOT NULL,
    parameters JSONB DEFAULT '{}',
    schedule JSONB, -- Cron-like schedule definition
    recipients JSONB DEFAULT '[]', -- List of email recipients
    is_active BOOLEAN DEFAULT TRUE,
    last_run_at TIMESTAMP WITH TIME ZONE,
    next_run_at TIMESTAMP WITH TIME ZONE,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Report executions table
CREATE TABLE report_executions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    report_id UUID NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
    execution_status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, running, completed, failed
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    execution_time_seconds INTEGER,
    result_data JSONB, -- Report results
    error_message TEXT,
    file_path VARCHAR(500), -- Path to generated report file
    file_size_bytes BIGINT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- User preferences for analytics
CREATE TABLE user_analytics_preferences (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    preference_key VARCHAR(100) NOT NULL,
    preference_value JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id, preference_key)
);

-- Create indexes for analytics tables

-- Analytics events indexes
CREATE INDEX idx_analytics_events_event_type ON analytics_events(event_type);
CREATE INDEX idx_analytics_events_user_id ON analytics_events(user_id);
CREATE INDEX idx_analytics_events_timestamp ON analytics_events(timestamp);
CREATE INDEX idx_analytics_events_processed ON analytics_events(processed);
CREATE INDEX idx_analytics_events_session_id ON analytics_events(session_id);
CREATE INDEX idx_analytics_events_wallet_id ON analytics_events(wallet_id);
CREATE INDEX idx_analytics_events_transaction_id ON analytics_events(transaction_id);
CREATE INDEX idx_analytics_events_bounty_id ON analytics_events(bounty_id);

-- Aggregated metrics indexes
CREATE INDEX idx_aggregated_metrics_name ON aggregated_metrics(metric_name);
CREATE INDEX idx_aggregated_metrics_period ON aggregated_metrics(aggregation_period);
CREATE INDEX idx_aggregated_metrics_period_start ON aggregated_metrics(period_start);
CREATE INDEX idx_aggregated_metrics_period_end ON aggregated_metrics(period_end);
CREATE INDEX idx_aggregated_metrics_created_at ON aggregated_metrics(created_at);

-- User activity summaries indexes
CREATE INDEX idx_user_activity_user_id ON user_activity_summaries(user_id);
CREATE INDEX idx_user_activity_date ON user_activity_summaries(date);
CREATE INDEX idx_user_activity_login_count ON user_activity_summaries(login_count);

-- Platform metrics indexes
CREATE INDEX idx_platform_metrics_date ON platform_metrics(metric_date);
CREATE INDEX idx_platform_metrics_created_at ON platform_metrics(created_at);

-- Performance metrics indexes
CREATE INDEX idx_performance_metrics_timestamp ON performance_metrics(metric_timestamp);
CREATE INDEX idx_performance_metrics_cpu_usage ON performance_metrics(cpu_usage_percentage);
CREATE INDEX idx_performance_metrics_memory_usage ON performance_metrics(memory_usage_percentage);
CREATE INDEX idx_performance_metrics_created_at ON performance_metrics(created_at);

-- Reports indexes
CREATE INDEX idx_reports_type ON reports(report_type);
CREATE INDEX idx_reports_is_active ON reports(is_active);
CREATE INDEX idx_reports_next_run ON reports(next_run_at);
CREATE INDEX idx_reports_created_by ON reports(created_by);
CREATE INDEX idx_reports_created_at ON reports(created_at);

-- Report executions indexes
CREATE INDEX idx_report_executions_report_id ON report_executions(report_id);
CREATE INDEX idx_report_executions_status ON report_executions(execution_status);
CREATE INDEX idx_report_executions_started_at ON report_executions(started_at);
CREATE INDEX idx_report_executions_created_at ON report_executions(created_at);

-- User analytics preferences indexes
CREATE INDEX idx_user_analytics_preferences_user_id ON user_analytics_preferences(user_id);
CREATE INDEX idx_user_analytics_preferences_key ON user_analytics_preferences(preference_key);

-- Apply triggers for timestamp updates
CREATE TRIGGER update_aggregated_metrics_updated_at BEFORE UPDATE ON aggregated_metrics FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_user_activity_summaries_updated_at BEFORE UPDATE ON user_activity_summaries FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_platform_metrics_updated_at BEFORE UPDATE ON platform_metrics FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_reports_updated_at BEFORE UPDATE ON reports FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_user_analytics_preferences_updated_at BEFORE UPDATE ON user_analytics_preferences FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add constraints
ALTER TABLE user_activity_summaries ADD CONSTRAINT check_non_negative_counts CHECK (
    login_count >= 0 AND 
    transaction_sent_count >= 0 AND 
    transaction_received_count >= 0 AND
    bounty_submitted_count >= 0 AND
    bounty_accepted_count >= 0 AND
    scan_completed_count >= 0 AND
    session_duration_seconds >= 0
);

ALTER TABLE platform_metrics ADD CONSTRAINT check_non_negative_platform_metrics CHECK (
    total_users >= 0 AND
    active_users >= 0 AND
    new_users >= 0 AND
    total_wallets >= 0 AND
    active_wallets >= 0 AND
    total_transactions >= 0 AND
    transaction_volume >= 0 AND
    total_bounties >= 0 AND
    open_bounties >= 0 AND
    completed_bounties >= 0 AND
    total_bounty_value >= 0 AND
    paid_bounty_value >= 0 AND
    total_scans >= 0 AND
    completed_scans >= 0 AND
    vulnerabilities_found >= 0 AND
    critical_vulnerabilities >= 0 AND
    security_alerts_count >= 0 AND
    resolved_security_alerts_count >= 0 AND
    avg_response_time_ms >= 0 AND
    system_uptime_percentage >= 0 AND system_uptime_percentage <= 100
);

ALTER TABLE performance_metrics ADD CONSTRAINT check_performance_ranges CHECK (
    (cpu_usage_percentage IS NULL OR (cpu_usage_percentage >= 0 AND cpu_usage_percentage <= 100)) AND
    (memory_usage_percentage IS NULL OR (memory_usage_percentage >= 0 AND memory_usage_percentage <= 100)) AND
    (disk_usage_percentage IS NULL OR (disk_usage_percentage >= 0 AND disk_usage_percentage <= 100)) AND
    (database_connections_active IS NULL OR database_connections_active >= 0) AND
    (database_connections_idle IS NULL OR database_connections_idle >= 0) AND
    (database_query_time_avg_ms IS NULL OR database_query_time_avg_ms >= 0) AND
    (api_requests_per_second IS NULL OR api_requests_per_second >= 0) AND
    (error_rate_percentage IS NULL OR (error_rate_percentage >= 0 AND error_rate_percentage <= 100)) AND
    (response_time_p50_ms IS NULL OR response_time_p50_ms >= 0) AND
    (response_time_p95_ms IS NULL OR response_time_p95_ms >= 0) AND
    (response_time_p99_ms IS NULL OR response_time_p99_ms >= 0)
);

ALTER TABLE report_executions ADD CONSTRAINT check_execution_time CHECK (execution_time_seconds >= 0);
ALTER TABLE report_executions ADD CONSTRAINT check_file_size CHECK (file_size_bytes IS NULL OR file_size_bytes >= 0);

-- Create views for analytics

-- Daily user activity view
CREATE VIEW daily_user_activity AS
SELECT 
    user_id,
    DATE(timestamp) as activity_date,
    COUNT(*) FILTER (WHERE event_type = 'user_login') as login_count,
    COUNT(*) FILTER (WHERE event_type = 'transaction_sent') as transactions_sent,
    COUNT(*) FILTER (WHERE event_type = 'transaction_received') as transactions_received,
    COUNT(*) FILTER (WHERE event_type = 'bounty_submitted') as bounties_submitted,
    COUNT(*) FILTER (WHERE event_type = 'scan_completed') as scans_completed,
    COUNT(DISTINCT session_id) as unique_sessions
FROM analytics_events
WHERE timestamp >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY user_id, DATE(timestamp)
ORDER BY activity_date DESC;

-- Platform overview view
CREATE VIEW platform_overview AS
SELECT 
    metric_date,
    total_users,
    active_users,
    new_users,
    total_transactions,
    transaction_volume,
    total_bounties,
    open_bounties,
    completed_bounties,
    total_bounty_value,
    paid_bounty_value,
    vulnerabilities_found,
    security_alerts_count,
    avg_response_time_ms
FROM platform_metrics
ORDER BY metric_date DESC
LIMIT 30;

-- Top users by activity view
CREATE VIEW top_users_by_activity AS
SELECT 
    u.id,
    u.username,
    COALESCE(sums.total_logins, 0) as total_logins,
    COALESCE(sums.total_transactions, 0) as total_transactions,
    COALESCE(sums.total_bounties, 0) as total_bounties,
    COALESCE(sums.total_volume, 0) as total_volume,
    u.reputation_score
FROM users u
LEFT JOIN (
    SELECT 
        user_id,
        SUM(login_count) as total_logins,
        SUM(transaction_sent_count + transaction_received_count) as total_transactions,
        SUM(bounty_submitted_count + bounty_accepted_count) as total_bounties,
        SUM(total_transaction_volume) as total_volume
    FROM user_activity_summaries
    WHERE date >= CURRENT_DATE - INTERVAL '30 days'
    GROUP BY user_id
) sums ON u.id = sums.user_id
WHERE u.status = 'active'
ORDER BY COALESCE(sums.total_logins, 0) DESC, COALESCE(sums.total_transactions, 0) DESC
LIMIT 100;

-- Transaction trends view
CREATE VIEW transaction_trends AS
SELECT 
    DATE_TRUNC('day', timestamp) as transaction_date,
    COUNT(*) FILTER (WHERE event_type = 'transaction_sent') as sent_count,
    COUNT(*) FILTER (WHERE event_type = 'transaction_received') as received_count,
    COUNT(*) as total_count
FROM analytics_events
WHERE event_type IN ('transaction_sent', 'transaction_received')
  AND timestamp >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY DATE_TRUNC('day', timestamp)
ORDER BY transaction_date DESC;

-- Bounty performance view
CREATE VIEW bounty_performance AS
SELECT 
    DATE_TRUNC('week', created_at) as week_start,
    COUNT(*) as created_count,
    COUNT(*) FILTER (WHERE status = 'open') as open_count,
    COUNT(*) FILTER (WHERE status = 'completed') as completed_count,
    COUNT(*) FILTER (WHERE status = 'paid') as paid_count,
    COALESCE(SUM(reward_amount), 0) as total_reward_value,
    AVG(reward_amount) as average_reward
FROM bounties
WHERE created_at >= CURRENT_DATE - INTERVAL '12 weeks'
GROUP BY DATE_TRUNC('week', created_at)
ORDER BY week_start DESC;

-- Create stored procedures for analytics

-- Procedure to aggregate daily metrics
CREATE OR REPLACE FUNCTION aggregate_daily_metrics(p_target_date DATE DEFAULT CURRENT_DATE)
RETURNS VOID AS $$
DECLARE
    day_start TIMESTAMP WITH TIME ZONE;
    day_end TIMESTAMP WITH TIME ZONE;
BEGIN
    day_start := p_target_date AT TIME ZONE 'UTC' AT TIME ZONE 'UTC';
    day_end := day_start + INTERVAL '1 day';
    
    -- Aggregate user activity
    INSERT INTO user_activity_summaries (
        user_id, date, login_count, transaction_sent_count, 
        transaction_received_count, bounty_submitted_count, 
        scan_completed_count, session_duration_seconds, 
        total_transaction_volume, total_earned
    )
    SELECT 
        user_id,
        p_target_date,
        COUNT(*) FILTER (WHERE event_type = 'user_login'),
        COUNT(*) FILTER (WHERE event_type = 'transaction_sent'),
        COUNT(*) FILTER (WHERE event_type = 'transaction_received'),
        COUNT(*) FILTER (WHERE event_type = 'bounty_submitted'),
        COUNT(*) FILTER (WHERE event_type = 'scan_completed'),
        COALESCE(SUM(EXTRACT(EPOCH FROM (event_data->>'session_duration')::INTERVAL)), 0)::INTEGER,
        COALESCE(SUM(CASE WHEN event_type = 'transaction_sent' THEN (event_data->>'amount')::DECIMAL ELSE 0 END), 0),
        COALESCE(SUM(CASE WHEN event_type = 'bounty_paid' THEN (event_data->>'amount')::DECIMAL ELSE 0 END), 0)
    FROM analytics_events
    WHERE timestamp >= day_start AND timestamp < day_end
      AND user_id IS NOT NULL
    GROUP BY user_id
    ON CONFLICT (user_id, date) DO UPDATE SET
        login_count = EXCLUDED.login_count,
        transaction_sent_count = EXCLUDED.transaction_sent_count,
        transaction_received_count = EXCLUDED.transaction_received_count,
        bounty_submitted_count = EXCLUDED.bounty_submitted_count,
        scan_completed_count = EXCLUDED.scan_completed_count,
        session_duration_seconds = EXCLUDED.session_duration_seconds,
        total_transaction_volume = EXCLUDED.total_transaction_volume,
        total_earned = EXCLUDED.total_earned,
        updated_at = NOW();
    
    -- Aggregate platform metrics
    INSERT INTO platform_metrics (
        metric_date, total_users, active_users, new_users,
        total_wallets, active_wallets, total_transactions,
        transaction_volume, total_bounties, open_bounties,
        completed_bounties, total_bounty_value, paid_bounty_value,
        total_scans, completed_scans, vulnerabilities_found,
        critical_vulnerabilities, security_alerts_count,
        resolved_security_alerts
    )
    SELECT 
        p_target_date,
        (SELECT COUNT(*) FROM users),
        (SELECT COUNT(*) FROM users WHERE last_login_at >= day_start),
        (SELECT COUNT(*) FROM users WHERE DATE(created_at) = p_target_date),
        (SELECT COUNT(*) FROM wallets),
        (SELECT COUNT(*) FROM wallets WHERE last_transaction_at >= day_start),
        (SELECT COUNT(*) FROM transactions WHERE DATE(created_at) = p_target_date),
        COALESCE((SELECT SUM(amount_lumens) FROM transactions WHERE DATE(created_at) = p_target_date), 0),
        (SELECT COUNT(*) FROM bounties),
        (SELECT COUNT(*) FROM bounties WHERE status = 'open'),
        (SELECT COUNT(*) FROM bounties WHERE status = 'completed'),
        COALESCE((SELECT SUM(reward_amount) FROM bounties), 0),
        COALESCE((SELECT SUM(reward_amount) FROM bounties WHERE status = 'paid'), 0),
        (SELECT COUNT(*) FROM analytics_events WHERE event_type = 'scan_completed' AND DATE(timestamp) = p_target_date),
        (SELECT COUNT(*) FROM analytics_events WHERE event_type = 'scan_completed' AND DATE(timestamp) = p_target_date),
        (SELECT COUNT(*) FROM security_alerts WHERE DATE(created_at) = p_target_date),
        (SELECT COUNT(*) FROM security_alerts WHERE severity = 'critical' AND DATE(created_at) = p_target_date),
        (SELECT COUNT(*) FROM security_alerts WHERE DATE(created_at) = p_target_date),
        (SELECT COUNT(*) FROM security_alerts WHERE status = 'resolved' AND DATE(resolved_at) = p_target_date)
    ON CONFLICT (metric_date) DO UPDATE SET
        total_users = EXCLUDED.total_users,
        active_users = EXCLUDED.active_users,
        new_users = EXCLUDED.new_users,
        total_wallets = EXCLUDED.total_wallets,
        active_wallets = EXCLUDED.active_wallets,
        total_transactions = EXCLUDED.total_transactions,
        transaction_volume = EXCLUDED.transaction_volume,
        total_bounties = EXCLUDED.total_bounties,
        open_bounties = EXCLUDED.open_bounties,
        completed_bounties = EXCLUDED.completed_bounties,
        total_bounty_value = EXCLUDED.total_bounty_value,
        paid_bounty_value = EXCLUDED.paid_bounty_value,
        total_scans = EXCLUDED.total_scans,
        completed_scans = EXCLUDED.completed_scans,
        vulnerabilities_found = EXCLUDED.vulnerabilities_found,
        critical_vulnerabilities = EXCLUDED.critical_vulnerabilities,
        security_alerts_count = EXCLUDED.security_alerts_count,
        resolved_security_alerts = EXCLUDED.resolved_security_alerts,
        updated_at = NOW();
END;
$$ LANGUAGE plpgsql;

-- Procedure to cleanup old analytics events
CREATE OR REPLACE FUNCTION cleanup_analytics_events(p_days_to_keep INTEGER DEFAULT 90)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM analytics_events
    WHERE timestamp < NOW() - INTERVAL '1 day' * p_days_to_keep
      AND processed = TRUE;
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Procedure to generate daily report
CREATE OR REPLACE FUNCTION generate_daily_report(p_report_date DATE DEFAULT CURRENT_DATE)
RETURNS JSONB AS $$
DECLARE
    report_data JSONB;
BEGIN
    SELECT jsonb_build_object(
        'date', p_report_date,
        'user_metrics', (
            SELECT jsonb_build_object(
                'total_users', total_users,
                'active_users', active_users,
                'new_users', new_users
            )
            FROM platform_metrics WHERE metric_date = p_report_date
        ),
        'transaction_metrics', (
            SELECT jsonb_build_object(
                'total_transactions', total_transactions,
                'transaction_volume', transaction_volume
            )
            FROM platform_metrics WHERE metric_date = p_report_date
        ),
        'bounty_metrics', (
            SELECT jsonb_build_object(
                'total_bounties', total_bounties,
                'open_bounties', open_bounties,
                'completed_bounties', completed_bounties,
                'total_bounty_value', total_bounty_value
            )
            FROM platform_metrics WHERE metric_date = p_report_date
        ),
        'security_metrics', (
            SELECT jsonb_build_object(
                'vulnerabilities_found', vulnerabilities_found,
                'critical_vulnerabilities', critical_vulnerabilities,
                'security_alerts_count', security_alerts_count
            )
            FROM platform_metrics WHERE metric_date = p_report_date
        )
    ) INTO report_data;
    
    RETURN report_data;
END;
$$ LANGUAGE plpgsql;
