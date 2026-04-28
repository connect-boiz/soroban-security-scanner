#!/usr/bin/env node

const { Command } = require('commander');
const chalk = require('chalk');
const fs = require('fs');
const path = require('path');
const TimeBasedAttackDetector = require('./detectors/time-based-attack-detector');
const SecurityReporter = require('./reporters/security-reporter');
const { NotificationService, NotificationChannel, NotificationPriority, Recipient, NotificationPreferences } = require('./notification-service');
const { initializeI18n, t, getTextDirection, formatCurrency, formatDate } = require('./i18n/config');

// Notification service
let notificationService = null;

// Initialize notification service
async function initializeNotificationService() {
  if (!notificationService) {
    notificationService = new NotificationService();
    console.log(chalk.blue(t('notification_service.initialized')));
  }
  return notificationService;
}

// Send scan notifications
async function sendScanNotifications(service, contractPath, vulnerabilities, userEmail) {
  try {
    const recipient = new Recipient({
      id: 'scanner_user',
      email: userEmail || 'user@example.com',
      userId: 'scanner_user',
      preferences: new NotificationPreferences({
        emailEnabled: true,
        inAppEnabled: true,
        smsEnabled: false,
        pushEnabled: false
      })
    });

    const context = {
      user_name: 'Security Scanner User',
      file_path: contractPath,
      total_issues: vulnerabilities.length,
      critical_count: vulnerabilities.filter(v => v.severity === 'critical').length,
      high_count: vulnerabilities.filter(v => v.severity === 'high').length,
      medium_count: vulnerabilities.filter(v => v.severity === 'medium').length,
      has_issues: vulnerabilities.length > 0,
      report_url: 'https://scanner.example.com/report/' + Date.now()
    };

    const priority = vulnerabilities.some(v => v.severity === 'critical') 
      ? NotificationPriority.CRITICAL 
      : vulnerabilities.some(v => v.severity === 'high')
      ? NotificationPriority.HIGH
      : NotificationPriority.NORMAL;

    const result = await service.sendTemplatedNotification(
      'scan_completed',
      recipient,
      context,
      [NotificationChannel.IN_APP, NotificationChannel.EMAIL],
      priority
    );

    if (result.success) {
      console.log(chalk.green(t('commands.scan.notifications_sent')));
    } else {
      console.log(chalk.yellow(t('commands.scan.some_notifications_failed', { channels: result.failedChannels })));
    }
  } catch (error) {
    console.error(chalk.red(t('commands.scan.notification_error', { error: error.message })));
  }
}

// Emergency stop functionality
let emergencyStopActive = false;
const emergencyStop = {
  isActive: () => emergencyStopActive,
  trigger: (reason) => {
    emergencyStopActive = true;
    console.log(chalk.red(t('commands.emergency_stop.triggered', { reason })));
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
  .description(t('scanner.description'))
  .version('1.0.0');

program
  .command('scan')
  .description(t('commands.scan.description'))
  .argument('<contract-path>', 'Path to Soroban contract file or directory')
  .option('-o, --output <file>', 'Output report to file')
  .option('-f, --format <format>', 'Report format (json, text)', 'text')
  .option('--emergency-stop', 'Enable emergency stop on critical vulnerabilities')
  .option('--notify', 'Enable notifications for scan results')
  .option('--notify-email <email>', 'Email address for notifications')
  .action(async (contractPath, options) => {
    try {
      console.log(chalk.blue(t('commands.scan.starting')));
      
      if (options.emergencyStop) {
        console.log(chalk.yellow(t('commands.scan.emergency_stop_enabled')));
      }
      
      // Initialize notification service if notifications are enabled
      let notificationService = null;
      if (options.notify) {
        notificationService = await initializeNotificationService();
        console.log(chalk.blue(t('commands.scan.notifications_enabled')));
      }
      
      const detector = new TimeBasedAttackDetector();
      const reporter = new SecurityReporter();
      
      // Scan for time-based attack vulnerabilities
      const vulnerabilities = await detector.scan(contractPath, emergencyStop);
      
      // Check for emergency stop
      if (emergencyStop.isActive()) {
        console.log(chalk.yellow(t('commands.scan.scan_stopped')));
        if (vulnerabilities.length > 0) {
          console.log(chalk.yellow(t('commands.scan.partial_results', { count: vulnerabilities.length })));
        }
        return;
      }
      
      // Generate report
      const report = reporter.generate(vulnerabilities, options.format);
      
      if (options.output) {
        fs.writeFileSync(options.output, report);
        console.log(chalk.green(t('commands.scan.report_saved', { file: options.output })));
      } else {
        console.log(report);
      }
      
      // Send notifications if enabled
      if (notificationService && vulnerabilities.length > 0) {
        await sendScanNotifications(notificationService, contractPath, vulnerabilities, options.notifyEmail);
      }
      
      // Exit with error code if vulnerabilities found
      if (vulnerabilities.length > 0) {
        process.exit(1);
      }
      
    } catch (error) {
      console.error(chalk.red(t('commands.scan.error', { error: error.message })));
      process.exit(1);
    }
  });

// Emergency stop management commands
program
  .command('emergency-stop')
  .description(t('commands.emergency_stop.description'))
  .addCommand(
    new Command('trigger')
      .description('Trigger emergency stop manually')
      .option('-r, --reason <reason>', 'Reason for emergency stop')
      .action((options) => {
        const reason = options.reason || 'Manual trigger';
        emergencyStop.trigger(reason);
        console.log(chalk.red(t('commands.emergency_stop.manual_trigger', { reason })));
      })
  )
  .addCommand(
    new Command('status')
      .description('Check emergency stop status')
      .action(() => {
        if (emergencyStop.isActive()) {
          console.log(chalk.red(t('commands.emergency_stop.status_active')));
        } else {
          console.log(chalk.green(t('commands.emergency_stop.status_inactive')));
        }
      })
  )
  .addCommand(
    new Command('test')
      .description('Test emergency stop functionality')
      .action(() => {
        console.log(chalk.blue(t('commands.emergency_stop.testing')));
        
        // Test initial state
        if (emergencyStop.isActive()) {
          console.log(chalk.red(t('commands.emergency_stop.test_initial_failed')));
          return;
        }
        
        // Test trigger
        emergencyStop.trigger('Test trigger');
        if (!emergencyStop.isActive()) {
          console.log(chalk.red(t('commands.emergency_stop.test_trigger_failed')));
          return;
        }
        
        console.log(chalk.green(t('commands.emergency_stop.test_passed')));
        
        // Reset after test
        emergencyStop.reset();
      })
  );

// Notification management commands
program
  .command('notifications')
  .description(t('commands.notifications.description'))
  .addCommand(
    new Command('test')
      .description(t('commands.notifications.test.description'))
      .option('--email <email>', 'Email address for testing')
      .action(async (options) => {
        try {
          const service = initializeNotificationService();
          
          const recipient = new Recipient({
            id: 'test_user',
            email: options.email || 'test@example.com',
            userId: 'test_user',
            preferences: new NotificationPreferences({
              emailEnabled: true,
              inAppEnabled: true,
              smsEnabled: false,
              pushEnabled: false
            })
          });

          const context = {
            user_name: 'Test User',
            severity: 'High',
            contract_name: 'TestContract',
            description: 'Test vulnerability for demonstration',
            risk_score: 75,
            critical: true,
            report_url: 'https://scanner.example.com/test'
          };

          const result = await service.sendTemplatedNotification(
            'vulnerability_alert',
            recipient,
            context,
            [NotificationChannel.IN_APP],
            NotificationPriority.HIGH
          );

          if (result.success) {
            console.log(chalk.green(t('commands.notifications.test.sent')));
            console.log(chalk.blue(t('commands.notifications.test.delivered_to', { channels: result.deliveredChannels })));
          } else {
            console.log(chalk.red(t('commands.notifications.test.failed')));
            console.log(chalk.yellow(t('commands.notifications.test.failed_channels'), result.failedChannels));
          }

          // Show provider health
          const health = await service.healthCheck();
          console.log(chalk.blue('\n' + t('commands.notifications.provider_health')));
          for (const [channel, healthy] of Object.entries(health)) {
            console.log(`  ${channel}: ${healthy ? '✅' : '❌'}`);
          }

          // Show statistics
          const stats = await service.getProviderStats();
          console.log(chalk.blue('\n' + t('commands.notifications.provider_stats')));
          for (const [channel, channelStats] of Object.entries(stats)) {
            console.log(`  ${channel}: ${channelStats.totalSent} sent, ${channelStats.totalFailed} failed`);
          }

        } catch (error) {
          console.error(chalk.red(t('commands.notifications.test.test_failed', { error: error.message })));
        }
      })
  )
  .addCommand(
    new Command('templates')
      .description(t('commands.notifications.templates.description'))
      .action(async () => {
        try {
          const service = initializeNotificationService();
          const templates = await service.listTemplates();
          
          console.log(chalk.blue(t('commands.notifications.templates.available')));
          for (const template of templates) {
            console.log(`\n  ${t('commands.notifications.templates.template_info', { name: template.name, id: template.id })}`);
            console.log(`     ${template.description || t('commands.notifications.templates.no_description')}`);
            console.log(`     ${t('commands.notifications.templates.channels', { channels: template.supportedChannels.join(', ') })}`);
            console.log(`     ${t('commands.notifications.templates.variables', { variables: template.variables.map(v => v.name).join(', ') })}`);
          }
        } catch (error) {
          console.error(chalk.red(t('commands.notifications.templates.list_failed', { error: error.message })));
        }
      })
  )
  .addCommand(
    new Command('in-app')
      .description('Manage in-app notifications')
      .addCommand(
        new Command('list')
          .description(t('commands.notifications.in_app.list.description'))
          .argument('<user-id>', 'User ID')
          .action(async (userId) => {
            try {
              const service = initializeNotificationService();
              const notifications = service.getInAppNotifications(userId);
              
              console.log(chalk.blue(t('commands.notifications.in_app.list.notifications_for', { userId })));
              if (notifications.length === 0) {
                console.log(t('commands.notifications.in_app.list.no_notifications'));
              } else {
                for (const notification of notifications) {
                  console.log(`\n  ${notification.read ? '✅' : '🔵'} ${notification.title}`);
                  console.log(`     ${notification.body}`);
                  console.log(`     Created: ${notification.createdAt.toLocaleString()}`);
                }
              }
            } catch (error) {
              console.error(chalk.red(t('commands.notifications.in_app.list.list_failed', { error: error.message })));
            }
          })
      )
  );

// Legacy support for direct argument
program
  .argument('[contract-path]', 'Path to Soroban contract file or directory (deprecated, use "scan" command)')
  .option('-o, --output <file>', 'Output report to file')
  .option('-f, --format <format>', 'Report format (json, text)', 'text')
  .action(async (contractPath, options) => {
    if (contractPath) {
      console.log(chalk.yellow(t('errors.deprecated_usage')));
      // Delegate to scan command
      await program.commands.find(cmd => cmd.name() === 'scan').action(contractPath, options);
    } else {
      program.help();
    }
  });

// Main function to initialize i18n and start the CLI
async function main() {
  try {
    // Initialize i18n first
    await initializeI18n();
    
    // Parse command line arguments
    program.parse();
  } catch (error) {
    console.error(chalk.red('❌ Failed to initialize:', error.message));
    process.exit(1);
  }
}

// Run the main function
main();
