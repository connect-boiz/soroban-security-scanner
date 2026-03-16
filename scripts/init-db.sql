-- Initialize Soroban Security Scanner Database

-- Create extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(50) DEFAULT 'user',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Scans table
CREATE TABLE IF NOT EXISTS scans (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    contract_code TEXT NOT NULL,
    contract_hash VARCHAR(64) NOT NULL,
    status VARCHAR(50) DEFAULT 'pending',
    scan_time TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deep_analysis BOOLEAN DEFAULT FALSE,
    check_invariants BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Vulnerabilities table
CREATE TABLE IF NOT EXISTS vulnerabilities (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    scan_id UUID REFERENCES scans(id) ON DELETE CASCADE,
    vulnerability_type VARCHAR(100) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    location_file VARCHAR(255),
    location_line INTEGER,
    location_column INTEGER,
    location_function VARCHAR(255),
    recommendation TEXT NOT NULL,
    cwe_id VARCHAR(20),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Bounty programs table
CREATE TABLE IF NOT EXISTS bounty_programs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    reward_amount BIGINT NOT NULL,
    status VARCHAR(50) DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Vulnerability reports table
CREATE TABLE IF NOT EXISTS vulnerability_reports (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    reporter_id UUID REFERENCES users(id) ON DELETE CASCADE,
    bounty_program_id UUID REFERENCES bounty_programs(id) ON DELETE CASCADE,
    contract_id VARCHAR(64) NOT NULL,
    vulnerability_type VARCHAR(100) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    description TEXT NOT NULL,
    location VARCHAR(255),
    status VARCHAR(50) DEFAULT 'pending',
    bounty_amount BIGINT DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Reputation table
CREATE TABLE IF NOT EXISTS reputation (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    score INTEGER DEFAULT 0,
    successful_reports INTEGER DEFAULT 0,
    total_earnings BIGINT DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_scans_user_id ON scans(user_id);
CREATE INDEX IF NOT EXISTS idx_scans_status ON scans(status);
CREATE INDEX IF NOT EXISTS idx_scans_scan_time ON scans(scan_time);
CREATE INDEX IF NOT EXISTS idx_vulnerabilities_scan_id ON vulnerabilities(scan_id);
CREATE INDEX IF NOT EXISTS idx_vulnerabilities_severity ON vulnerabilities(severity);
CREATE INDEX IF NOT EXISTS idx_vulnerability_reports_reporter_id ON vulnerability_reports(reporter_id);
CREATE INDEX IF NOT EXISTS idx_vulnerability_reports_status ON vulnerability_reports(status);

-- Insert default admin user (password: admin123)
INSERT INTO users (email, password_hash, role) 
VALUES ('admin@soroban-scanner.dev', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj3QJflLxQxe', 'admin')
ON CONFLICT (email) DO NOTHING;

-- Insert sample bounty program
INSERT INTO bounty_programs (name, description, reward_amount, status)
VALUES (
    'Soroban Smart Contract Security',
    'Find and report security vulnerabilities in Soroban smart contracts',
    1000000000, -- 100 XLM in stroops
    'active'
)
ON CONFLICT DO NOTHING;
