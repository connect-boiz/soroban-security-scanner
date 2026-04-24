#!/usr/bin/env node

const { Command } = require('commander');
const chalk = require('chalk');
const fs = require('fs');
const path = require('path');
const TimeBasedAttackDetector = require('./detectors/time-based-attack-detector');
const SecurityReporter = require('./reporters/security-reporter');

const program = new Command();

program
  .name('soroban-security-scanner')
  .description('Security scanner for Soroban smart contracts')
  .version('1.0.0')
  .argument('<contract-path>', 'Path to Soroban contract file or directory')
  .option('-o, --output <file>', 'Output report to file')
  .option('-f, --format <format>', 'Report format (json, text)', 'text')
  .action(async (contractPath, options) => {
    try {
      console.log(chalk.blue('🔍 Starting Soroban Security Scanner...'));
      
      const detector = new TimeBasedAttackDetector();
      const reporter = new SecurityReporter();
      
      // Scan for time-based attack vulnerabilities
      const vulnerabilities = await detector.scan(contractPath);
      
      // Generate report
      const report = reporter.generate(vulnerabilities, options.format);
      
      if (options.output) {
        fs.writeFileSync(options.output, report);
        console.log(chalk.green(`📄 Report saved to ${options.output}`));
      } else {
        console.log(report);
      }
      
      // Exit with error code if vulnerabilities found
      if (vulnerabilities.length > 0) {
        process.exit(1);
      }
      
    } catch (error) {
      console.error(chalk.red(`❌ Error: ${error.message}`));
      process.exit(1);
    }
  });

program.parse();
