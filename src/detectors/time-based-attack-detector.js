const fs = require('fs');
const path = require('path');

class TimeBasedAttackDetector {
  constructor() {
    this.vulnerabilities = [];
    this.vulnerablePatterns = [
      // Direct timestamp comparisons without protection
      {
        pattern: /now\s*<\s*timestamp|timestamp\s*>\s*now/g,
        type: 'DIRECT_TIMESTAMP_COMPARISON',
        severity: 'HIGH',
        description: 'Direct timestamp comparison without manipulation protection'
      },
      // Lock period calculations using timestamps
      {
        pattern: /lock_period\s*=\s*.*timestamp|timestamp.*lock_period/gi,
        type: 'LOCK_PERIOD_TIMESTAMP_USAGE',
        severity: 'HIGH', 
        description: 'Lock period calculated using vulnerable timestamp comparisons'
      },
      // Time-based conditions without validation
      {
        pattern: /if\s*\(\s*.*time.*\)\s*{|require\s*\(\s*.*time.*\s*,/gi,
        type: 'UNPROTECTED_TIME_CONDITION',
        severity: 'MEDIUM',
        description: 'Time-based condition without protection against manipulation'
      },
      // Timestamp arithmetic for locks
      {
        pattern: /timestamp\s*[+\-]\s*\d+\s*[*/]\s*\d+/g,
        type: 'TIMESTAMP_ARITHMETIC_LOCK',
        severity: 'HIGH',
        description: 'Timestamp arithmetic used for lock calculations without protection'
      }
    ];
  }

  async scan(contractPath) {
    this.vulnerabilities = [];
    
    if (fs.statSync(contractPath).isDirectory()) {
      await this.scanDirectory(contractPath);
    } else {
      await this.scanFile(contractPath);
    }
    
    return this.vulnerabilities;
  }

  async scanDirectory(dirPath) {
    const files = fs.readdirSync(dirPath);
    
    for (const file of files) {
      const filePath = path.join(dirPath, file);
      const stat = fs.statSync(filePath);
      
      if (stat.isDirectory()) {
        await this.scanDirectory(filePath);
      } else if (this.isContractFile(file)) {
        await this.scanFile(filePath);
      }
    }
  }

  async scanFile(filePath) {
    try {
      const content = fs.readFileSync(filePath, 'utf8');
      const lines = content.split('\n');
      
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const lineNumber = i + 1;
        
        for (const pattern of this.vulnerablePatterns) {
          const matches = line.match(pattern.pattern);
          if (matches) {
            this.vulnerabilities.push({
              type: pattern.type,
              severity: pattern.severity,
              description: pattern.description,
              file: filePath,
              line: lineNumber,
              code: line.trim(),
              matches: matches
            });
          }
        }
      }
    } catch (error) {
      console.error(`Error scanning file ${filePath}: ${error.message}`);
    }
  }

  isContractFile(filename) {
    const contractExtensions = ['.rs', '.js', '.ts', '.sol'];
    return contractExtensions.some(ext => filename.endsWith(ext));
  }
}

module.exports = TimeBasedAttackDetector;
