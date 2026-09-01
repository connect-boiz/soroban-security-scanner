'use strict';

/**
 * A/B testing infrastructure for UI variations and features.
 */

/**
 * @typedef {Object} Variant
 * @property {string} name
 * @property {number} weight - Relative weight (e.g. 50 for 50%)
 */

/**
 * @typedef {Object} Experiment
 * @property {string} name
 * @property {Variant[]} variants
 * @property {boolean} [active]
 */

class ABTesting {
  constructor() {
    /** @type {Record<string, Experiment>} */
    this._experiments = {};
    /** @type {Record<string, Record<string, string>>} */
    this._assignments = {};
  }

  /**
   * Register an experiment.
   * @param {Experiment} experiment
   */
  define(experiment) {
    this._experiments[experiment.name] = experiment;
  }

  /**
   * Get the variant assigned to a user for an experiment.
   * Assignment is deterministic and sticky per user+experiment.
   * @param {string} experimentName
   * @param {string} userId
   * @returns {string|null} variant name, or null if experiment inactive/unknown
   */
  getVariant(experimentName, userId) {
    const experiment = this._experiments[experimentName];
    if (!experiment || experiment.active === false) {
      return null;
    }

    if (!this._assignments[experimentName]) {
      this._assignments[experimentName] = {};
    }
    if (this._assignments[experimentName][userId]) {
      return this._assignments[experimentName][userId];
    }

    const variant = this._assignVariant(experiment, userId);
    this._assignments[experimentName][userId] = variant;
    return variant;
  }

  /**
   * @param {Experiment} experiment
   * @param {string} userId
   * @returns {string}
   */
  _assignVariant(experiment, userId) {
    const totalWeight = experiment.variants.reduce((sum, v) => sum + v.weight, 0);
    const bucket = this._hash(experiment.name, userId) % totalWeight;
    let cumulative = 0;
    for (const variant of experiment.variants) {
      cumulative += variant.weight;
      if (bucket < cumulative) {
        return variant.name;
      }
    }
    return experiment.variants[experiment.variants.length - 1].name;
  }

  /**
   * Deterministic hash → integer.
   * @param {string} experiment
   * @param {string} userId
   * @returns {number}
   */
  _hash(experiment, userId) {
    const str = `${experiment}:${userId}`;
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = (hash * 31 + str.charCodeAt(i)) >>> 0;
    }
    return hash;
  }

  /**
   * Track a conversion event for analytics.
   * @param {string} experimentName
   * @param {string} userId
   * @param {string} event
   */
  track(experimentName, userId, event) {
    const variant = this.getVariant(experimentName, userId);
    return { experimentName, userId, variant, event, timestamp: new Date().toISOString() };
  }
}

const abTesting = new ABTesting();

module.exports = { ABTesting, abTesting };
