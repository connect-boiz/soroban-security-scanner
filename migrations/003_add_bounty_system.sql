-- Migration 003: Add Bounty System and Escrow Features
-- This migration adds tables for the bounty marketplace, escrow system, and related functionality

-- Bounty categories
CREATE TYPE bounty_category AS ENUM (
    'access_control', 'token_economics', 'logic_vulnerability', 
    'gas_optimization', 'event_logging', 'randomness', 
    'stellar_specific', 'performance', 'documentation', 'other'
);

-- Bounty severity levels
CREATE TYPE bounty_severity AS ENUM ('critical', 'high', 'medium', 'low', 'informational');

-- Bounty status
CREATE TYPE bounty_status AS ENUM (
    'draft', 'open', 'assigned', 'submitted', 'under_review', 
    'accepted', 'rejected', 'paid', 'disputed', 'closed'
);

-- Escrow status
CREATE TYPE escrow_status AS ENUM ('pending', 'funded', 'released', 'refunded', 'disputed');

-- Projects table (for organizing bounties)
CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    repository_url VARCHAR(500),
    contract_address VARCHAR(56), -- Stellar contract address
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_active BOOLEAN DEFAULT TRUE,
    is_public BOOLEAN DEFAULT TRUE,
    total_budget DECIMAL(19, 7) DEFAULT 0,
    budget_currency VARCHAR(10) DEFAULT 'XLM',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Bounties table
CREATE TABLE bounties (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID REFERENCES projects(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    category bounty_category NOT NULL,
    severity bounty_severity NOT NULL,
    status bounty_status NOT NULL DEFAULT 'draft',
    reward_amount DECIMAL(19, 7) NOT NULL,
    reward_currency VARCHAR(10) DEFAULT 'XLM',
    max_reward_amount DECIMAL(19, 7), -- For variable rewards
    assignee_id UUID REFERENCES users(id) ON DELETE SET NULL,
    submitter_id UUID REFERENCES users(id) ON DELETE SET NULL,
    reviewer_id UUID REFERENCES users(id) ON DELETE SET NULL,
    deadline TIMESTAMP WITH TIME ZONE,
    requirements JSONB DEFAULT '{}',
    submission_guidelines TEXT,
    evaluation_criteria JSONB DEFAULT '{}',
    tags JSONB DEFAULT '[]',
    view_count INTEGER DEFAULT 0,
    applicant_count INTEGER DEFAULT 0,
    submission_count INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    submitted_at TIMESTAMP WITH TIME ZONE,
    reviewed_at TIMESTAMP WITH TIME ZONE,
    accepted_at TIMESTAMP WITH TIME ZONE,
    paid_at TIMESTAMP WITH TIME ZONE,
    metadata JSONB DEFAULT '{}'
);

-- Bounty applications table
CREATE TABLE bounty_applications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bounty_id UUID NOT NULL REFERENCES bounties(id) ON DELETE CASCADE,
    applicant_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    cover_letter TEXT,
    proposed_solution TEXT,
    estimated_completion_time INTERVAL,
    proposed_budget DECIMAL(19, 7),
    attachments JSONB DEFAULT '[]',
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, accepted, rejected
    reviewer_notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(bounty_id, applicant_id)
);

-- Bounty submissions table
CREATE TABLE bounty_submissions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bounty_id UUID NOT NULL REFERENCES bounties(id) ON DELETE CASCADE,
    submitter_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255),
    description TEXT NOT NULL,
    vulnerability_details JSONB NOT NULL,
    proof_of_concept TEXT,
    reproduction_steps TEXT,
    impact_assessment TEXT,
    recommended_fix TEXT,
    attachments JSONB DEFAULT '[]',
    code_snippets JSONB DEFAULT '{}',
    test_cases JSONB DEFAULT '{}',
    severity_suggestion bounty_severity,
    status VARCHAR(20) NOT NULL DEFAULT 'submitted', -- submitted, under_review, accepted, rejected
    review_score INTEGER, -- 1-10 quality score
    review_feedback TEXT,
    public_visibility BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Escrow accounts table
CREATE TABLE escrow_accounts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bounty_id UUID REFERENCES bounties(id) ON DELETE CASCADE,
    funder_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    beneficiary_id UUID REFERENCES users(id) ON DELETE SET NULL,
    amount DECIMAL(19, 7) NOT NULL,
    currency VARCHAR(10) DEFAULT 'XLM',
    status escrow_status NOT NULL DEFAULT 'pending',
    release_conditions JSONB DEFAULT '{}',
    dispute_reason TEXT,
    dispute_evidence JSONB DEFAULT '{}',
    stellar_transaction_hash VARCHAR(64), -- Funding transaction
    release_transaction_hash VARCHAR(64), -- Release transaction
    refund_transaction_hash VARCHAR(64), -- Refund transaction
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    funded_at TIMESTAMP WITH TIME ZONE,
    released_at TIMESTAMP WITH TIME ZONE,
    refunded_at TIMESTAMP WITH TIME ZONE,
    disputed_at TIMESTAMP WITH TIME ZONE,
    resolved_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE,
    metadata JSONB DEFAULT '{}'
);

-- Bounty reviews table
CREATE TABLE bounty_reviews (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bounty_id UUID NOT NULL REFERENCES bounties(id) ON DELETE CASCADE,
    submission_id UUID NOT NULL REFERENCES bounty_submissions(id) ON DELETE CASCADE,
    reviewer_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    overall_score INTEGER NOT NULL CHECK (overall_score >= 1 AND overall_score <= 10),
    severity_rating bounty_severity,
    quality_score INTEGER CHECK (quality_score >= 1 AND quality_score <= 10),
    impact_score INTEGER CHECK (impact_score >= 1 AND impact_score <= 10),
    originality_score INTEGER CHECK (originality_score >= 1 AND originality_score <= 10),
    review_comments TEXT,
    recommendation VARCHAR(20) NOT NULL, -- accept, reject, request_changes
    review_criteria JSONB DEFAULT '{}',
    status VARCHAR(20) NOT NULL DEFAULT 'draft', -- draft, submitted, final
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    finalized_at TIMESTAMP WITH TIME ZONE,
    UNIQUE(bounty_id, submission_id, reviewer_id)
);

-- Bounty payments table
CREATE TABLE bounty_payments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bounty_id UUID NOT NULL REFERENCES bounties(id) ON DELETE CASCADE,
    submission_id UUID NOT NULL REFERENCES bounty_submissions(id) ON DELETE CASCADE,
    recipient_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount DECIMAL(19, 7) NOT NULL,
    currency VARCHAR(10) DEFAULT 'XLM',
    payment_method VARCHAR(50) DEFAULT 'stellar', -- stellar, bank_transfer, crypto
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, processing, completed, failed, cancelled
    transaction_hash VARCHAR(64), -- Stellar transaction hash
    payment_details JSONB DEFAULT '{}',
    fee_amount DECIMAL(19, 7) DEFAULT 0,
    fee_currency VARCHAR(10) DEFAULT 'XLM',
    net_amount DECIMAL(19, 7),
    due_date TIMESTAMP WITH TIME ZONE,
    processed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Bounty activity log table
CREATE TABLE bounty_activity_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bounty_id UUID REFERENCES bounties(id) ON DELETE CASCADE,
    submission_id UUID REFERENCES bounty_submissions(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR(50) NOT NULL, -- created, updated, applied, submitted, reviewed, paid, etc.
    description TEXT NOT NULL,
    old_values JSONB,
    new_values JSONB,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Bounty tags table (for better categorization and search)
CREATE TABLE bounty_tags (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(50) UNIQUE NOT NULL,
    description TEXT,
    color VARCHAR(7), -- Hex color code
    usage_count INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Bounty-tag relationships table
CREATE TABLE bounty_tag_relations (
    bounty_id UUID NOT NULL REFERENCES bounties(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES bounty_tags(id) ON DELETE CASCADE,
    PRIMARY KEY (bounty_id, tag_id)
);

-- Create indexes for bounty system tables

-- Projects indexes
CREATE INDEX idx_projects_owner_id ON projects(owner_id);
CREATE INDEX idx_projects_is_active ON projects(is_active);
CREATE INDEX idx_projects_is_public ON projects(is_public);
CREATE INDEX idx_projects_created_at ON projects(created_at);

-- Bounties indexes
CREATE INDEX idx_bounties_project_id ON bounties(project_id);
CREATE INDEX idx_bounties_category ON bounties(category);
CREATE INDEX idx_bounties_severity ON bounties(severity);
CREATE INDEX idx_bounties_status ON bounties(status);
CREATE INDEX idx_bounties_assignee_id ON bounties(assignee_id);
CREATE INDEX idx_bounties_submitter_id ON bounties(submitter_id);
CREATE INDEX idx_bounties_reviewer_id ON bounties(reviewer_id);
CREATE INDEX idx_bounties_reward_amount ON bounties(reward_amount);
CREATE INDEX idx_bounties_deadline ON bounties(deadline);
CREATE INDEX idx_bounties_created_at ON bounties(created_at);
CREATE INDEX idx_bounties_status_reward ON bounties(status, reward_amount);

-- Bounty applications indexes
CREATE INDEX idx_bounty_applications_bounty_id ON bounty_applications(bounty_id);
CREATE INDEX idx_bounty_applications_applicant_id ON bounty_applications(applicant_id);
CREATE INDEX idx_bounty_applications_status ON bounty_applications(status);
CREATE INDEX idx_bounty_applications_created_at ON bounty_applications(created_at);

-- Bounty submissions indexes
CREATE INDEX idx_bounty_submissions_bounty_id ON bounty_submissions(bounty_id);
CREATE INDEX idx_bounty_submissions_submitter_id ON bounty_submissions(submitter_id);
CREATE INDEX idx_bounty_submissions_status ON bounty_submissions(status);
CREATE INDEX idx_bounty_submissions_severity ON bounty_submissions(severity_suggestion);
CREATE INDEX idx_bounty_submissions_created_at ON bounty_submissions(created_at);

-- Escrow accounts indexes
CREATE INDEX idx_escrow_bounty_id ON escrow_accounts(bounty_id);
CREATE INDEX idx_escrow_funder_id ON escrow_accounts(funder_id);
CREATE INDEX idx_escrow_beneficiary_id ON escrow_accounts(beneficiary_id);
CREATE INDEX idx_escrow_status ON escrow_accounts(status);
CREATE INDEX idx_escrow_amount ON escrow_accounts(amount);
CREATE INDEX idx_escrow_created_at ON escrow_accounts(created_at);
CREATE INDEX idx_escrow_expires_at ON escrow_accounts(expires_at);

-- Bounty reviews indexes
CREATE INDEX idx_bounty_reviews_bounty_id ON bounty_reviews(bounty_id);
CREATE INDEX idx_bounty_reviews_submission_id ON bounty_reviews(submission_id);
CREATE INDEX idx_bounty_reviews_reviewer_id ON bounty_reviews(reviewer_id);
CREATE INDEX idx_bounty_reviews_status ON bounty_reviews(status);
CREATE INDEX idx_bounty_reviews_overall_score ON bounty_reviews(overall_score);
CREATE INDEX idx_bounty_reviews_created_at ON bounty_reviews(created_at);

-- Bounty payments indexes
CREATE INDEX idx_bounty_payments_bounty_id ON bounty_payments(bounty_id);
CREATE INDEX idx_bounty_payments_submission_id ON bounty_payments(submission_id);
CREATE INDEX idx_bounty_payments_recipient_id ON bounty_payments(recipient_id);
CREATE INDEX idx_bounty_payments_status ON bounty_payments(status);
CREATE INDEX idx_bounty_payments_amount ON bounty_payments(amount);
CREATE INDEX idx_bounty_payments_due_date ON bounty_payments(due_date);
CREATE INDEX idx_bounty_payments_created_at ON bounty_payments(created_at);

-- Bounty activity log indexes
CREATE INDEX idx_bounty_activity_bounty_id ON bounty_activity_log(bounty_id);
CREATE INDEX idx_bounty_activity_submission_id ON bounty_activity_log(submission_id);
CREATE INDEX idx_bounty_activity_user_id ON bounty_activity_log(user_id);
CREATE INDEX idx_bounty_activity_action ON bounty_activity_log(action);
CREATE INDEX idx_bounty_activity_created_at ON bounty_activity_log(created_at);

-- Apply triggers for timestamp updates
CREATE TRIGGER update_projects_updated_at BEFORE UPDATE ON projects FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_bounties_updated_at BEFORE UPDATE ON bounties FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_bounty_applications_updated_at BEFORE UPDATE ON bounty_applications FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_bounty_submissions_updated_at BEFORE UPDATE ON bounty_submissions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_escrow_accounts_updated_at BEFORE UPDATE ON escrow_accounts FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_bounty_reviews_updated_at BEFORE UPDATE ON bounty_reviews FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_bounty_payments_updated_at BEFORE UPDATE ON bounty_payments FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add constraints
ALTER TABLE bounties ADD CONSTRAINT check_positive_reward CHECK (reward_amount > 0);
ALTER TABLE bounties ADD CONSTRAINT check_valid_max_reward CHECK (max_reward_amount IS NULL OR max_reward_amount >= reward_amount);
ALTER TABLE bounties ADD CONSTRAINT check_deadline_future CHECK (deadline IS NULL OR deadline > created_at);

ALTER TABLE bounty_applications ADD CONSTRAINT check_positive_proposed_budget CHECK (proposed_budget IS NULL OR proposed_budget > 0);

ALTER TABLE escrow_accounts ADD CONSTRAINT check_positive_escrow_amount CHECK (amount > 0);
ALTER TABLE escrow_accounts ADD CONSTRAINT check_valid_expires_at CHECK (expires_at IS NULL OR expires_at > created_at);

ALTER TABLE bounty_reviews ADD CONSTRAINT check_quality_score_range CHECK (quality_score IS NULL OR (quality_score >= 1 AND quality_score <= 10));
ALTER TABLE bounty_reviews ADD CONSTRAINT check_impact_score_range CHECK (impact_score IS NULL OR (impact_score >= 1 AND impact_score <= 10));
ALTER TABLE bounty_reviews ADD CONSTRAINT check_originality_score_range CHECK (originality_score IS NULL OR (originality_score >= 1 AND originality_score <= 10));

ALTER TABLE bounty_payments ADD CONSTRAINT check_positive_payment_amount CHECK (amount > 0);
ALTER TABLE check_positive_fee_amount CHECK (fee_amount >= 0);
ALTER TABLE bounty_payments ADD CONSTRAINT check_positive_net_amount CHECK (net_amount > 0);
ALTER TABLE bounty_payments ADD CONSTRAINT check_net_amount_calculation CHECK (net_amount = amount - fee_amount);

-- Create views for bounty system

-- Active bounties view
CREATE VIEW active_bounties AS
SELECT b.*, p.name as project_name, p.owner_id as project_owner_id
FROM bounties b
LEFT JOIN projects p ON b.project_id = p.id
WHERE b.status IN ('open', 'assigned')
  AND (b.deadline IS NULL OR b.deadline > NOW())
  AND (p.is_active = TRUE OR p.id IS NULL);

-- User bounty statistics view
CREATE VIEW user_bounty_stats AS
SELECT 
    u.id as user_id,
    u.username,
    COUNT(DISTINCT CASE WHEN b.assignee_id = u.id THEN b.id END) as assigned_bounties,
    COUNT(DISTINCT CASE WHEN bs.submitter_id = u.id THEN bs.id END) as submissions,
    COUNT(DISTINCT CASE WHEN bp.recipient_id = u.id AND bp.status = 'completed' THEN bp.id END) as completed_payments,
    COALESCE(SUM(CASE WHEN bp.recipient_id = u.id AND bp.status = 'completed' THEN bp.net_amount END), 0) as total_earned,
    AVG(CASE WHEN br.overall_score IS NOT NULL THEN br.overall_score END) as average_review_score
FROM users u
LEFT JOIN bounties b ON u.id = b.assignee_id
LEFT JOIN bounty_submissions bs ON u.id = bs.submitter_id
LEFT JOIN bounty_payments bp ON u.id = bp.recipient_id
LEFT JOIN bounty_reviews br ON bs.id = br.submission_id
GROUP BY u.id, u.username;

-- Bounty analytics view
CREATE VIEW bounty_analytics AS
SELECT 
    DATE_TRUNC('month', b.created_at) as month,
    COUNT(*) as total_bounties,
    COUNT(CASE WHEN b.status = 'open' THEN 1 END) as open_bounties,
    COUNT(CASE WHEN b.status = 'closed' THEN 1 END) as closed_bounties,
    COUNT(CASE WHEN b.status = 'paid' THEN 1 END) as paid_bounties,
    COALESCE(SUM(b.reward_amount), 0) as total_rewards_offered,
    COALESCE(SUM(CASE WHEN b.status = 'paid' THEN b.reward_amount END), 0) as total_rewards_paid,
    AVG(b.reward_amount) as average_reward,
    COUNT(DISTINCT b.project_id) as active_projects
FROM bounties b
GROUP BY DATE_TRUNC('month', b.created_at)
ORDER BY month DESC;

-- Escrow summary view
CREATE VIEW escrow_summary AS
SELECT 
    status,
    COUNT(*) as count,
    COALESCE(SUM(amount), 0) as total_amount,
    AVG(amount) as average_amount
FROM escrow_accounts
GROUP BY status;

-- Create stored procedures for bounty operations

-- Procedure to create and fund escrow for bounty
CREATE OR REPLACE FUNCTION create_bounty_escrow(
    p_bounty_id UUID,
    p_funder_id UUID,
    p_amount DECIMAL(19, 7),
    p_currency VARCHAR(10) DEFAULT 'XLM',
    p_expires_at TIMESTAMP WITH TIME ZONE DEFAULT NULL
) RETURNS UUID AS $$
DECLARE
    escrow_id UUID;
BEGIN
    -- Create escrow account
    INSERT INTO escrow_accounts (bounty_id, funder_id, amount, currency, expires_at)
    VALUES (p_bounty_id, p_funder_id, p_amount, p_currency, p_expires_at)
    RETURNING id INTO escrow_id;
    
    -- Update bounty status to open if it was draft
    UPDATE bounties 
    SET status = 'open'
    WHERE id = p_bounty_id AND status = 'draft';
    
    -- Log activity
    INSERT INTO bounty_activity_log (bounty_id, user_id, action, description)
    VALUES (p_bounty_id, p_funder_id, 'escrow_created', 
            FORMAT('Escrow created with %s %s', p_amount, p_currency));
    
    RETURN escrow_id;
END;
$$ LANGUAGE plpgsql;

-- Procedure to release escrow payment
CREATE OR REPLACE FUNCTION release_escrow_payment(
    p_escrow_id UUID,
    p_beneficiary_id UUID,
    p_transaction_hash VARCHAR(64),
    p_reviewer_id UUID DEFAULT NULL
) RETURNS BOOLEAN AS $$
DECLARE
    escrow_record RECORD;
    bounty_record RECORD;
BEGIN
    -- Get escrow details
    SELECT * INTO escrow_record FROM escrow_accounts WHERE id = p_escrow_id;
    
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Escrow account not found';
    END IF;
    
    -- Check if escrow can be released
    IF escrow_record.status != 'funded' THEN
        RAISE EXCEPTION 'Escrow is not in funded status';
    END IF;
    
    -- Update escrow
    UPDATE escrow_accounts 
    SET status = 'released',
        beneficiary_id = p_beneficiary_id,
        release_transaction_hash = p_transaction_hash,
        released_at = NOW()
    WHERE id = p_escrow_id;
    
    -- Get bounty details for activity log
    SELECT * INTO bounty_record FROM bounties WHERE id = escrow_record.bounty_id;
    
    -- Create payment record
    INSERT INTO bounty_payments (bounty_id, recipient_id, amount, currency, transaction_hash, status, processed_at)
    VALUES (escrow_record.bounty_id, p_beneficiary_id, escrow_record.amount, escrow_record.currency, p_transaction_hash, 'completed', NOW());
    
    -- Update bounty status
    UPDATE bounties 
    SET status = 'paid',
        paid_at = NOW()
    WHERE id = escrow_record.bounty_id;
    
    -- Log activity
    INSERT INTO bounty_activity_log (bounty_id, user_id, action, description, metadata)
    VALUES (escrow_record.bounty_id, p_reviewer_id, 'escrow_released', 
            FORMAT('Escrow released to beneficiary %s', p_beneficiary_id),
            json_build_object('escrow_id', p_escrow_id, 'amount', escrow_record.amount));
    
    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;
