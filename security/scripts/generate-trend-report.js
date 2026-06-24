#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

class TrendReportGenerator {
  constructor() {
    this.metricsPath = process.env.METRICS_DATA_PATH ||
      path.join(process.cwd(), 'security', 'reports', 'metrics', 'metrics-history.json');
    this.reportDir = path.join(process.cwd(), 'security', 'reports');
  }

  generate() {
    if (!fs.existsSync(this.metricsPath)) {
      console.log('No metrics history found. Run security-metrics first.');
      this.generateEmptyReport();
      return;
    }

    const history = JSON.parse(fs.readFileSync(this.metricsPath, 'utf8'));

    if (history.length === 0) {
      this.generateEmptyReport();
      return;
    }

    const latest = history[history.length - 1];
    const previous = history.length > 1 ? history[history.length - 2] : null;

    const report = {
      generated: new Date().toISOString(),
      title: 'Security Metrics Trend Report',
      period: {
        start: history[0]?.period?.start || 'N/A',
        end: latest.period?.end || 'N/A',
        total_weeks: history.length,
      },
      current_state: {
        total_vulnerabilities: latest.vulnerabilities?.total || 0,
        critical: latest.vulnerabilities?.critical || 0,
        high: latest.vulnerabilities?.high || 0,
        scan_coverage: latest.coverage || {},
        coverage_score: Object.values(latest.coverage || {}).filter(Boolean).length /
          Object.keys(latest.coverage || {}).length * 100,
      },
      trend: {
        direction: this.calculateTrend(history),
        week_over_week: previous ? {
          critical: latest.vulnerabilities.critical - previous.vulnerabilities.critical,
          high: latest.vulnerabilities.high - previous.vulnerabilities.high,
          total: latest.vulnerabilities.total - previous.vulnerabilities.total,
        } : null,
      },
      recommendations: this.generateRecommendations(latest),
    };

    fs.writeFileSync(
      path.join(this.reportDir, 'trend-report.json'),
      JSON.stringify(report, null, 2)
    );

    const markdown = this.toMarkdown(report);
    fs.writeFileSync(path.join(this.reportDir, 'trend-report.md'), markdown);

    console.log(markdown);
  }

  calculateTrend(history) {
    if (history.length < 3) return 'insufficient_data';
    const recent = history.slice(-3);
    const avgRecent = recent.reduce((s, m) => s + m.vulnerabilities.total, 0) / recent.length;
    const older = history.slice(-6, -3);
    const avgOlder = older.reduce((s, m) => s + m.vulnerabilities.total, 0) / older.length;

    if (avgRecent < avgOlder * 0.9) return 'improving';
    if (avgRecent > avgOlder * 1.1) return 'degrading';
    return 'stable';
  }

  generateRecommendations(latest) {
    const recs = [];

    if (latest.vulnerabilities?.critical > 0) {
      recs.push('CRITICAL: Address critical vulnerabilities immediately');
    }
    if (latest.vulnerabilities?.high > 5) {
      recs.push('HIGH: Reduce high-severity vulnerabilities below threshold of 5');
    }
    if (!latest.coverage?.sast) {
      recs.push('Enable SAST scanning for all code changes');
    }
    if (!latest.coverage?.dast) {
      recs.push('Configure DAST scanning for staging environment');
    }
    if (!latest.coverage?.sca) {
      recs.push('Enable dependency vulnerability scanning');
    }
    if (!latest.coverage?.container) {
      recs.push('Add container image security scanning');
    }

    if (recs.length === 0) {
      recs.push('Maintain current security posture. Continue monitoring.');
    }

    return recs;
  }

  generateEmptyReport() {
    const report = {
      generated: new Date().toISOString(),
      title: 'Security Metrics Trend Report',
      status: 'no_data',
      message: 'No metrics data available. Security scanning workflows may not have run yet.',
    };
    fs.writeFileSync(
      path.join(this.reportDir, 'trend-report.json'),
      JSON.stringify(report, null, 2)
    );
    console.log(JSON.stringify(report, null, 2));
  }

  toMarkdown(report) {
    let md = `# ${report.title}\n\n`;
    md += `**Generated**: ${report.generated}\n\n`;

    if (report.status === 'no_data') {
      md += `_${report.message}_\n`;
      return md;
    }

    md += `## Period\n\n`;
    md += `- **Start**: ${report.period.start}\n`;
    md += `- **End**: ${report.period.end}\n`;
    md += `- **Weeks tracked**: ${report.period.total_weeks}\n\n`;

    md += `## Current State\n\n`;
    md += `| Metric | Value |\n`;
    md += `|---|---|\n`;
    md += `| Total Vulnerabilities | ${report.current_state.total_vulnerabilities} |\n`;
    md += `| Critical | ${report.current_state.critical} |\n`;
    md += `| High | ${report.current_state.high} |\n`;
    md += `| Scan Coverage Score | ${report.current_state.coverage_score.toFixed(1)}% |\n\n`;

    md += `### Coverage\n\n`;
    for (const [key, covered] of Object.entries(report.current_state.scan_coverage)) {
      md += `- **${key.toUpperCase()}**: ${covered ? '✅' : '❌'}\n`;
    }

    md += `\n## Trend\n\n`;
    md += `- **Direction**: ${report.trend.direction}\n`;
    if (report.trend.week_over_week) {
      const wow = report.trend.week_over_week;
      md += `- **Week-over-week change**: ${wow.total} total (critical: ${wow.critical >= 0 ? '+' : ''}${wow.critical}, high: ${wow.high >= 0 ? '+' : ''}${wow.high})\n`;
    }

    md += `\n## Recommendations\n\n`;
    for (const rec of report.recommendations) {
      md += `- ${rec}\n`;
    }

    return md;
  }
}

const generator = new TrendReportGenerator();
generator.generate();
