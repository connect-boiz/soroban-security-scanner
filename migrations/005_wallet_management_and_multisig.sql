-- Migration 005: Wallet Management Service & Multi-Signature Business Logic
-- Adds wallet sync records and enhances multi-sig proposal tracking

-- ============================================================
-- Wallet sync records (cross-device synchronization)
-- ============================================================

CREATE TABLE wallet_sync_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id VARCHAR(255) NOT NULL,
    device_name VARCHAR(100),
    last_synced_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    -- AES-256-GCM encrypted wallet state delta, base64-encoded
    encrypted_state TEXT NOT NULL,
    -- Monotonically increasing version for conflict detection
    sync_version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(wallet_id, device_id)
);

CREATE INDEX idx_wallet_sync_wallet_id ON wallet_sync_records(wallet_id);
CREATE INDEX idx_wallet_sync_user_id ON wallet_sync_records(user_id);
CREATE INDEX idx_wallet_sync_device_id ON wallet_sync_records(device_id);

-- ============================================================
-- Per-user HMAC keys for wallet export integrity
-- ============================================================

CREATE TABLE user_wallet_hmac_keys (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    -- HMAC-SHA256 key, stored encrypted at rest (application-level encryption)
    hmac_key_encrypted TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- ============================================================
-- Wallet export audit log
-- ============================================================

CREATE TABLE wallet_export_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    exported_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    ip_address INET,
    user_agent TEXT
);

CREATE INDEX idx_wallet_export_log_wallet ON wallet_export_log(wallet_id);
CREATE INDEX idx_wallet_export_log_user ON wallet_export_log(user_id);

-- ============================================================
-- Multi-signature proposals (enhanced)
-- ============================================================

-- Add cumulative weight columns to existing multi_signature_operations
ALTER TABLE multi_signature_operations
    ADD COLUMN IF NOT EXISTS threshold_weight INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS total_signer_weight INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS approved_weight INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS rejected_weight INTEGER NOT NULL DEFAULT 0;

-- Add weight column to existing multi_signature_signers
ALTER TABLE multi_signature_signers
    ADD COLUMN IF NOT EXISTS decision VARCHAR(20) NOT NULL DEFAULT 'pending';

-- ============================================================
-- Multi-sig proposal activity log
-- ============================================================

CREATE TABLE multi_sig_activity_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    proposal_id UUID NOT NULL REFERENCES multi_signature_operations(id) ON DELETE CASCADE,
    actor_address VARCHAR(56),
    actor_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR(50) NOT NULL, -- created, approved, rejected, executed, expired, threshold_updated
    old_status VARCHAR(20),
    new_status VARCHAR(20),
    details JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_multisig_activity_proposal ON multi_sig_activity_log(proposal_id);
CREATE INDEX idx_multisig_activity_actor ON multi_sig_activity_log(actor_user_id);
CREATE INDEX idx_multisig_activity_created ON multi_sig_activity_log(created_at);

-- ============================================================
-- Trigger: auto-update multi_sig approved/rejected weight
-- ============================================================

CREATE OR REPLACE FUNCTION update_multisig_weights()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE multi_signature_operations
    SET
        approved_weight = (
            SELECT COALESCE(SUM(weight), 0)
            FROM multi_signature_signers
            WHERE multi_sig_operation_id = NEW.multi_sig_operation_id
              AND decision = 'approved'
        ),
        rejected_weight = (
            SELECT COALESCE(SUM(weight), 0)
            FROM multi_signature_signers
            WHERE multi_sig_operation_id = NEW.multi_sig_operation_id
              AND decision = 'rejected'
        ),
        updated_at = NOW()
    WHERE id = NEW.multi_sig_operation_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_update_multisig_weights
AFTER INSERT OR UPDATE OF decision ON multi_signature_signers
FOR EACH ROW EXECUTE FUNCTION update_multisig_weights();
