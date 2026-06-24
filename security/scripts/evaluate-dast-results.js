#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

class DASTEvaluator {
  constructor() {
    this.zapReportPath = process.env.ZAP_REPORT_PATH || 'security/reports/dast/zap-report.json';
    this.results = {
      passed: true,
      timestamp: new Date().toISOString(),
      summary: '',
      findings: {
        high: 0,
        medium: 0,
        low: 0,
        informational: 0,
      },
      alerts: [],
    };
  }

  evaluate() {
    this.parseZAPReport();
    this.applyThresholds();
    this.saveReport();
    this.outputResults();
  }

  parseZAPReport() {
    if (!fs.existsSync(this.zapReportPath)) {
      this.results.summary = 'No ZAP report found. Scan may not have run.';
      return;
    }

    try {
      const report = JSON.parse(fs.readFileSync(this.zapReportPath, 'utf8'));
      const site = report.site?.[0];

      if (!site) {
        this.results.summary = 'No site data in ZAP report';
        return;
      }

      const alerts = site.alerts || [];
      for (const alert of alerts) {
        const riskCode = alert.riskcode || '0';
        const severity = this.mapRiskCode(riskCode);
        this.results.findings[severity]++;

        this.results.alerts.push({
          name: alert.name || alert.alert,
          severity,
          riskCode,
          description: alert.desc || alert.description,
          url: alert.instances?.[0]?.uri || '',
          solution: alert.solution || '',
        });
      }

      this.results.summary = `DAST scan found ${this.results.findings.high} high, ${this.results.findings.medium} medium, ${this.results.findings.low} low, ${this.results.findings.informational} informational findings`;
    } catch (e) {
      this.results.summary = `Failed to parse ZAP report: ${e.message}`;
      this.results.passed = false;
    }
  }

  mapRiskCode(code) {
    const map = {
      '0': 'informational',
      '1': 'low',
      '2': 'medium',
      '3': 'high',
    };
    return map[code] || 'informational';
  }

  applyThresholds() {
    if (this.results.findings.high > 0) {
      this.results.passed = false;
      this.results.summary += '. BLOCKED: High-severity findings detected.';
    }

    if (this.results.findings.medium > 10) {
      this.results.passed = false;
      this.results.summary += '. BLOCKED: Too many medium-severity findings.';
    }
  }

  saveReport() {
    const reportDir = path.join(process.cwd(), 'security', 'reports', 'dast');
    fs.mkdirSync(reportDir, { recursive: true });
    fs.writeFileSync(
      path.join(reportDir, 'dast-evaluation.json'),
      JSON.stringify(this.results, null, 2)
    );
  }

  outputResults() {
    const githubOutput = process.env.GITHUB_OUTPUT;
    if (githubOutput) {
      fs.appendFileSync(githubOutput, `passed=${this.results.passed}\n`);
      fs.appendFileSync(githubOutput, `summary=${this.results.summary}\n`);
    }
  }
}

const evaluator = new DASTEvaluator();
evaluator.evaluate();
