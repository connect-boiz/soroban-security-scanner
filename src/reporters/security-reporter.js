const chalk = require('chalk');
const { t } = require('./i18n/config');

class SecurityReporter {
  generate(vulnerabilities, format = 'text') {
    if (format === 'json') {
      return this.generateJsonReport(vulnerabilities);
    }
    return this.generateTextReport(vulnerabilities);
  }

  generateTextReport(vulnerabilities) {
    let report = '';
    
    if (vulnerabilities.length === 0) {
      report += chalk.green(t('reporter.no_vulnerabilities') + '\n');
      return report;
    }

    report += chalk.red(t('reporter.found_vulnerabilities', { count: vulnerabilities.length }) + '\n\n');

    const groupedVulns = this.groupBySeverity(vulnerabilities);
    
    for (const [severity, vulns] of Object.entries(groupedVulns)) {
      const color = this.getSeverityColor(severity);
      report += color(t('reporter.severity', { severity, count: vulns.length }) + '\n');
      
      for (const vuln of vulns) {
        report += color(`  └── [${vuln.type}] ${vuln.description}\n`);
        report += chalk.gray(t('reporter.file', { file: vuln.file, line: vuln.line }) + '\n');
        report += chalk.yellow(t('reporter.code', { code: vuln.code }) + '\n\n');
      }
    }

    report += this.generateRecommendations(groupedVulns);
    
    return report;
  }

  generateJsonReport(vulnerabilities) {
    const report = {
      scanDate: new Date().toISOString(),
      summary: {
        total: vulnerabilities.length,
        bySeverity: this.countBySeverity(vulnerabilities)
      },
      vulnerabilities: vulnerabilities,
      recommendations: this.getRecommendations(vulnerabilities)
    };
    
    return JSON.stringify(report, null, 2);
  }

  groupBySeverity(vulnerabilities) {
    return vulnerabilities.reduce((groups, vuln) => {
      if (!groups[vuln.severity]) {
        groups[vuln.severity] = [];
      }
      groups[vuln.severity].push(vuln);
      return groups;
    }, {});
  }

  countBySeverity(vulnerabilities) {
    const counts = {};
    for (const vuln of vulnerabilities) {
      counts[vuln.severity] = (counts[vuln.severity] || 0) + 1;
    }
    return counts;
  }

  getSeverityColor(severity) {
    switch (severity) {
      case 'HIGH': return chalk.red;
      case 'MEDIUM': return chalk.yellow;
      case 'LOW': return chalk.blue;
      default: return chalk.white;
    }
  }

  generateRecommendations(groupedVulns) {
    let recommendations = chalk.blue(t('reporter.recommendations') + '\n\n');
    
    if (groupedVulns.HIGH) {
      recommendations += chalk.red(t('reporter.high_priority') + '\n');
      recommendations += '1. ' + t('reporter.timestamp_validation') + '\n';
      recommendations += '2. ' + t('reporter.timestamp_bounds') + '\n';
      recommendations += '3. ' + t('reporter.block_heights') + '\n';
      recommendations += '4. ' + t('reporter.replay_protection') + '\n\n';
    }
    
    if (groupedVulns.MEDIUM) {
      recommendations += chalk.yellow(t('reporter.medium_priority') + '\n');
      recommendations += '1. ' + t('reporter.timestamp_drift') + '\n';
      recommendations += '2. ' + t('reporter.time_window') + '\n';
      recommendations += '3. ' + t('reporter.monotonic_counters') + '\n\n';
    }
    
    recommendations += chalk.blue(t('reporter.general_protection') + '\n');
    recommendations += '• ' + t('reporter.validate_ranges') + '\n';
    recommendations += '• ' + t('reporter.multiple_sources') + '\n';
    recommendations += '• ' + t('reporter.rate_limiting') + '\n';
    recommendations += '• ' + t('reporter.consider_blocks') + '\n';
    
    return recommendations;
  }

  getRecommendations(vulnerabilities) {
    const recommendations = [];
    
    if (vulnerabilities.some(v => v.severity === 'HIGH')) {
      recommendations.push({
        priority: 'HIGH',
        action: 'Implement timestamp validation and bounds checking',
        description: 'Add validation to ensure timestamps are within acceptable ranges'
      });
    }
    
    if (vulnerabilities.some(v => v.type.includes('LOCK_PERIOD'))) {
      recommendations.push({
        priority: 'HIGH', 
        action: 'Replace timestamp-based lock periods with block heights',
        description: 'Use block numbers or other non-manipulable time sources'
      });
    }
    
    return recommendations;
  }
}

module.exports = SecurityReporter;
