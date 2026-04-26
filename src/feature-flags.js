'use strict';

/**
 * Feature flag management for gradual rollouts and conditional feature availability.
 */

/**
 * @typedef {Object} FlagDefinition
 * @property {boolean} enabled - Default enabled state
 * @property {number} [rolloutPercentage] - 0-100; enables for that % of users
 * @property {string[]} [allowList] - User IDs always enabled for
 * @property {string[]} [denyList] - User IDs always disabled for
 */

class FeatureFlags {
  /** @param {Record<string, FlagDefinition>} [flags] */
  constructor(flags = {}) {
    /** @type {Record<string, FlagDefinition>} */
    this._flags = { ...flags };
  }

  /**
   * @param {string} name
   * @param {FlagDefinition} definition
   */
  define(name, definition) {
    this._flags[name] = definition;
  }

  /**
   * Check if a flag is enabled for an optional user ID.
   * @param {string} name
   * @param {string} [userId]
   * @returns {boolean}
   */
  isEnabled(name, userId) {
    const flag = this._flags[name];
    if (!flag) {
      return false;
    }
    if (userId) {
      if (flag.denyList?.includes(userId)) {
        return false;
      }
      if (flag.allowList?.includes(userId)) {
        return true;
      }
      if (flag.rolloutPercentage !== undefined) {
        return this._hashUser(name, userId) < flag.rolloutPercentage;
      }
    }
    return flag.enabled;
  }

  /**
   * Deterministic hash of (flag, userId) → 0-100.
   * @param {string} flag
   * @param {string} userId
   * @returns {number}
   */
  _hashUser(flag, userId) {
    const str = `${flag}:${userId}`;
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = (hash * 31 + str.charCodeAt(i)) >>> 0;
    }
    return hash % 100;
  }

  /**
   * Override a flag at runtime (e.g. from remote config).
   * @param {string} name
   * @param {Partial<FlagDefinition>} overrides
   */
  override(name, overrides) {
    if (!this._flags[name]) {
      this._flags[name] = { enabled: false };
    }
    Object.assign(this._flags[name], overrides);
  }

  /** @returns {Record<string, FlagDefinition>} */
  getAll() {
    return { ...this._flags };
  }
}

const featureFlags = new FeatureFlags();

module.exports = { FeatureFlags, featureFlags };
