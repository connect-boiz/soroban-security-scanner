const SecurityReporter = require('../src/reporters/security-reporter');

describe('SecurityReporter', () => {
  let reporter;

  beforeEach(() => {
    reporter = new SecurityReporter();
  });

  const makeVuln = (overrides = {}) => ({
    type: 'DIRECT_TIMESTAMP_COMPARISON',
    severity: 'HIGH',
    description: 'Direct timestamp comparison without manipulation protection',
    file: 'contract.rs',
    line: 10,
    code: 'if now > timestamp { release(); }',
    ...overrides,
  });

  describe('generate()', () => {
    test('defaults to text format', () => {
      const report = reporter.generate([makeVuln()]);
      expect(typeof report).toBe('string');
      expect(report).toContain('HIGH SEVERITY');
    });

    test('returns JSON string when format is "json"', () => {
      const report = reporter.generate([makeVuln()], 'json');
      expect(() => JSON.parse(report)).not.toThrow();
    });
  });

  describe('generateTextReport()', () => {
    test('shows success message when no vulnerabilities', () => {
      const report = reporter.generate([], 'text');
      expect(report).toContain('No time-based attack vulnerabilities found');
    });

    test('shows vulnerability count in header', () => {
      const vulns = [makeVuln(), makeVuln({ severity: 'MEDIUM', type: 'UNPROTECTED_TIME_CONDITION' })];
      const report = reporter.generate(vulns, 'text');
      expect(report).toContain('2 time-based attack vulnerabilities');
    });

    test('groups vulnerabilities by severity', () => {
      const vulns = [
        makeVuln({ severity: 'HIGH' }),
        makeVuln({ severity: 'MEDIUM', type: 'UNPROTECTED_TIME_CONDITION' }),
        makeVuln({ severity: 'LOW', type: 'OTHER' }),
      ];
      const report = reporter.generate(vulns, 'text');
      expect(report).toContain('HIGH SEVERITY');
      expect(report).toContain('MEDIUM SEVERITY');
      expect(report).toContain('LOW SEVERITY');
    });

    test('includes file and line info', () => {
      const report = reporter.generate([makeVuln({ file: 'src/foo.rs', line: 42 })], 'text');
      expect(report).toContain('src/foo.rs');
      expect(report).toContain('42');
    });

    test('includes recommendations section', () => {
      const report = reporter.generate([makeVuln()], 'text');
      expect(report).toContain('RECOMMENDATIONS');
    });

    test('includes HIGH PRIORITY recommendations for HIGH severity', () => {
      const report = reporter.generate([makeVuln({ severity: 'HIGH' })], 'text');
      expect(report).toContain('HIGH PRIORITY');
    });

    test('includes MEDIUM PRIORITY recommendations for MEDIUM severity', () => {
      const report = reporter.generate([makeVuln({ severity: 'MEDIUM' })], 'text');
      expect(report).toContain('MEDIUM PRIORITY');
    });
  });

  describe('generateJsonReport()', () => {
    test('includes scanDate', () => {
      const report = JSON.parse(reporter.generate([], 'json'));
      expect(report.scanDate).toBeDefined();
      expect(new Date(report.scanDate).toString()).not.toBe('Invalid Date');
    });

    test('summary.total matches vulnerability count', () => {
      const vulns = [makeVuln(), makeVuln()];
      const report = JSON.parse(reporter.generate(vulns, 'json'));
      expect(report.summary.total).toBe(2);
    });

    test('summary.bySeverity counts correctly', () => {
      const vulns = [
        makeVuln({ severity: 'HIGH' }),
        makeVuln({ severity: 'HIGH' }),
        makeVuln({ severity: 'MEDIUM', type: 'UNPROTECTED_TIME_CONDITION' }),
      ];
      const report = JSON.parse(reporter.generate(vulns, 'json'));
      expect(report.summary.bySeverity.HIGH).toBe(2);
      expect(report.summary.bySeverity.MEDIUM).toBe(1);
    });

    test('vulnerabilities array is included', () => {
      const vulns = [makeVuln()];
      const report = JSON.parse(reporter.generate(vulns, 'json'));
      expect(report.vulnerabilities).toHaveLength(1);
      expect(report.vulnerabilities[0].type).toBe('DIRECT_TIMESTAMP_COMPARISON');
    });

    test('recommendations array is included', () => {
      const report = JSON.parse(reporter.generate([makeVuln()], 'json'));
      expect(Array.isArray(report.recommendations)).toBe(true);
    });

    test('recommendations include HIGH priority for HIGH severity vulns', () => {
      const report = JSON.parse(reporter.generate([makeVuln({ severity: 'HIGH' })], 'json'));
      const highRec = report.recommendations.find(r => r.priority === 'HIGH');
      expect(highRec).toBeDefined();
    });

    test('recommendations include LOCK_PERIOD action when applicable', () => {
      const report = JSON.parse(reporter.generate([makeVuln({ type: 'LOCK_PERIOD_TIMESTAMP_USAGE' })], 'json'));
      const lockRec = report.recommendations.find(r => r.action && r.action.includes('block heights'));
      expect(lockRec).toBeDefined();
    });

    test('returns empty recommendations for no vulnerabilities', () => {
      const report = JSON.parse(reporter.generate([], 'json'));
      expect(report.recommendations).toHaveLength(0);
    });
  });

  describe('groupBySeverity()', () => {
    test('groups correctly', () => {
      const vulns = [
        makeVuln({ severity: 'HIGH' }),
        makeVuln({ severity: 'HIGH' }),
        makeVuln({ severity: 'LOW', type: 'OTHER' }),
      ];
      const groups = reporter.groupBySeverity(vulns);
      expect(groups.HIGH).toHaveLength(2);
      expect(groups.LOW).toHaveLength(1);
      expect(groups.MEDIUM).toBeUndefined();
    });

    test('returns empty object for empty input', () => {
      expect(reporter.groupBySeverity([])).toEqual({});
    });
  });

  describe('countBySeverity()', () => {
    test('counts correctly', () => {
      const vulns = [makeVuln({ severity: 'HIGH' }), makeVuln({ severity: 'MEDIUM', type: 'X' })];
      const counts = reporter.countBySeverity(vulns);
      expect(counts.HIGH).toBe(1);
      expect(counts.MEDIUM).toBe(1);
    });
  });
});
