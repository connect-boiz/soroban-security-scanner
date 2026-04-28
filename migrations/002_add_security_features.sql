-- Migration 002: Add Security Features and Enhanced Constraints
-- This migration adds additional security-related columns, constraints, and indexes

-- Add security columns to users table
ALTER TABLE users ADD COLUMN failed_login_attempts INTEGER DEFAULT 0;
ALTER TABLE users ADD COLUMN last_failed_login_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE users ADD COLUMN account_locked_until TIMESTAMP WITH TIME ZONE;
ALTER TABLE users ADD COLUMN security_questions JSONB DEFAULT '{}';
ALTER TABLE users ADD COLUMN backup_codes JSONB DEFAULT '[]';
ALTER TABLE users ADD COLUMN ip_whitelist JSONB DEFAULT '[]';
ALTER TABLE users ADD COLUMN device_fingerprints JSONB DEFAULT '[]';
ALTER TABLE users ADD COLUMN risk_score INTEGER DEFAULT 0; -- 0-100 risk assessment

-- Add security columns to wallets table
ALTER TABLE wallets ADD COLUMN last_security_scan_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE wallets ADD COLUMN security_scan_result JSONB DEFAULT '{}';
ALTER TABLE wallets ADD COLUMN suspicious_activity_count INTEGER DEFAULT 0;
ALTER TABLE wallets ADD COLUMN last_suspicious_activity_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE wallets ADD COLUMN transaction_limits JSONB DEFAULT '{}'; -- Daily, weekly, monthly limits
ALTER TABLE wallets ADD COLUMN approved_origins JSONB DEFAULT '[]'; -- Approved transaction origins

-- Add security columns to transactions table
ALTER TABLE transactions ADD COLUMN risk_level VARCHAR(20) DEFAULT 'low'; -- low, medium, high, critical
ALTER TABLE transactions ADD COLUMN fraud_score INTEGER DEFAULT 0; -- 0-100 fraud likelihood
ALTER TABLE transactions ADD COLUMN ip_address INET;
ALTER TABLE transactions ADD COLUMN device_fingerprint VARCHAR(255);
ALTER TABLE transactions ADD COLUMN geolocation JSONB DEFAULT '{}';
ALTER TABLE transactions ADD COLUMN is_suspicious BOOLEAN DEFAULT FALSE;
ALTER TABLE transactions ADD COLUMN requires_review BOOLEAN DEFAULT FALSE;
ALTER TABLE transactions ADD COLUMN reviewed_by UUID REFERENCES users(id);
ALTER TABLE transactions ADD COLUMN reviewed_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE transactions ADD COLUMN review_notes TEXT;

-- Create security alerts table
CREATE TABLE security_alerts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    wallet_id UUID REFERENCES wallets(id) ON DELETE CASCADE,
    transaction_id UUID REFERENCES transactions(id) ON DELETE CASCADE,
    alert_type VARCHAR(50) NOT NULL, -- suspicious_login, unusual_transaction, failed_multi_sig, etc.
    severity VARCHAR(20) NOT NULL DEFAULT 'medium', -- low, medium, high, critical
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    alert_data JSONB DEFAULT '{}',
    status VARCHAR(20) NOT NULL DEFAULT 'open', -- open, investigating, resolved, false_positive
    resolved_by UUID REFERENCES users(id),
    resolved_at TIMESTAMP WITH TIME ZONE,
    resolution_notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create rate limiting table
CREATE TABLE rate_limits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    identifier VARCHAR(255) NOT NULL, -- IP address, user ID, or wallet address
    resource_type VARCHAR(50) NOT NULL, -- api_calls, transactions, logins, etc.
    window_start TIMESTAMP WITH TIME ZONE NOT NULL,
    window_end TIMESTAMP WITH TIME ZONE NOT NULL,
    request_count INTEGER NOT NULL DEFAULT 1,
    max_allowed INTEGER NOT NULL DEFAULT 100,
    is_blocked BOOLEAN DEFAULT FALSE,
    block_expires_at TIMESTAMP WITH TIME ZONE,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(identifier, resource_type, window_start)
);

-- Create device tracking table
CREATE TABLE user_devices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_fingerprint VARCHAR(255) NOT NULL,
    device_name VARCHAR(100),
    device_type VARCHAR(50), -- desktop, mobile, tablet, etc.
    operating_system VARCHAR(100),
    browser VARCHAR(100),
    ip_address INET,
    user_agent TEXT,
    is_trusted BOOLEAN DEFAULT FALSE,
    last_seen_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    first_seen_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    usage_count INTEGER DEFAULT 1,
    metadata JSONB DEFAULT '{}',
    UNIQUE(user_id, device_fingerprint)
);

-- Create access patterns table for behavioral analysis
CREATE TABLE access_patterns (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    pattern_type VARCHAR(50) NOT NULL, -- login_time, transaction_amount, location, etc.
    pattern_data JSONB NOT NULL,
    confidence_score DECIMAL(3, 2) DEFAULT 0.0, -- 0.00-1.00
    is_anomaly BOOLEAN DEFAULT FALSE,
    detected_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'
);

-- Add indexes for new security tables
CREATE INDEX idx_security_alerts_user_id ON security_alerts(user_id);
CREATE INDEX idx_security_alerts_wallet_id ON security_alerts(wallet_id);
CREATE INDEX idx_security_alerts_transaction_id ON security_alerts(transaction_id);
CREATE INDEX idx_security_alerts_type ON security_alerts(alert_type);
CREATE INDEX idx_security_alerts_severity ON security_alerts(severity);
CREATE INDEX idx_security_alerts_status ON security_alerts(status);
CREATE INDEX idx_security_alerts_created_at ON security_alerts(created_at);

CREATE INDEX idx_rate_limits_identifier ON rate_limits(identifier);
CREATE INDEX idx_rate_limits_resource_type ON rate_limits(resource_type);
CREATE INDEX idx_rate_limits_window ON rate_limits(window_start, window_end);
CREATE INDEX idx_rate_limits_is_blocked ON rate_limits(is_blocked);
CREATE INDEX idx_rate_limits_block_expires ON rate_limits(block_expires_at);

CREATE INDEX idx_user_devices_user_id ON user_devices(user_id);
CREATE INDEX idx_user_devices_fingerprint ON user_devices(device_fingerprint);
CREATE INDEX idx_user_devices_is_trusted ON user_devices(is_trusted);
CREATE INDEX idx_user_devices_last_seen ON user_devices(last_seen_at);

CREATE INDEX idx_access_patterns_user_id ON access_patterns(user_id);
CREATE INDEX idx_access_patterns_type ON access_patterns(pattern_type);
CREATE INDEX idx_access_patterns_is_anomaly ON access_patterns(is_anomaly);
CREATE INDEX idx_access_patterns_detected_at ON access_patterns(detected_at);

-- Add indexes for new security columns
CREATE INDEX idx_users_failed_login_attempts ON users(failed_login_attempts);
CREATE INDEX idx_users_account_locked_until ON users(account_locked_until);
CREATE INDEX idx_users_risk_score ON users(risk_score);

CREATE INDEX idx_wallets_suspicious_activity ON wallets(suspicious_activity_count);
CREATE INDEX idx_wallets_last_security_scan ON wallets(last_security_scan_at);

CREATE INDEX idx_transactions_risk_level ON transactions(risk_level);
CREATE INDEX idx_transactions_fraud_score ON transactions(fraud_score);
CREATE INDEX idx_transactions_ip_address ON transactions(ip_address);
CREATE INDEX idx_transactions_is_suspicious ON transactions(is_suspicious);
CREATE INDEX idx_transactions_requires_review ON transactions(requires_review);

-- Apply trigger for security alerts
CREATE TRIGGER update_security_alerts_updated_at BEFORE UPDATE ON security_alerts FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add constraints for security features
ALTER TABLE users ADD CONSTRAINT check_failed_login_attempts CHECK (failed_login_attempts >= 0);
ALTER TABLE users ADD CONSTRAINT check_risk_score CHECK (risk_score >= 0 AND risk_score <= 100);

ALTER TABLE wallets ADD CONSTRAINT check_suspicious_activity CHECK (suspicious_activity_count >= 0);

ALTER TABLE transactions ADD CONSTRAINT check_fraud_score CHECK (fraud_score >= 0 AND fraud_score <= 100);
ALTER TABLE transactions ADD CONSTRAINT check_valid_risk_level CHECK (risk_level IN ('low', 'medium', 'high', 'critical'));

ALTER TABLE security_alerts ADD CONSTRAINT check_valid_severity CHECK (severity IN ('low', 'medium', 'high', 'critical'));
ALTER TABLE security_alerts ADD CONSTRAINT check_valid_alert_status CHECK (status IN ('open', 'investigating', 'resolved', 'false_positive'));

ALTER TABLE rate_limits ADD CONSTRAINT check_request_count CHECK (request_count >= 0);
ALTER TABLE rate_limits ADD CONSTRAINT check_max_allowed CHECK (max_allowed > 0);
ALTER TABLE rate_limits ADD CONSTRAINT check_valid_window CHECK (window_end > window_start);

ALTER TABLE user_devices ADD CONSTRAINT check_usage_count CHECK (usage_count >= 0);

ALTER TABLE access_patterns ADD CONSTRAINT check_confidence_score CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0);

-- Create security views
CREATE VIEW high_risk_users AS
SELECT * FROM users 
WHERE risk_score >= 70 
   OR failed_login_attempts >= 5 
   OR account_locked_until > NOW();

CREATE VIEW suspicious_transactions AS
SELECT * FROM transactions 
WHERE is_suspicious = TRUE 
   OR risk_level IN ('high', 'critical')
   OR fraud_score >= 70
   OR requires_review = TRUE;

CREATE VIEW active_security_alerts AS
SELECT * FROM security_alerts 
WHERE status IN ('open', 'investigating')
ORDER BY severity DESC, created_at DESC;

CREATE VIEW rate_limit_violations AS
SELECT * FROM rate_limits 
WHERE request_count > max_allowed 
   OR is_blocked = TRUE
   AND block_expires_at > NOW();

-- Create stored procedures for security operations

-- Procedure to check and update failed login attempts
CREATE OR REPLACE FUNCTION handle_failed_login(p_user_id UUID, p_ip_address INET)
RETURNS BOOLEAN AS $$
DECLARE
    current_attempts INTEGER;
    max_attempts INTEGER := 5;
    lock_duration INTERVAL := '30 minutes';
BEGIN
    -- Update failed login attempts
    UPDATE users 
    SET failed_login_attempts = failed_login_attempts + 1,
        last_failed_login_at = NOW()
    WHERE id = p_user_id
    RETURNING failed_login_attempts INTO current_attempts;
    
    -- Lock account if threshold exceeded
    IF current_attempts >= max_attempts THEN
        UPDATE users 
        SET account_locked_until = NOW() + lock_duration
        WHERE id = p_user_id;
        
        -- Create security alert
        INSERT INTO security_alerts (user_id, alert_type, severity, title, description, alert_data)
        VALUES (p_user_id, 'suspicious_login', 'high', 'Account Locked Due to Failed Logins', 
                FORMAT('Account locked after %s failed login attempts from IP %s', current_attempts, p_ip_address),
                json_build_object('failed_attempts', current_attempts, 'ip_address', p_ip_address));
        
        RETURN TRUE; -- Account locked
    END IF;
    
    RETURN FALSE; -- Account not locked
END;
$$ LANGUAGE plpgsql;

-- Procedure to check transaction risk
CREATE OR REPLACE FUNCTION assess_transaction_risk(p_transaction_id UUID)
RETURNS VOID AS $$
DECLARE
    risk_score INTEGER := 0;
    risk_level VARCHAR(20) := 'low';
    transaction_record RECORD;
BEGIN
    -- Get transaction details
    SELECT * INTO transaction_record FROM transactions WHERE id = p_transaction_id;
    
    IF NOT FOUND THEN
        RETURN;
    END IF;
    
    -- Assess risk based on various factors
    -- High amount risk
    IF transaction_record.amount_lumens > 10000 THEN
        risk_score := risk_score + 30;
    ELSIF transaction_record.amount_lumens > 1000 THEN
        risk_score := risk_score + 15;
    END IF;
    
    -- New wallet risk (simplified check)
    IF (SELECT COUNT(*) FROM transactions WHERE from_wallet_id = transaction_record.from_wallet_id) <= 3 THEN
        risk_score := risk_score + 20;
    END IF;
    
    -- Suspicious IP (placeholder logic)
    IF transaction_record.ip_address IS NOT NULL THEN
        -- Add IP-based risk assessment logic here
        NULL;
    END IF;
    
    -- Determine risk level
    IF risk_score >= 70 THEN
        risk_level := 'critical';
    ELSIF risk_score >= 50 THEN
        risk_level := 'high';
    ELSIF risk_score >= 30 THEN
        risk_level := 'medium';
    END IF;
    
    -- Update transaction with risk assessment
    UPDATE transactions 
    SET risk_level = risk_level,
        fraud_score = risk_score,
        is_suspicious = (risk_score >= 50),
        requires_review = (risk_score >= 70)
    WHERE id = p_transaction_id;
    
    -- Create security alert for high-risk transactions
    IF risk_score >= 50 THEN
        INSERT INTO security_alerts (user_id, transaction_id, alert_type, severity, title, description, alert_data)
        VALUES (
            transaction_record.user_id,
            p_transaction_id,
            'suspicious_transaction',
            CASE WHEN risk_score >= 70 THEN 'critical' ELSE 'high' END,
            'High-Risk Transaction Detected',
            FORMAT('Transaction flagged with risk score %s (%s)', risk_score, risk_level),
            json_build_object('risk_score', risk_score, 'risk_level', risk_level, 'amount', transaction_record.amount_lumens)
        );
    END IF;
END;
$$ LANGUAGE plpgsql;
