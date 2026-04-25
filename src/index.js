#!/usr/bin/env node

const { Command } = require('commander');
const chalk = require('chalk');
const fs = require('fs');
const path = require('path');
const TimeBasedAttackDetector = require('./detectors/time-based-attack-detector');
const SecurityReporter = require('./reporters/security-reporter');

// Emergency stop functionality
let emergencyStopActive = false;
const emergencyStop = {
  isActive: () => emergencyStopActive,
  trigger: (reason) => {
    emergencyStopActive = true;
    console.log(chalk.red(`🛑 EMERGENCY STOP TRIGGERED: ${reason}`));
  },
  reset: () => {
    emergencyStopActive = false;
  }
};

// Setup signal handlers for graceful shutdown
process.on('SIGINT', () => {
  emergencyStop.trigger('SIGINT (Ctrl+C) received');
  process.exit(130);
});

process.on('SIGTERM', () => {
  emergencyStop.trigger('SIGTERM received');
  process.exit(143);
});

const program = new Command();

program
  .name('soroban-security-scanner')
  .description('Security scanner for Soroban smart contracts')
  .version('1.0.0');

program
  .command('scan')
  .description('Scan for security vulnerabilities')
  .argument('<contract-path>', 'Path to Soroban contract file or directory')
  .option('-o, --output <file>', 'Output report to file')
  .option('-f, --format <format>', 'Report format (json, text)', 'text')
  .option('--emergency-stop', 'Enable emergency stop on critical vulnerabilities')
  .action(async (contractPath, options) => {
    try {
      console.log(chalk.blue('🔍 Starting Soroban Security Scanner...'));
      
      if (options.emergencyStop) {
        console.log(chalk.yellow('🛑 Emergency stop system enabled'));
      }
      
      const detector = new TimeBasedAttackDetector();
      const reporter = new SecurityReporter();
      
      // Scan for time-based attack vulnerabilities
      const vulnerabilities = await detector.scan(contractPath, emergencyStop);
      
      // Check for emergency stop
      if (emergencyStop.isActive()) {
        console.log(chalk.yellow('⚠️  Scan was stopped due to emergency condition'));
        if (vulnerabilities.length > 0) {
          console.log(chalk.yellow(`📊 Partial results: ${vulnerabilities.length} vulnerabilities found`));
        }
        return;
      }
      
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

// Emergency stop management commands
program
  .command('emergency-stop')
  .description('Emergency stop management')
  .addCommand(
    new Command('trigger')
      .description('Trigger emergency stop manually')
      .option('-r, --reason <reason>', 'Reason for emergency stop')
      .action((options) => {
        const reason = options.reason || 'Manual trigger';
        emergencyStop.trigger(reason);
        console.log(chalk.red(`🛑 Emergency stop triggered: ${reason}`));
      })
  )
  .addCommand(
    new Command('status')
      .description('Check emergency stop status')
      .action(() => {
        if (emergencyStop.isActive()) {
          console.log(chalk.red('🔴 Emergency stop is ACTIVE'));
        } else {
          console.log(chalk.green('🟢 Emergency stop is INACTIVE'));
        }
      })
  )
  .addCommand(
    new Command('test')
      .description('Test emergency stop functionality')
      .action(() => {
        console.log(chalk.blue('🧪 Testing emergency stop functionality...'));
        
        // Test initial state
        if (emergencyStop.isActive()) {
          console.log(chalk.red('❌ Emergency stop should be inactive initially'));
          return;
        }
        
        // Test trigger
        emergencyStop.trigger('Test trigger');
        if (!emergencyStop.isActive()) {
          console.log(chalk.red('❌ Emergency stop should be active after trigger'));
          return;
        }
        
        console.log(chalk.green('✅ Emergency stop test passed'));
        
        // Reset after test
        emergencyStop.reset();
      })
  );

// Legacy support for direct argument
program
  .argument('[contract-path]', 'Path to Soroban contract file or directory (deprecated, use "scan" command)')
  .option('-o, --output <file>', 'Output report to file')
  .option('-f, --format <format>', 'Report format (json, text)', 'text')
  .action(async (contractPath, options) => {
    if (contractPath) {
      console.log(chalk.yellow('⚠️  Direct argument usage is deprecated. Use "scan" command instead.'));
      // Delegate to scan command
      await program.commands.find(cmd => cmd.name() === 'scan').action(contractPath, options);
    } else {
      program.help();
    }
  });

program.parse();
