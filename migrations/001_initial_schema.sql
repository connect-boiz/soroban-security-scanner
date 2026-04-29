-- Initial Database Schema for Soroban Security Scanner
-- This migration creates the core tables for users, wallets, transactions, and multi-signature operations

-- Enable UUID extension for UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create custom types for enums
CREATE TYPE user_role AS ENUM ('admin', 'security_researcher', 'developer', 'auditor', 'user');
CREATE TYPE user_status AS ENUM ('active', 'inactive', 'suspended', 'pending_verification');
CREATE TYPE wallet_status AS ENUM ('active', 'inactive', 'frozen', 'compromised');
CREATE TYPE transaction_type AS ENUM ('scan_payment', 'bounty_payment', 'escrow_deposit', 'escrow_release', 'multi_sig_execution', 'fee_payment', 'refund');
CREATE TYPE transaction_status AS ENUM ('pending', 'confirmed', 'failed', 'cancelled', 'processing');
CREATE_TYPE multi_sig_status AS ENUM ('pending', 'approved', 'rejected', 'executed', 'expired');
CREATE TYPE signature_status AS ENUM ('pending', 'signed', 'rejected');

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    username VARCHAR(100) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    stellar_address VARCHAR(56) UNIQUE, -- Stellar public key format
    role user_role NOT NULL DEFAULT 'user',
    status user_status NOT NULL DEFAULT 'pending_verification',
    email_verified BOOLEAN DEFAULT FALSE,
    two_factor_enabled BOOLEAN DEFAULT FALSE,
    two_factor_secret VARCHAR(32),
    profile JSONB DEFAULT '{}', -- Store additional profile data
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_login_at TIMESTAMP WITH TIME ZONE,
    login_count INTEGER DEFAULT 0,
    reputation_score INTEGER DEFAULT 0,
    is_verified BOOLEAN DEFAULT FALSE,
    verification_token VARCHAR(255),
    password_reset_token VARCHAR(255),
    password_reset_expires TIMESTAMP WITH TIME ZONE
);

-- Wallets table
CREATE TABLE wallets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stellar_address VARCHAR(56) UNIQUE NOT NULL,
    wallet_name VARCHAR(100) NOT NULL,
    description TEXT,
    wallet_type VARCHAR(50) NOT NULL DEFAULT 'standard', -- standard, hardware, multisig, etc.
    status wallet_status NOT NULL DEFAULT 'active',
    balance_lumens DECIMAL(19, 7) DEFAULT 0, -- XLM balance with 7 decimal places
    native_balance DECIMAL(19, 7) DEFAULT 0, -- Native token balance
    is_primary BOOLEAN DEFAULT FALSE,
    is_verified BOOLEAN DEFAULT FALSE,
    verification_level INTEGER DEFAULT 0, -- 0=unverified, 1=basic, 2=enhanced, 3=premium
    metadata JSONB DEFAULT '{}', -- Additional wallet metadata
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_transaction_at TIMESTAMP WITH TIME ZONE,
    transaction_count INTEGER DEFAULT 0,
    frozen_reason TEXT,
    security_score INTEGER DEFAULT 0 -- 0-100 security rating
);

-- Transactions table
CREATE TABLE transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_hash VARCHAR(64) UNIQUE NOT NULL, -- Stellar transaction hash
    from_wallet_id UUID REFERENCES wallets(id) ON DELETE SET NULL,
    to_wallet_id UUID REFERENCES wallets(id) ON DELETE SET NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    transaction_type transaction_type NOT NULL,
    status transaction_status NOT NULL DEFAULT 'pending',
    amount_lumens DECIMAL(19, 7),
    amount_native DECIMAL(19, 7),
    fee_paid DECIMAL(19, 7) DEFAULT 0,
    memo TEXT,
    memo_type VARCHAR(20), -- text, id, hash, return
    stellar_ledger_sequence BIGINT,
    stellar_operation_count INTEGER DEFAULT 1,
    envelope JSONB NOT NULL, -- Full Stellar transaction envelope
    result JSONB, -- Transaction result from Stellar
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    confirmed_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE,
    related_scan_id UUID, -- Reference to security scan if applicable
    related_bounty_id UUID, -- Reference to bounty if applicable
    batch_transaction_id UUID, -- For batch operations
    metadata JSONB DEFAULT '{}'
);

-- Multi-signature operations table
CREATE TABLE multi_signature_operations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    operation_name VARCHAR(255) NOT NULL,
    description TEXT,
    stellar_address VARCHAR(56) NOT NULL, -- The multi-signature account address
    threshold_signers INTEGER NOT NULL DEFAULT 1,
    total_signers INTEGER NOT NULL DEFAULT 1,
    status multi_sig_status NOT NULL DEFAULT 'pending',
    transaction_envelope JSONB NOT NULL, -- The transaction envelope to be signed
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE,
    executed_at TIMESTAMP WITH TIME ZONE,
    executed_transaction_hash VARCHAR(64), -- Hash of the executed transaction
    metadata JSONB DEFAULT '{}'
);

-- Multi-signature signers table
CREATE TABLE multi_signature_signers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    multi_sig_operation_id UUID NOT NULL REFERENCES multi_signature_operations(id) ON DELETE CASCADE,
    signer_address VARCHAR(56) NOT NULL,
    signer_wallet_id UUID REFERENCES wallets(id) ON DELETE SET NULL,
    signer_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    weight INTEGER NOT NULL DEFAULT 1,
    status signature_status NOT NULL DEFAULT 'pending',
    signature_data TEXT, -- Base64 encoded signature
    signed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    comments TEXT,
    UNIQUE(multi_sig_operation_id, signer_address)
);

-- Transaction signatures table (for tracking all signatures on transactions)
CREATE TABLE transaction_signatures (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
    signer_address VARCHAR(56) NOT NULL,
    signature_data TEXT NOT NULL, -- Base64 encoded signature
    signature_type VARCHAR(50) NOT NULL DEFAULT 'stellar', -- stellar, ed25519, etc.
    weight INTEGER DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(transaction_id, signer_address)
);

-- User sessions table for authentication
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token VARCHAR(255) UNIQUE NOT NULL,
    refresh_token VARCHAR(255) UNIQUE,
    ip_address INET,
    user_agent TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    last_accessed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Audit log table for tracking all important operations
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(50) NOT NULL, -- user, wallet, transaction, multi_sig
    resource_id UUID,
    old_values JSONB,
    new_values JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'
);

-- Create indexes for performance optimization

-- Users table indexes
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_stellar_address ON users(stellar_address);
CREATE INDEX idx_users_role ON users(role);
CREATE INDEX idx_users_status ON users(status);
CREATE INDEX idx_users_created_at ON users(created_at);

-- Wallets table indexes
CREATE INDEX idx_wallets_user_id ON wallets(user_id);
CREATE INDEX idx_wallets_stellar_address ON wallets(stellar_address);
CREATE INDEX idx_wallets_status ON wallets(status);
CREATE INDEX idx_wallets_type ON wallets(wallet_type);
CREATE INDEX idx_wallets_created_at ON wallets(created_at);
CREATE INDEX idx_wallets_user_primary ON wallets(user_id, is_primary);

-- Transactions table indexes
CREATE INDEX idx_transactions_transaction_hash ON transactions(transaction_hash);
CREATE INDEX idx_transactions_from_wallet_id ON transactions(from_wallet_id);
CREATE INDEX idx_transactions_to_wallet_id ON transactions(to_wallet_id);
CREATE INDEX idx_transactions_user_id ON transactions(user_id);
CREATE INDEX idx_transactions_type ON transactions(transaction_type);
CREATE INDEX idx_transactions_status ON transactions(status);
CREATE INDEX idx_transactions_created_at ON transactions(created_at);
CREATE INDEX idx_transactions_confirmed_at ON transactions(confirmed_at);
CREATE INDEX idx_transactions_stellar_ledger ON transactions(stellar_ledger_sequence);
CREATE INDEX idx_transactions_batch_id ON transactions(batch_transaction_id);
CREATE INDEX idx_transactions_related_scan ON transactions(related_scan_id);
CREATE INDEX idx_transactions_related_bounty ON transactions(related_bounty_id);

-- Multi-signature operations indexes
CREATE INDEX idx_multi_sig_user_id ON multi_signature_operations(user_id);
CREATE INDEX idx_multi_sig_stellar_address ON multi_signature_operations(stellar_address);
CREATE INDEX idx_multi_sig_status ON multi_signature_operations(status);
CREATE INDEX idx_multi_sig_created_at ON multi_signature_operations(created_at);
CREATE INDEX idx_multi_sig_expires_at ON multi_signature_operations(expires_at);

-- Multi-signature signers indexes
CREATE INDEX idx_multi_sig_signers_operation_id ON multi_signature_signers(multi_sig_operation_id);
CREATE INDEX idx_multi_sig_signers_address ON multi_signature_signers(signer_address);
CREATE INDEX idx_multi_sig_signers_status ON multi_signature_signers(status);
CREATE INDEX idx_multi_sig_signers_user_id ON multi_signature_signers(signer_user_id);

-- Transaction signatures indexes
CREATE INDEX idx_transaction_signatures_transaction_id ON transaction_signatures(transaction_id);
CREATE INDEX idx_transaction_signatures_signer_address ON transaction_signatures(signer_address);

-- User sessions indexes
CREATE INDEX idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX idx_user_sessions_token ON user_sessions(session_token);
CREATE INDEX idx_user_sessions_refresh_token ON user_sessions(refresh_token);
CREATE INDEX idx_user_sessions_expires_at ON user_sessions(expires_at);

-- Audit logs indexes
CREATE INDEX idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);

-- Create triggers for automatic timestamp updates
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply triggers to tables with updated_at columns
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_wallets_updated_at BEFORE UPDATE ON wallets FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_transactions_updated_at BEFORE UPDATE ON transactions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_multi_sig_operations_updated_at BEFORE UPDATE ON multi_signature_operations FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_multi_sig_signers_updated_at BEFORE UPDATE ON multi_signature_signers FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Create constraints and additional validations

-- Ensure only one primary wallet per user
ALTER TABLE wallets ADD CONSTRAINT unique_primary_wallet_per_user 
    EXCLUDE (user_id WITH =) WHERE (is_primary = TRUE);

-- Ensure transaction amounts are non-negative
ALTER TABLE transactions ADD CONSTRAINT check_positive_lumen_amount CHECK (amount_lumens >= 0);
ALTER TABLE transactions ADD CONSTRAINT check_positive_native_amount CHECK (amount_native >= 0);
ALTER TABLE transactions ADD CONSTRAINT check_positive_fee CHECK (fee_paid >= 0);

-- Ensure wallet balances are non-negative
ALTER TABLE wallets ADD CONSTRAINT check_non_negative_lumen_balance CHECK (balance_lumens >= 0);
ALTER TABLE wallets ADD CONSTRAINT check_non_negative_native_balance CHECK (native_balance >= 0);

-- Ensure multi-signature thresholds are valid
ALTER TABLE multi_signature_operations ADD CONSTRAINT check_valid_threshold 
    CHECK (threshold_signers >= 1 AND threshold_signers <= total_signers);

-- Ensure signer weights are positive
ALTER TABLE multi_signature_signers ADD CONSTRAINT check_positive_weight CHECK (weight > 0);

-- Create views for common queries

-- Active users view
CREATE VIEW active_users AS
SELECT * FROM users WHERE status = 'active' AND email_verified = TRUE;

-- User wallet summary view
CREATE VIEW user_wallet_summary AS
SELECT 
    u.id as user_id,
    u.username,
    u.email,
    COUNT(w.id) as wallet_count,
    COALESCE(SUM(w.balance_lumens), 0) as total_lumen_balance,
    COALESCE(SUM(w.native_balance), 0) as total_native_balance,
    MAX(w.created_at) as latest_wallet_created
FROM users u
LEFT JOIN wallets w ON u.id = w.user_id
WHERE u.status = 'active'
GROUP BY u.id, u.username, u.email;

-- Transaction summary view
CREATE VIEW transaction_summary AS
SELECT 
    DATE_TRUNC('day', created_at) as transaction_date,
    transaction_type,
    status,
    COUNT(*) as transaction_count,
    COALESCE(SUM(amount_lumens), 0) as total_lumen_volume,
    COALESCE(SUM(amount_native), 0) as total_native_volume,
    COALESCE(SUM(fee_paid), 0) as total_fees
FROM transactions
GROUP BY DATE_TRUNC('day', created_at), transaction_type, status
ORDER BY transaction_date DESC;
