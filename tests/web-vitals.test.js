const { reportWebVital, classifyMetric, checkResourceBudget, PERFORMANCE_BUDGETS } = require('../src/performance/web-vitals');

describe('Web Vitals', () => {
  describe('classifyMetric', () => {
    test('classifies good LCP', () => {
      const result = classifyMetric('LCP', 2000);
      expect(result.rating).toBe('good');
    });

    test('classifies needs-improvement LCP', () => {
      const result = classifyMetric('LCP', 3000);
      expect(result.rating).toBe('needs-improvement');
    });

    test('classifies poor LCP', () => {
      const result = classifyMetric('LCP', 5000);
      expect(result.rating).toBe('poor');
    });

    test('returns unknown for unrecognized metric', () => {
      const result = classifyMetric('UNKNOWN', 100);
      expect(result.rating).toBe('unknown');
    });
  });

  describe('reportWebVital', () => {
    test('calls reporter with correct shape', () => {
      const reporter = jest.fn();
      reportWebVital({ name: 'LCP', value: 2000, id: 'v3-123' }, reporter);
      expect(reporter).toHaveBeenCalledWith(
        expect.objectContaining({
          metric: 'LCP',
          rating: 'good',
          id: 'v3-123',
        })
      );
    });

    test('returns report object', () => {
      const report = reportWebVital({ name: 'CLS', value: 0.05, id: 'v3-456' });
      expect(report).toMatchObject({ metric: 'CLS', rating: 'good' });
    });
  });

  describe('checkResourceBudget', () => {
    test('passes when within budget', () => {
      const result = checkResourceBudget('totalBundleSize', 400 * 1024);
      expect(result.withinBudget).toBe(true);
    });

    test('fails when over budget', () => {
      const result = checkResourceBudget('totalBundleSize', 600 * 1024);
      expect(result.withinBudget).toBe(false);
    });
  });

  test('PERFORMANCE_BUDGETS exports expected keys', () => {
    expect(PERFORMANCE_BUDGETS).toHaveProperty('LCP');
    expect(PERFORMANCE_BUDGETS).toHaveProperty('CLS');
    expect(PERFORMANCE_BUDGETS).toHaveProperty('totalBundleSize');
  });
});
