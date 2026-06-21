'use strict';

const { ErrorLogger, ConsoleTransport } = require('../src/error-logger');
const { FeatureFlags } = require('../src/feature-flags');
const { ABTesting } = require('../src/ab-testing');

// ── ErrorLogger ──────────────────────────────────────────────────────────────

describe('ErrorLogger', () => {
  test('logs to transport', () => {
    const entries = [];
    const transport = { log: (e) => entries.push(e) };
    const logger = new ErrorLogger({ transports: [transport] });
    logger.info('hello', { foo: 'bar' });
    expect(entries).toHaveLength(1);
    expect(entries[0].level).toBe('info');
    expect(entries[0].message).toBe('hello');
  });

  test('respects minLevel', () => {
    const entries = [];
    const transport = { log: (e) => entries.push(e) };
    const logger = new ErrorLogger({ transports: [transport], minLevel: 'warn' });
    logger.debug('ignored');
    logger.info('also ignored');
    logger.warn('captured');
    expect(entries).toHaveLength(1);
    expect(entries[0].level).toBe('warn');
  });

  test('captureException records error', () => {
    const entries = [];
    const transport = { log: (e) => entries.push(e) };
    const logger = new ErrorLogger({ transports: [transport] });
    const err = new Error('boom');
    logger.captureException(err);
    expect(entries[0].error).toBe(err);
    expect(entries[0].message).toBe('boom');
  });

  test('transport failure does not throw', () => {
    const bad = { log: () => { throw new Error('transport down'); } };
    const logger = new ErrorLogger({ transports: [bad] });
    expect(() => logger.error('msg', new Error('x'))).not.toThrow();
  });
});

// ── FeatureFlags ─────────────────────────────────────────────────────────────

describe('FeatureFlags', () => {
  test('returns false for unknown flag', () => {
    const ff = new FeatureFlags();
    expect(ff.isEnabled('unknown')).toBe(false);
  });

  test('returns default enabled state', () => {
    const ff = new FeatureFlags({ myFlag: { enabled: true } });
    expect(ff.isEnabled('myFlag')).toBe(true);
  });

  test('allowList overrides rollout', () => {
    const ff = new FeatureFlags({
      flag: { enabled: false, rolloutPercentage: 0, allowList: ['user1'] },
    });
    expect(ff.isEnabled('flag', 'user1')).toBe(true);
    expect(ff.isEnabled('flag', 'user2')).toBe(false);
  });

  test('denyList overrides enabled', () => {
    const ff = new FeatureFlags({
      flag: { enabled: true, denyList: ['blocked'] },
    });
    expect(ff.isEnabled('flag', 'blocked')).toBe(false);
    expect(ff.isEnabled('flag', 'other')).toBe(true);
  });

  test('rollout is deterministic', () => {
    const ff = new FeatureFlags({ flag: { enabled: false, rolloutPercentage: 50 } });
    const result1 = ff.isEnabled('flag', 'userA');
    const result2 = ff.isEnabled('flag', 'userA');
    expect(result1).toBe(result2);
  });

  test('override updates flag', () => {
    const ff = new FeatureFlags({ flag: { enabled: false } });
    ff.override('flag', { enabled: true });
    expect(ff.isEnabled('flag')).toBe(true);
  });
});

// ── ABTesting ────────────────────────────────────────────────────────────────

describe('ABTesting', () => {
  const experiment = {
    name: 'button-color',
    active: true,
    variants: [
      { name: 'control', weight: 50 },
      { name: 'treatment', weight: 50 },
    ],
  };

  test('returns null for unknown experiment', () => {
    const ab = new ABTesting();
    expect(ab.getVariant('unknown', 'user1')).toBeNull();
  });

  test('returns null for inactive experiment', () => {
    const ab = new ABTesting();
    ab.define({ ...experiment, active: false });
    expect(ab.getVariant('button-color', 'user1')).toBeNull();
  });

  test('assignment is deterministic', () => {
    const ab = new ABTesting();
    ab.define(experiment);
    const v1 = ab.getVariant('button-color', 'user42');
    const v2 = ab.getVariant('button-color', 'user42');
    expect(v1).toBe(v2);
    expect(['control', 'treatment']).toContain(v1);
  });

  test('assignment is sticky', () => {
    const ab = new ABTesting();
    ab.define(experiment);
    const first = ab.getVariant('button-color', 'stickyUser');
    // Call again — should return same cached value
    expect(ab.getVariant('button-color', 'stickyUser')).toBe(first);
  });

  test('track returns structured event', () => {
    const ab = new ABTesting();
    ab.define(experiment);
    const result = ab.track('button-color', 'user1', 'click');
    expect(result).toMatchObject({
      experimentName: 'button-color',
      userId: 'user1',
      event: 'click',
    });
    expect(['control', 'treatment']).toContain(result.variant);
  });
});
