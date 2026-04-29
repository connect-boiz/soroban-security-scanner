const TimeBasedAttackDetector = require('../src/detectors/time-based-attack-detector');
const SecurityReporter = require('../src/reporters/security-reporter');
const fs = require('fs');
const path = require('path');

describe('Time-Based Attack Detector', () => {
  let detector;
  let reporter;

  beforeEach(() => {
    detector = new TimeBasedAttackDetector();
    reporter = new SecurityReporter();
  });

  describe('Vulnerability Detection', () => {
    test('should detect direct timestamp comparisons', async () => {
      const vulnerableCode = `
        if now > timestamp {
            release_funds();
        }
      `;
      
      const tempFile = path.join(__dirname, 'temp-vulnerable.rs');
      fs.writeFileSync(tempFile, vulnerableCode);
      
      const vulnerabilities = await detector.scan(tempFile);
      
      expect(vulnerabilities).toHaveLength(1);
      expect(vulnerabilities[0].type).toBe('DIRECT_TIMESTAMP_COMPARISON');
      expect(vulnerabilities[0].severity).toBe('HIGH');
      
      fs.unlinkSync(tempFile);
    });

    test('should detect lock period timestamp usage', async () => {
      const vulnerableCode = `
        let lock_period = timestamp - now;
        require(lock_period > 0, "Lock not expired");
      `;
      
      const tempFile = path.join(__dirname, 'temp-lock-period.rs');
      fs.writeFileSync(tempFile, vulnerableCode);
      
      const vulnerabilities = await detector.scan(tempFile);
      
      expect(vulnerabilities.length).toBeGreaterThan(0);
      const lockPeriodVuln = vulnerabilities.find(v => v.type === 'LOCK_PERIOD_TIMESTAMP_USAGE');
      expect(lockPeriodVuln).toBeDefined();
      expect(lockPeriodVuln.severity).toBe('HIGH');
      
      fs.unlinkSync(tempFile);
    });

    test('should detect unprotected time conditions', async () => {
      const vulnerableCode = `
        if (block.timestamp > deadline) {
            execute_action();
        }
      `;
      
      const tempFile = path.join(__dirname, 'temp-time-condition.rs');
      fs.writeFileSync(tempFile, vulnerableCode);
      
      const vulnerabilities = await detector.scan(tempFile);
      
      expect(vulnerabilities.length).toBeGreaterThan(0);
      const timeConditionVuln = vulnerabilities.find(v => v.type === 'UNPROTECTED_TIME_CONDITION');
      expect(timeConditionVuln).toBeDefined();
      expect(timeConditionVuln.severity).toBe('MEDIUM');
      
      fs.unlinkSync(tempFile);
    });

    test('should detect timestamp arithmetic for locks', async () => {
      const vulnerableCode = `
        let unlock_time = timestamp + 86400 * 7;
        if (now >= unlock_time) {
            withdraw();
        }
      `;
      
      const tempFile = path.join(__dirname, 'temp-timestamp-arithmetic.rs');
      fs.writeFileSync(tempFile, vulnerableCode);
      
      const vulnerabilities = await detector.scan(tempFile);
      
      expect(vulnerabilities.length).toBeGreaterThan(0);
      const arithmeticVuln = vulnerabilities.find(v => v.type === 'TIMESTAMP_ARITHMETIC_LOCK');
      expect(arithmeticVuln).toBeDefined();
      expect(arithmeticVuln.severity).toBe('HIGH');
      
      fs.unlinkSync(tempFile);
    });
  });

  describe('Report Generation', () => {
    test('should generate text report for vulnerabilities', () => {
      const vulnerabilities = [
        {
          type: 'DIRECT_TIMESTAMP_COMPARISON',
          severity: 'HIGH',
          description: 'Direct timestamp comparison without protection',
          file: 'test.rs',
          line: 10,
          code: 'if now > timestamp { release(); }'
        }
      ];

      const report = reporter.generate(vulnerabilities, 'text');
      
      expect(report).toContain('HIGH SEVERITY');
      expect(report).toContain('DIRECT_TIMESTAMP_COMPARISON');
      expect(report).toContain('RECOMMENDATIONS');
    });

    test('should generate JSON report for vulnerabilities', () => {
      const vulnerabilities = [
        {
          type: 'LOCK_PERIOD_TIMESTAMP_USAGE',
          severity: 'HIGH',
          description: 'Lock period using vulnerable timestamp',
          file: 'test.rs',
          line: 5,
          code: 'lock_period = timestamp - now'
        }
      ];

      const report = JSON.parse(reporter.generate(vulnerabilities, 'json'));
      
      expect(report.summary.total).toBe(1);
      expect(report.summary.bySeverity.HIGH).toBe(1);
      expect(report.vulnerabilities).toHaveLength(1);
      expect(report.recommendations).toBeDefined();
    });

    test('should show success message when no vulnerabilities found', () => {
      const report = reporter.generate([], 'text');
      
      expect(report).toContain('No time-based attack vulnerabilities found');
    });
  });

  describe('File Scanning', () => {
    test('should scan directory recursively', async () => {
      const testDir = path.join(__dirname, 'test-contracts');
      fs.mkdirSync(testDir, { recursive: true });
      
      // Create test files
      fs.writeFileSync(
        path.join(testDir, 'contract1.rs'),
        'if now > timestamp { release(); }'
      );
      fs.writeFileSync(
        path.join(testDir, 'contract2.rs'), 
        'let lock = timestamp + 86400;'
      );
      
      const vulnerabilities = await detector.scan(testDir);
      
      expect(vulnerabilities.length).toBeGreaterThan(0);
      
      // Cleanup
      fs.rmSync(testDir, { recursive: true, force: true });
    });

    test('should handle non-contract files gracefully', async () => {
      const nonContractFile = path.join(__dirname, 'readme.md');
      fs.writeFileSync(nonContractFile, '# README');
      
      const vulnerabilities = await detector.scan(nonContractFile);
      
      expect(vulnerabilities).toHaveLength(0);
      
      fs.unlinkSync(nonContractFile);
    });
  });
});

describe('Integration Tests', () => {
  test('should scan example vulnerable contract', async () => {
    const detector = new TimeBasedAttackDetector();
    const vulnerableContractPath = path.join(__dirname, '../examples/vulnerable-contract.rs');
    
    // Skip test if example file doesn't exist
    if (!fs.existsSync(vulnerableContractPath)) {
      return;
    }
    
    const vulnerabilities = await detector.scan(vulnerableContractPath);
    
    expect(vulnerabilities.length).toBeGreaterThan(0);
    
    // Should detect multiple vulnerability types
    const vulnerabilityTypes = vulnerabilities.map(v => v.type);
    expect(vulnerabilityTypes).toContain('DIRECT_TIMESTAMP_COMPARISON');
  });

  test('should scan example secure contract with fewer vulnerabilities', async () => {
    const detector = new TimeBasedAttackDetector();
    const secureContractPath = path.join(__dirname, '../examples/secure-contract.rs');
    
    // Skip test if example file doesn't exist
    if (!fs.existsSync(secureContractPath)) {
      return;
    }
    
    const vulnerabilities = await detector.scan(secureContractPath);
    
    // Secure contract should have fewer detected vulnerabilities
    // (may still have some false positives from pattern matching)
    expect(vulnerabilities.length).toBeLessThan(5);
  });
});
