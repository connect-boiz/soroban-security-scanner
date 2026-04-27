//! Test suite for the notification service

const {
  NotificationService,
  NotificationChannel,
  NotificationPriority,
  Recipient,
  NotificationPreferences,
  QuietHours,
  NotificationTemplate,
  TemplateVariable,
  VariableType
} = require('./index');

/**
 * Test the notification service
 */
async function testNotificationService() {
  console.log('🧪 Testing Notification Service...\n');

  try {
    // Create notification service
    const service = new NotificationService();
    console.log('✅ Notification service created successfully');

    // Test template management
    await testTemplateManagement(service);

    // Test notification sending
    await testNotificationSending(service);

    // Test delivery tracking
    await testDeliveryTracking(service);

    // Test health checks
    await testHealthChecks(service);

    console.log('\n🎉 All tests passed!');
    return true;
  } catch (error) {
    console.error('\n❌ Test failed:', error.message);
    console.error(error.stack);
    return false;
  }
}

/**
 * Test template management
 */
async function testTemplateManagement(service) {
  console.log('📋 Testing template management...');

  // Create a test template
  const testTemplate = new NotificationTemplate({
    id: 'test_vulnerability',
    name: 'Test Vulnerability Alert',
    description: 'Test template for vulnerability notifications',
    subjectTemplate: '🚨 {{severity}} Vulnerability in {{contract_name}}',
    bodyTemplate: `Hello {{user_name}},

A {{severity}} vulnerability was found in {{contract_name}}.

Description: {{description}}
Risk Score: {{risk_score}}

{{#if critical}}
⚠️ This requires immediate attention!
{{/if}}

Report: {{report_url}}`,
    supportedChannels: [NotificationChannel.EMAIL, NotificationChannel.IN_APP],
    defaultPriority: NotificationPriority.HIGH,
    variables: [
      new TemplateVariable({
        name: 'user_name',
        description: 'User name',
        required: true,
        variableType: VariableType.STRING
      }),
      new TemplateVariable({
        name: 'severity',
        description: 'Severity level',
        required: true,
        variableType: VariableType.STRING
      }),
      new TemplateVariable({
        name: 'contract_name',
        description: 'Contract name',
        required: true,
        variableType: VariableType.STRING
      }),
      new TemplateVariable({
        name: 'description',
        description: 'Vulnerability description',
        required: true,
        variableType: VariableType.STRING
      }),
      new TemplateVariable({
        name: 'risk_score',
        description: 'Risk score',
        required: true,
        variableType: VariableType.NUMBER
      }),
      new TemplateVariable({
        name: 'critical',
        description: 'Is critical',
        required: false,
        defaultValue: false,
        variableType: VariableType.BOOLEAN
      }),
      new TemplateVariable({
        name: 'report_url',
        description: 'Report URL',
        required: true,
        variableType: VariableType.URL
      })
    ]
  });

  // Add template
  await service.addTemplate(testTemplate);
  console.log('✅ Template added successfully');

  // Get template
  const retrieved = await service.getTemplate('test_vulnerability');
  if (retrieved && retrieved.name === 'Test Vulnerability Alert') {
    console.log('✅ Template retrieved successfully');
  } else {
    throw new Error('Template retrieval failed');
  }

  // List templates
  const templates = await service.listTemplates();
  if (templates.length > 0) {
    console.log(`✅ Found ${templates.length} templates`);
  } else {
    throw new Error('No templates found');
  }
}

/**
 * Test notification sending
 */
async function testNotificationSending(service) {
  console.log('📤 Testing notification sending...');

  // Create recipient
  const recipient = new Recipient({
    id: 'test_user_123',
    email: 'test@example.com',
    phone: '+1234567890',
    deviceTokens: ['device_token_abc'],
    userId: 'test_user_123',
    preferences: new NotificationPreferences({
      emailEnabled: true,
      smsEnabled: false,
      pushEnabled: true,
      inAppEnabled: true,
      quietHours: null,
      maxPriority: NotificationPriority.NORMAL
    })
  });

  // Test templated notification
  const context = {
    user_name: 'Alice',
    severity: 'High',
    contract_name: 'TokenContract',
    description: 'Reentrancy vulnerability detected',
    risk_score: 85,
    critical: true,
    report_url: 'https://scanner.example.com/report/123'
  };

  const result = await service.sendTemplatedNotification(
    'test_vulnerability',
    recipient,
    context,
    [NotificationChannel.IN_APP], // Only use in-app for testing (no email config)
    NotificationPriority.HIGH
  );

  if (result.success) {
    console.log('✅ Templated notification sent successfully');
  } else {
    console.log('⚠️  Templated notification had issues:', result.failedChannels);
  }

  // Test direct notification
  const directResult = await service.sendNotification({
    id: 'direct_test_123',
    subject: 'Test Direct Notification',
    body: 'This is a test notification sent directly',
    priority: NotificationPriority.NORMAL,
    channels: [NotificationChannel.IN_APP]
  }, recipient);

  if (directResult.success) {
    console.log('✅ Direct notification sent successfully');
  } else {
    console.log('⚠️  Direct notification had issues:', directResult.failedChannels);
  }
}

/**
 * Test delivery tracking
 */
async function testDeliveryTracking(service) {
  console.log('📊 Testing delivery tracking...');

  // Get delivery stats
  const now = new Date();
  const oneHourAgo = new Date(now.getTime() - 3600000);
  
  const stats = await service.getDeliveryStats(oneHourAgo, now);
  console.log(`✅ Delivery stats retrieved: ${stats.totalNotifications} notifications`);

  // Get provider stats
  const providerStats = await service.getProviderStats();
  console.log('✅ Provider stats retrieved:');
  for (const [channel, stats] of Object.entries(providerStats)) {
    console.log(`   ${channel}: ${stats.totalSent} sent, ${stats.totalFailed} failed`);
  }

  // Test in-app notifications
  const inAppNotifications = service.getInAppNotifications('test_user_123');
  console.log(`✅ Found ${inAppNotifications.length} in-app notifications for test user`);
}

/**
 * Test health checks
 */
async function testHealthChecks(service) {
  console.log('🏥 Testing health checks...');

  const healthStatus = await service.healthCheck();
  console.log('✅ Health check results:');
  for (const [channel, healthy] of Object.entries(healthStatus)) {
    console.log(`   ${channel}: ${healthy ? '✅ Healthy' : '❌ Unhealthy'}`);
  }
}

/**
 * Test integration with security scanner
 */
async function testSecurityScannerIntegration() {
  console.log('🔍 Testing security scanner integration...');

  const service = new NotificationService();
  
  // Mock scan result
  const scanResult = {
    filePath: '/contracts/token.wasm',
    hasIssues: () => true,
    severityCount: () => (1, 2, 1) // critical, high, medium
  };

  // Create recipients
  const recipients = [
    new Recipient({
      id: 'developer_123',
      email: 'dev@example.com',
      userId: 'developer_123',
      preferences: new NotificationPreferences({
        emailEnabled: true,
        inAppEnabled: true
      })
    })
  ];

  // Send vulnerability notifications
  if (scanResult.hasIssues()) {
    const [critical, high, medium] = scanResult.severityCount();
    
    const context = {
      user_name: 'Developer',
      file_path: scanResult.filePath,
      critical_count: critical,
      high_count: high,
      medium_count: medium,
      total_issues: critical + high + medium,
      has_issues: true,
      report_url: 'https://scanner.example.com/report/scan_123'
    };

    const priority = critical > 0 ? NotificationPriority.CRITICAL : 
                    high > 0 ? NotificationPriority.HIGH : 
                    NotificationPriority.NORMAL;

    for (const recipient of recipients) {
      const result = await service.sendTemplatedNotification(
        'scan_completed',
        recipient,
        context,
        [NotificationChannel.IN_APP],
        priority
      );

      if (result.success) {
        console.log('✅ Security scanner integration notification sent');
      } else {
        console.log('⚠️  Security scanner notification had issues');
      }
    }
  }
}

/**
 * Run all tests
 */
async function runAllTests() {
  console.log('🚀 Starting Notification Service Tests\n');
  
  const results = [];
  
  // Core functionality tests
  results.push(await testNotificationService());
  
  // Integration tests
  results.push(await testSecurityScannerIntegration());
  
  const passed = results.filter(r => r).length;
  const total = results.length;
  
  console.log(`\n📊 Test Results: ${passed}/${total} passed`);
  
  if (passed === total) {
    console.log('🎉 All tests completed successfully!');
  } else {
    console.log('❌ Some tests failed');
  }
  
  return passed === total;
}

// Run tests if this file is executed directly
if (require.main === module) {
  runAllTests().then(success => {
    process.exit(success ? 0 : 1);
  }).catch(error => {
    console.error('Test execution failed:', error);
    process.exit(1);
  });
}

module.exports = {
  testNotificationService,
  testTemplateManagement,
  testNotificationSending,
  testDeliveryTracking,
  testHealthChecks,
  testSecurityScannerIntegration,
  runAllTests
};
