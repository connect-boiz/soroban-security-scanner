/**
 * Web Vitals monitoring for Soroban Security Scanner
 * Tracks Core Web Vitals: LCP, FID, CLS, FCP, TTFB
 */

const PERFORMANCE_BUDGETS = {
  // Core Web Vitals thresholds (in ms, except CLS)
  LCP: { good: 2500, needsImprovement: 4000 },
  FID: { good: 100, needsImprovement: 300 },
  CLS: { good: 0.1, needsImprovement: 0.25 },
  FCP: { good: 1800, needsImprovement: 3000 },
  TTFB: { good: 800, needsImprovement: 1800 },
  // Resource budgets
  totalBundleSize: 500 * 1024, // 500KB
  imageSize: 200 * 1024,       // 200KB per image
  scriptSize: 150 * 1024,      // 150KB per script
};

/**
 * Classify a metric value against its budget thresholds
 * @param {string} metric - Metric name
 * @param {number} value - Metric value
 * @returns {{ rating: string, value: number, budget: object }}
 */
function classifyMetric(metric, value) {
  const budget = PERFORMANCE_BUDGETS[metric];
  if (!budget) {
    return { rating: 'unknown', value, budget: null };
  }
  let rating;
  if (value <= budget.good) {
    rating = 'good';
  } else if (value <= budget.needsImprovement) {
    rating = 'needs-improvement';
  } else {
    rating = 'poor';
  }
  return { rating, value, budget };
}

/**
 * Report a web vital metric
 * @param {{ name: string, value: number, id: string }} metric
 * @param {Function} [reporter] - Optional custom reporter function
 */
function reportWebVital(metric, reporter) {
  const { name, value, id } = metric;
  const classification = classifyMetric(name, value);

  const report = {
    metric: name,
    value: Math.round(name === 'CLS' ? value * 1000 : value),
    id,
    rating: classification.rating,
    timestamp: new Date().toISOString(),
  };

  if (typeof reporter === 'function') {
    reporter(report);
  }

  return report;
}

/**
 * Check if a resource size is within budget
 * @param {string} type - Resource type ('totalBundleSize' | 'imageSize' | 'scriptSize')
 * @param {number} sizeBytes - Size in bytes
 * @returns {{ withinBudget: boolean, budget: number, actual: number }}
 */
function checkResourceBudget(type, sizeBytes) {
  const budget = PERFORMANCE_BUDGETS[type];
  return {
    withinBudget: sizeBytes <= budget,
    budget,
    actual: sizeBytes,
  };
}

module.exports = { reportWebVital, classifyMetric, checkResourceBudget, PERFORMANCE_BUDGETS };
