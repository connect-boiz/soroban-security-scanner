const chalk = require('chalk');

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
      report += chalk.green('✅ No time-based attack vulnerabilities found!\n');
      return report;
    }

    report += chalk.red(`🚨 Found ${vulnerabilities.length} time-based attack vulnerabilities:\n\n`);

    const groupedVulns = this.groupBySeverity(vulnerabilities);
    
    for (const [severity, vulns] of Object.entries(groupedVulns)) {
      const color = this.getSeverityColor(severity);
      report += color(`${severity} SEVERITY (${vulns.length}):\n`);
      
      for (const vuln of vulns) {
        report += color(`  └── ${vuln.description}\n`);
        report += chalk.gray(`     File: ${vuln.file}:${vuln.line}\n`);
        report += chalk.yellow(`     Code: ${vuln.code}\n\n`);
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
    let recommendations = chalk.blue('📋 RECOMMENDATIONS:\n\n');
    
    if (groupedVulns.HIGH) {
      recommendations += chalk.red('HIGH PRIORITY:\n');
      recommendations += '1. Implement timestamp validation using trusted oracles\n';
      recommendations += '2. Add minimum/maximum timestamp bounds\n';
      recommendations += '3. Use block heights instead of timestamps when possible\n';
      recommendations += '4. Implement replay attack protection\n\n';
    }
    
    if (groupedVulns.MEDIUM) {
      recommendations += chalk.yellow('MEDIUM PRIORITY:\n');
      recommendations += '1. Add timestamp drift tolerance\n';
      recommendations += '2. Implement time window validation\n';
      recommendations += '3. Use monotonic counters for critical operations\n\n';
    }
    
    recommendations += chalk.blue('GENERAL PROTECTION MEASURES:\n');
    recommendations += '• Always validate timestamp ranges\n';
    recommendations += '• Use multiple time sources when possible\n';
    recommendations += '• Implement rate limiting for time-sensitive operations\n';
    recommendations += '• Consider using block numbers instead of timestamps\n';
    
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
