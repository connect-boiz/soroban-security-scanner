const TimeBasedAttackDetector = require('../src/detectors/time-based-attack-detector');
const fs = require('fs');
const path = require('path');

const TMP = path.join(__dirname, '_tmp_detector');

beforeAll(() => fs.mkdirSync(TMP, { recursive: true }));
afterAll(() => fs.rmSync(TMP, { recursive: true, force: true }));

function writeTmp(name, content) {
  const p = path.join(TMP, name);
  fs.writeFileSync(p, content);
  return p;
}

describe('TimeBasedAttackDetector', () => {
  let detector;

  beforeEach(() => {
    detector = new TimeBasedAttackDetector();
  });

  // ── Issue #87: memory-leak regression ──────────────────────────────────────
  describe('Memory leak fix (#87)', () => {
    test('scan() does not accumulate results across multiple calls', async () => {
      const file = writeTmp('ml-test.rs', 'if now > timestamp { release(); }');

      const first = await detector.scan(file);
      const second = await detector.scan(file);

      // Each call must return an independent result set of the same length,
      // not a growing array from shared instance state.
      expect(first).toHaveLength(second.length);
      expect(first).not.toBe(second); // different array references
    });

    test('scan() result is isolated from subsequent scans', async () => {
      const file = writeTmp('ml-isolation.rs', 'if now > timestamp { release(); }');

      const first = await detector.scan(file);
      await detector.scan(file); // second scan should not mutate first result

      expect(first).toHaveLength(1);
    });
  });

  // ── Pattern detection ───────────────────────────────────────────────────────
  describe('DIRECT_TIMESTAMP_COMPARISON', () => {
    test('detects "now < timestamp"', async () => {
      const file = writeTmp('dtc1.rs', 'if now < timestamp { release(); }');
      const vulns = await detector.scan(file);
      expect(vulns.some(v => v.type === 'DIRECT_TIMESTAMP_COMPARISON')).toBe(true);
    });

    test('detects "timestamp > now"', async () => {
      const file = writeTmp('dtc2.rs', 'if timestamp > now { release(); }');
      const vulns = await detector.scan(file);
      expect(vulns.some(v => v.type === 'DIRECT_TIMESTAMP_COMPARISON')).toBe(true);
    });

    test('severity is HIGH', async () => {
      const file = writeTmp('dtc3.rs', 'if now < timestamp { }');
      const vulns = await detector.scan(file);
      const v = vulns.find(v => v.type === 'DIRECT_TIMESTAMP_COMPARISON');
      expect(v.severity).toBe('HIGH');
    });
  });

  describe('LOCK_PERIOD_TIMESTAMP_USAGE', () => {
    test('detects "lock_period = ... timestamp"', async () => {
      const file = writeTmp('lp1.rs', 'let lock_period = timestamp - now;');
      const vulns = await detector.scan(file);
      expect(vulns.some(v => v.type === 'LOCK_PERIOD_TIMESTAMP_USAGE')).toBe(true);
    });

    test('severity is HIGH', async () => {
      const file = writeTmp('lp2.rs', 'let lock_period = timestamp - now;');
      const vulns = await detector.scan(file);
      const v = vulns.find(v => v.type === 'LOCK_PERIOD_TIMESTAMP_USAGE');
      expect(v.severity).toBe('HIGH');
    });
  });

  describe('UNPROTECTED_TIME_CONDITION', () => {
    test('detects "if (... time ...) {"', async () => {
      const file = writeTmp('utc1.rs', 'if (block.timestamp > deadline) {');
      const vulns = await detector.scan(file);
      expect(vulns.some(v => v.type === 'UNPROTECTED_TIME_CONDITION')).toBe(true);
    });

    test('severity is MEDIUM', async () => {
      const file = writeTmp('utc2.rs', 'if (block.timestamp > deadline) {');
      const vulns = await detector.scan(file);
      const v = vulns.find(v => v.type === 'UNPROTECTED_TIME_CONDITION');
      expect(v.severity).toBe('MEDIUM');
    });
  });

  describe('TIMESTAMP_ARITHMETIC_LOCK', () => {
    test('detects "timestamp + N * M"', async () => {
      const file = writeTmp('tal1.rs', 'let unlock_time = timestamp + 86400 * 7;');
      const vulns = await detector.scan(file);
      expect(vulns.some(v => v.type === 'TIMESTAMP_ARITHMETIC_LOCK')).toBe(true);
    });

    test('severity is HIGH', async () => {
      const file = writeTmp('tal2.rs', 'let unlock_time = timestamp + 86400 * 7;');
      const vulns = await detector.scan(file);
      const v = vulns.find(v => v.type === 'TIMESTAMP_ARITHMETIC_LOCK');
      expect(v.severity).toBe('HIGH');
    });
  });

  // ── Vulnerability metadata ──────────────────────────────────────────────────
  describe('Vulnerability metadata', () => {
    test('includes correct file path', async () => {
      const file = writeTmp('meta.rs', 'if now < timestamp { }');
      const vulns = await detector.scan(file);
      expect(vulns[0].file).toBe(file);
    });

    test('includes correct line number', async () => {
      const file = writeTmp('lineno.rs', '// safe line\nif now < timestamp { }');
      const vulns = await detector.scan(file);
      expect(vulns[0].line).toBe(2);
    });

    test('includes trimmed code snippet', async () => {
      const file = writeTmp('snippet.rs', '   if now < timestamp { }   ');
      const vulns = await detector.scan(file);
      expect(vulns[0].code).toBe('if now < timestamp { }');
    });
  });

  // ── File filtering ──────────────────────────────────────────────────────────
  describe('File filtering', () => {
    test('scans .rs files', async () => {
      const file = writeTmp('contract.rs', 'if now < timestamp { }');
      const vulns = await detector.scan(file);
      expect(vulns.length).toBeGreaterThan(0);
    });

    test('scans .js files', async () => {
      const file = writeTmp('contract.js', 'if now < timestamp { }');
      const vulns = await detector.scan(file);
      expect(vulns.length).toBeGreaterThan(0);
    });

    test('ignores .md files', async () => {
      const dir = path.join(TMP, 'filter-test');
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, 'README.md'), 'if now < timestamp { }');
      const vulns = await detector.scan(dir);
      expect(vulns).toHaveLength(0);
      fs.rmSync(dir, { recursive: true, force: true });
    });

    test('returns empty array for file with no matches', async () => {
      const file = writeTmp('clean.rs', 'fn safe_function() { let x = 1; }');
      const vulns = await detector.scan(file);
      expect(vulns).toHaveLength(0);
    });
  });

  // ── Directory scanning ──────────────────────────────────────────────────────
  describe('Directory scanning', () => {
    test('scans all contract files in a directory', async () => {
      const dir = path.join(TMP, 'dir-scan');
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, 'a.rs'), 'if now < timestamp { }');
      fs.writeFileSync(path.join(dir, 'b.rs'), 'let lock_period = timestamp - now;');

      const vulns = await detector.scan(dir);
      expect(vulns.length).toBeGreaterThanOrEqual(2);

      fs.rmSync(dir, { recursive: true, force: true });
    });

    test('scans subdirectories recursively', async () => {
      const dir = path.join(TMP, 'recursive');
      const sub = path.join(dir, 'sub');
      fs.mkdirSync(sub, { recursive: true });
      fs.writeFileSync(path.join(sub, 'deep.rs'), 'if now < timestamp { }');

      const vulns = await detector.scan(dir);
      expect(vulns.length).toBeGreaterThan(0);

      fs.rmSync(dir, { recursive: true, force: true });
    });
  });

  // ── Emergency stop ──────────────────────────────────────────────────────────
  describe('Emergency stop integration', () => {
    test('stops scanning when emergencyStop is active', async () => {
      const dir = path.join(TMP, 'estop');
      fs.mkdirSync(dir, { recursive: true });
      for (let i = 0; i < 5; i++) {
        fs.writeFileSync(path.join(dir, `c${i}.rs`), 'if now < timestamp { }');
      }

      const emergencyStop = { isActive: () => true };
      const vulns = await detector.scan(dir, emergencyStop);

      // With emergency stop active from the start, no files should be scanned
      expect(vulns).toHaveLength(0);

      fs.rmSync(dir, { recursive: true, force: true });
    });
  });
});
