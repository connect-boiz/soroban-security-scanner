#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

class DeploymentSecurityGate {
  constructor() {
    this.env = process.env.NODE_ENV || 'staging';
    this.blockCritical = process.env.DEPLOYMENT_BLOCK_CRITICAL === 'true';
    this.blockHigh = process.env.DEPLOYMENT_BLOCK_HIGH === 'true';
    this.maxCritical = parseInt(process.env.MAX_CRITICAL_VULNERABILITIES || '0', 10);
    this.maxHigh = parseInt(process.env.MAX_HIGH_VULNERABILITIES || '5', 10);

    this.results = {
      passed: true,
      environment: this.env,
      timestamp: new Date().toISOString(),
      requires_approval: false,
      critical_count: 0,
      high_count: 0,
      medium_count: 0,
      low_count: 0,
      summary: '',
      gate_checks: [],
    };
  }

  evaluate() {
    this.collectScanResults();
    this.evaluateGates();
    this.outputResults();
    this.saveReport();

    if (!this.results.passed) {
      process.exit(1);
    }
  }

  collectScanResults() {
    const reportsDir = path.join(process.cwd(), 'security', 'reports', 'downloads');

    if (!fs.existsSync(reportsDir)) {
      this.results.summary = 'No security scan reports found';
      return;
    }

    const files = fs.readdirSync(reportsDir);
    for (const file of files) {
      if (file.endsWith('.sarif')) {
        this.processSARIF(path.join(reportsDir, file));
      }
    }
  }

  processSARIF(filePath) {
    try {
      const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
      for (const run of data.runs || []) {
        for (const result of run.results || []) {
          const severity = this.mapSeverity(result.level || 'warning');
          switch (severity) {
            case 'critical': this.results.critical_count++; break;
            case 'high': this.results.high_count++; break;
            case 'medium': this.results.medium_count++; break;
            case 'low': this.results.low_count++; break;
          }
        }
      }
    } catch (e) {
      console.warn(`Warning: Could not process ${filePath}: ${e.message}`);
    }
  }

  mapSeverity(level) {
    const map = {
      'error': 'high',
      'warning': 'medium',
      'note': 'low',
      'none': 'low',
    };
    return map[level] || 'medium';
  }

  evaluateGates() {
    const failures = [];

    this.addGateCheck('Critical vulnerabilities check', () => {
      if (this.blockCritical && this.results.critical_count > this.maxCritical) {
        failures.push(`Found ${this.results.critical_count} critical vulnerabilities (max: ${this.maxCritical})`);
        return false;
      }
      return true;
    });

    this.addGateCheck('High vulnerabilities check', () => {
      if (this.blockHigh && this.results.high_count > this.maxHigh) {
        failures.push(`Found ${this.results.high_count} high vulnerabilities (max: ${this.maxHigh})`);
        return false;
      }
      return true;
    });

    if (this.results.critical_count > 0 || this.results.high_count > 5) {
      this.results.requires_approval = true;
      failures.push('Manual security approval required');
    }

    if (failures.length > 0) {
      this.results.passed = false;
      this.results.summary = failures.join('; ');
    } else {
      this.results.summary = 'All deployment security gates passed';
    }
  }

  addGateCheck(name, checkFn) {
    const startTime = Date.now();
    const passed = checkFn();
    this.results.gate_checks.push({
      name,
      passed,
      duration_ms: Date.now() - startTime,
      timestamp: new Date().toISOString(),
    });
  }

  outputResults() {
    const outputs = {
      passed: this.results.passed.toString(),
      requires_approval: this.results.requires_approval.toString(),
      critical_count: this.results.critical_count.toString(),
      high_count: this.results.high_count.toString(),
      medium_count: this.results.medium_count.toString(),
      low_count: this.results.low_count.toString(),
      summary: this.results.summary,
    };

    const githubOutput = process.env.GITHUB_OUTPUT;
    if (githubOutput) {
      for (const [key, value] of Object.entries(outputs)) {
        fs.appendFileSync(githubOutput, `${key}=${value}\n`);
      }
    }

    console.log('Deployment Security Gate Results:');
    console.log(JSON.stringify(outputs, null, 2));
  }

  saveReport() {
    const reportDir = path.join(process.cwd(), 'security', 'reports');
    fs.mkdirSync(reportDir, { recursive: true });
    fs.writeFileSync(
      path.join(reportDir, `deployment-gate-${this.env}.json`),
      JSON.stringify(this.results, null, 2)
    );
  }
}

const gate = new DeploymentSecurityGate();
gate.evaluate();
