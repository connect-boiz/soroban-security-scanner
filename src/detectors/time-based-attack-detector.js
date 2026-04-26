const fs = require('fs');
const path = require('path');

class TimeBasedAttackDetector {
  constructor() {
    // Fix #87: Do not store mutable scan state on the instance.
    // Each scan() call creates its own local array, preventing memory leaks
    // from accumulated results across multiple scans.
    this.vulnerablePatterns = [
      // Direct timestamp comparisons without protection (any direction, any variable names)
      {
        pattern: /\b(?:now|current_timestamp|block\.timestamp)\s*[<>]=?\s*\w+|\b\w+\s*[<>]=?\s*(?:now|current_timestamp|block\.timestamp)/,
        type: 'DIRECT_TIMESTAMP_COMPARISON',
        severity: 'HIGH',
        description: 'Direct timestamp comparison without manipulation protection'
      },
      // Lock period calculations using timestamps
      {
        pattern: /lock_period\s*=\s*.*timestamp|timestamp.*lock_period|lock_end\s*=\s*.*timestamp|current_timestamp\s*\+\s*\w+/i,
        type: 'LOCK_PERIOD_TIMESTAMP_USAGE',
        severity: 'HIGH',
        description: 'Lock period calculated using vulnerable timestamp comparisons'
      },
      // Time-based conditions without validation
      {
        pattern: /if\s*\(\s*.*time.*\)\s*{|require\s*\(\s*.*time.*\s*,/i,
        type: 'UNPROTECTED_TIME_CONDITION',
        severity: 'MEDIUM',
        description: 'Time-based condition without protection against manipulation'
      },
      // Timestamp arithmetic for locks
      {
        pattern: /(?:timestamp|current_timestamp|lock_end)\s*[+\-]\s*\d+(?:\s*[*/]\s*\d+)?/,
        type: 'TIMESTAMP_ARITHMETIC_LOCK',
        severity: 'HIGH',
        description: 'Timestamp arithmetic used for lock calculations without protection'
      }
    ];
  }

  async scan(contractPath, emergencyStop) {
    // Fix #87: Use a local array per scan invocation instead of shared instance state.
    // This prevents unbounded memory growth when scan() is called multiple times.
    const vulnerabilities = [];

    if (fs.statSync(contractPath).isDirectory()) {
      await this._scanDirectory(contractPath, vulnerabilities, emergencyStop);
    } else {
      await this._scanFile(contractPath, vulnerabilities);
    }

    return vulnerabilities;
  }

  async _scanDirectory(dirPath, vulnerabilities, emergencyStop) {
    const files = fs.readdirSync(dirPath);

    for (const file of files) {
      if (emergencyStop && emergencyStop.isActive()) break;

      const filePath = path.join(dirPath, file);
      const stat = fs.statSync(filePath);

      if (stat.isDirectory()) {
        await this._scanDirectory(filePath, vulnerabilities, emergencyStop);
      } else if (this._isContractFile(file)) {
        await this._scanFile(filePath, vulnerabilities);
      }
    }
  }

  async _scanFile(filePath, vulnerabilities) {
    try {
      const content = fs.readFileSync(filePath, 'utf8');
      const lines = content.split('\n');

      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const lineNumber = i + 1;

        for (const { pattern, type, severity, description } of this.vulnerablePatterns) {
          // Fix #87: Patterns are defined without the /g flag so they are stateless
          // and safe to reuse across lines without resetting lastIndex.
          const matches = line.match(pattern);
          if (matches) {
            vulnerabilities.push({
              type,
              severity,
              description,
              file: filePath,
              line: lineNumber,
              code: line.trim(),
              matches
            });
          }
        }
      }
    } catch (error) {
      console.error(`Error scanning file ${filePath}: ${error.message}`);
    }
  }

  _isContractFile(filename) {
    const contractExtensions = ['.rs', '.js', '.ts', '.sol'];
    return contractExtensions.some(ext => filename.endsWith(ext));
  }
}

module.exports = TimeBasedAttackDetector;
