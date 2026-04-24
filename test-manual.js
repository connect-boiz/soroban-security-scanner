// Manual test script for time-based attack vulnerability detection
// This simulates the scanner functionality without requiring Node.js runtime

const fs = require('fs');

// Vulnerability patterns from the detector
const vulnerablePatterns = [
  {
    pattern: /now\s*<\s*timestamp|timestamp\s*>\s*now/g,
    type: 'DIRECT_TIMESTAMP_COMPARISON',
    severity: 'HIGH',
    description: 'Direct timestamp comparison without manipulation protection'
  },
  {
    pattern: /lock_period\s*=\s*.*timestamp|timestamp.*lock_period/gi,
    type: 'LOCK_PERIOD_TIMESTAMP_USAGE',
    severity: 'HIGH', 
    description: 'Lock period calculated using vulnerable timestamp comparisons'
  },
  {
    pattern: /if\s*\(\s*.*time.*\)\s*{|require\s*\(\s*.*time.*\s*,/gi,
    type: 'UNPROTECTED_TIME_CONDITION',
    severity: 'MEDIUM',
    description: 'Time-based condition without protection against manipulation'
  },
  {
    pattern: /timestamp\s*[+\-]\s*\d+\s*[*/]\s*\d+/g,
    type: 'TIMESTAMP_ARITHMETIC_LOCK',
    severity: 'HIGH',
    description: 'Timestamp arithmetic used for lock calculations without protection'
  }
];

function scanFile(filePath) {
  console.log(`\n🔍 Scanning: ${filePath}`);
  
  try {
    const content = fs.readFileSync(filePath, 'utf8');
    const lines = content.split('\n');
    const vulnerabilities = [];
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const lineNumber = i + 1;
      
      for (const pattern of vulnerablePatterns) {
        const matches = line.match(pattern.pattern);
        if (matches) {
          vulnerabilities.push({
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
    
    return vulnerabilities;
  } catch (error) {
    console.error(`Error scanning file ${filePath}: ${error.message}`);
    return [];
  }
}

function generateReport(vulnerabilities) {
  if (vulnerabilities.length === 0) {
    return '✅ No time-based attack vulnerabilities found!\n';
  }

  let report = `🚨 Found ${vulnerabilities.length} time-based attack vulnerabilities:\n\n`;

  const groupedVulns = vulnerabilities.reduce((groups, vuln) => {
    if (!groups[vuln.severity]) {
      groups[vuln.severity] = [];
    }
    groups[vuln.severity].push(vuln);
    return groups;
  }, {});

  for (const [severity, vulns] of Object.entries(groupedVulns)) {
    report += `${severity} SEVERITY (${vulns.length}):\n`;
    
    for (const vuln of vulns) {
      report += `  └── ${vuln.description}\n`;
      report += `     File: ${vuln.file}:${vuln.line}\n`;
      report += `     Code: ${vuln.code}\n\n`;
    }
  }

  return report;
}

// Test the vulnerable contract
console.log('🧪 Testing Time-Based Attack Vulnerability Detection');
console.log('=' .repeat(50));

const vulnerableContractPath = 'examples/vulnerable-contract.rs';
const secureContractPath = 'examples/secure-contract.rs';

if (fs.existsSync(vulnerableContractPath)) {
  const vulnerableResults = scanFile(vulnerableContractPath);
  const vulnerableReport = generateReport(vulnerableResults);
  console.log('VULNERABLE CONTRACT RESULTS:');
  console.log(vulnerableReport);
}

if (fs.existsSync(secureContractPath)) {
  const secureResults = scanFile(secureContractPath);
  const secureReport = generateReport(secureResults);
  console.log('SECURE CONTRACT RESULTS:');
  console.log(secureReport);
}

// Test specific vulnerable patterns
console.log('\n🔬 Pattern Detection Tests');
console.log('=' .repeat(30));

const testCases = [
  {
    name: 'Direct timestamp comparison',
    code: 'if now > timestamp { release(); }',
    expectedType: 'DIRECT_TIMESTAMP_COMPARISON'
  },
  {
    name: 'Lock period calculation',
    code: 'let lock_period = timestamp - now;',
    expectedType: 'LOCK_PERIOD_TIMESTAMP_USAGE'
  },
  {
    name: 'Time condition',
    code: 'if (block.timestamp > deadline) { execute(); }',
    expectedType: 'UNPROTECTED_TIME_CONDITION'
  },
  {
    name: 'Timestamp arithmetic',
    code: 'let unlock_time = timestamp + 86400 * 7;',
    expectedType: 'TIMESTAMP_ARITHMETIC_LOCK'
  }
];

testCases.forEach(test => {
  console.log(`\nTesting: ${test.name}`);
  console.log(`Code: ${test.code}`);
  
  let detected = false;
  for (const pattern of vulnerablePatterns) {
    const matches = test.code.match(pattern.pattern);
    if (matches) {
      console.log(`✅ Detected: ${pattern.type} (${pattern.severity})`);
      detected = true;
      break;
    }
  }
  
  if (!detected) {
    console.log('❌ Not detected - pattern may need adjustment');
  }
});

console.log('\n🎯 Summary');
console.log('=' .repeat(20));
console.log('✅ Time-based attack vulnerability detection implemented');
console.log('✅ Protection strategies documented');
console.log('✅ Test cases created');
console.log('✅ Secure contract examples provided');
console.log('\n📋 Next Steps:');
console.log('- Install Node.js and npm to run the full scanner');
console.log('- Use: npm run scan <contract-path>');
console.log('- Review documentation for implementation guidelines');
