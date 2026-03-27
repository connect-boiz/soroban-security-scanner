-- Initial migration for remediation history and vulnerabilities

CREATE TABLE IF NOT EXISTS vulnerabilities (
    id VARCHAR(255) PRIMARY KEY,
    file_path VARCHAR(500) NOT NULL,
    vulnerability_type VARCHAR(100) NOT NULL,
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('Critical', 'High', 'Medium', 'Low')),
    title VARCHAR(500) NOT NULL,
    description TEXT NOT NULL,
    code_snippet TEXT NOT NULL,
    line_number INTEGER NOT NULL,
    sarif_report JSONB,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS remediation_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vulnerability_id VARCHAR(255) NOT NULL REFERENCES vulnerabilities(id) ON DELETE CASCADE,
    patch JSONB NOT NULL,
    confidence_score FLOAT NOT NULL CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
    verification_status VARCHAR(50) NOT NULL CHECK (verification_status IN ('Passed', 'Failed', 'Skipped')),
    applied BOOLEAN NOT NULL DEFAULT FALSE,
    success_rate FLOAT NOT NULL CHECK (success_rate >= 0.0 AND success_rate <= 1.0),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_remediation_vulnerability_id ON remediation_history(vulnerability_id);
CREATE INDEX IF NOT EXISTS idx_remediation_created_at ON remediation_history(created_at);
CREATE INDEX IF NOT EXISTS idx_remediation_success_rate ON remediation_history(success_rate);
CREATE INDEX IF NOT EXISTS idx_remediation_confidence_score ON remediation_history(confidence_score);
CREATE INDEX IF NOT EXISTS idx_vulnerability_type ON vulnerabilities(vulnerability_type);
CREATE INDEX IF NOT EXISTS idx_vulnerability_severity ON vulnerabilities(severity);
CREATE INDEX IF NOT EXISTS idx_vulnerability_file_path ON vulnerabilities(file_path);

-- Trigger to update updated_at column
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_vulnerabilities_updated_at BEFORE UPDATE ON vulnerabilities
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_remediation_history_updated_at BEFORE UPDATE ON remediation_history
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add comments for documentation
COMMENT ON TABLE vulnerabilities IS 'Stores detected vulnerabilities from security scans';
COMMENT ON TABLE remediation_history IS 'Stores history of AI-generated patches and their outcomes';
COMMENT ON COLUMN vulnerabilities.sarif_report IS 'SARIF format security report data';
COMMENT ON COLUMN remediation_history.patch IS 'JSON representation of the generated code patch';
COMMENT ON COLUMN remediation_history.confidence_score IS 'AI confidence score (0.0 to 1.0)';
COMMENT ON COLUMN remediation_history.verification_status IS 'Result of automated patch verification';
