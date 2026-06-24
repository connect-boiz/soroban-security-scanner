#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

class SecurityMetricsCollector {
  constructor() {
    this.repo = process.env.GITHUB_REPOSITORY || 'unknown/repo';
    this.metricsDir = path.join(process.cwd(), 'security', 'reports', 'metrics');
    this.historyFile = path.join(this.metricsDir, 'metrics-history.json');
    this.currentMetrics = {
      timestamp: new Date().toISOString(),
      period: this.getPeriod(),
      scans: [],
      vulnerabilities: {
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
        total: 0,
      },
      coverage: {
        sast: false,
        dast: false,
        sca: false,
        container: false,
        iac: false,
        secret: false,
      },
      findings_by_type: {},
      scan_duration_ms: 0,
      gate_passed: false,
    };
  }

  getPeriod() {
    const now = new Date();
    const monday = new Date(now);
    monday.setDate(now.getDate() - ((now.getDay() + 6) % 7));
    monday.setHours(0, 0, 0, 0);
    return {
      start: monday.toISOString(),
      end: now.toISOString(),
    };
  }

  async collect() {
    await this.collectSARIFMetrics();
    this.collectCoverageMetrics();
    this.collectScanDuration();
    this.updateTrend();
    this.saveReport();
  }

  async collectSARIFMetrics() {
    const reportsDir = path.join(process.cwd(), 'security', 'reports', 'downloads');

    if (!fs.existsSync(reportsDir)) return;

    const files = fs.readdirSync(reportsDir).filter(f => f.endsWith('.sarif'));
    for (const file of files) {
      try {
        const data = JSON.parse(fs.readFileSync(path.join(reportsDir, file), 'utf8'));

        for (const run of data.runs || []) {
          const toolName = run.tool?.driver?.name || file.replace('.sarif', '');

          for (const result of run.results || []) {
            const severity = this.mapSeverity(result.level || 'warning');
            this.currentMetrics.vulnerabilities[severity]++;
            this.currentMetrics.vulnerabilities.total++;

            const ruleId = result.ruleId || 'unknown';
            if (!this.currentMetrics.findings_by_type[toolName]) {
              this.currentMetrics.findings_by_type[toolName] = {};
            }
            if (!this.currentMetrics.findings_by_type[toolName][ruleId]) {
              this.currentMetrics.findings_by_type[toolName][ruleId] = 0;
            }
            this.currentMetrics.findings_by_type[toolName][ruleId]++;
          }

          this.currentMetrics.scans.push({
            tool: toolName,
            total_findings: run.results?.length || 0,
            timestamp: new Date().toISOString(),
          });
        }
      } catch (e) {
        console.warn(`Could not parse SARIF: ${file}`, e.message);
      }
    }
  }

  mapSeverity(level) {
    const map = { 'error': 'critical', 'warning': 'high', 'note': 'medium' };
    return map[level] || 'low';
  }

  collectCoverageMetrics() {
    this.currentMetrics.coverage = {
      sast: fs.existsSync(path.join(process.cwd(), 'security', 'reports', 'downloads', 'codeql-javascript-typescript.sarif')) ||
             fs.existsSync(path.join(process.cwd(), 'security', 'reports', 'downloads', 'semgrep.sarif')),
      dast: fs.existsSync(path.join(process.cwd(), 'security', 'reports', 'dast', 'zap-report.json')),
      sca: fs.existsSync(path.join(process.cwd(), 'security', 'reports', 'downloads', 'npm-audit-report.json')),
      container: fs.existsSync(path.join(process.cwd(), 'security', 'reports', 'downloads', 'trivy-fs.sarif')),
      iac: fs.existsSync(path.join(process.cwd(), 'security', 'reports', 'downloads', 'checkov.sarif')),
      secret: fs.existsSync(path.join(process.cwd(), 'security', 'reports', 'downloads', 'gitleaks.sarif')),
    };
  }

  collectScanDuration() {
    this.currentMetrics.scan_duration_ms = Date.now() - new Date(this.currentMetrics.period.start).getTime();
  }

  updateTrend() {
    let history = [];
    if (fs.existsSync(this.historyFile)) {
      try {
        history = JSON.parse(fs.readFileSync(this.historyFile, 'utf8'));
      } catch (e) {}
    }

    history.push(this.currentMetrics);

    if (history.length > 52) {
      history = history.slice(-52);
    }

    fs.mkdirSync(this.metricsDir, { recursive: true });
    fs.writeFileSync(this.historyFile, JSON.stringify(history, null, 2));

    this.generateTrendReport(history);
  }

  generateTrendReport(history) {
    if (history.length < 2) return;

    const latest = history[history.length - 1];
    const previous = history[history.length - 2];

    const trendReport = {
      generated: new Date().toISOString(),
      total_reports: history.length,
      current_period: latest.period,
      comparison: {
        critical_change: latest.vulnerabilities.critical - previous.vulnerabilities.critical,
        high_change: latest.vulnerabilities.high - previous.vulnerabilities.high,
        total_change: latest.vulnerabilities.total - previous.vulnerabilities.total,
      },
      averages: {
        weekly_critical: Math.round(history.reduce((sum, m) => sum + m.vulnerabilities.critical, 0) / history.length),
        weekly_high: Math.round(history.reduce((sum, m) => sum + m.vulnerabilities.high, 0) / history.length),
        weekly_total: Math.round(history.reduce((sum, m) => sum + m.vulnerabilities.total, 0) / history.length),
      },
      coverage_trend: history.map(m => ({
        date: m.timestamp,
        coverage: Object.values(m.coverage).filter(Boolean).length,
        total_possible: Object.keys(m.coverage).length,
      })),
    };

    fs.writeFileSync(
      path.join(this.metricsDir, 'trend-analysis.json'),
      JSON.stringify(trendReport, null, 2)
    );
  }

  saveReport() {
    fs.writeFileSync(
      path.join(this.metricsDir, 'current-metrics.json'),
      JSON.stringify(this.currentMetrics, null, 2)
    );

    const coveragePct = Object.values(this.currentMetrics.coverage).filter(Boolean).length /
      Object.keys(this.currentMetrics.coverage).length * 100;
    console.log(`Security coverage: ${coveragePct.toFixed(1)}%`);
    console.log(`Vulnerabilities: ${this.currentMetrics.vulnerabilities.total} total (${this.currentMetrics.vulnerabilities.critical} critical, ${this.currentMetrics.vulnerabilities.high} high)`);
  }
}

const collector = new SecurityMetricsCollector();
collector.collect().catch(console.error);
