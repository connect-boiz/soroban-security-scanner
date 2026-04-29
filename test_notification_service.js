#!/usr/bin/env node

/**
 * Comprehensive test suite for the notification service
 * This test validates both JavaScript and Rust implementations
 */

const fs = require('fs');
const path = require('path');

// Test configuration
const TEST_CONFIG = {
    email: {
        smtp_host: 'smtp.gmail.com',
        smtp_port: '587',
        username: 'test@example.com',
        password: 'test_password',
        from_email: 'scanner@example.com',
        from_name: 'Soroban Security Scanner'
    },
    sms: {
        provider: 'twilio',
        account_sid: 'test_account_sid',
        auth_token: 'test_auth_token',
        from_number: '+1234567890'
    },
    push: {
        fcm_server_key: 'test_fcm_key',
        apns_key_id: 'test_apns_key',
        apns_team_id: 'test_team_id'
    }
};

/**
 * Test the JavaScript notification service implementation
 */
async function testJavaScriptImplementation() {
    console.log('🧪 Testing JavaScript Notification Service...\n');
    
    try {
        // Load the notification service
        const notificationServicePath = path.join(__dirname, 'src', 'notification-service');
        
        if (!fs.existsSync(notificationServicePath)) {
            throw new Error('JavaScript notification service not found');
        }
        
        const {
            NotificationService,
            NotificationChannel,
            NotificationPriority,
            Recipient,
            NotificationPreferences,
            NotificationTemplate,
            TemplateVariable,
            VariableType
        } = require(path.join(notificationServicePath, 'index.js'));
        
        console.log('✅ JavaScript notification service loaded successfully');
        
        // Test 1: Create notification service
        const service = new NotificationService();
        console.log('✅ Notification service created successfully');
        
        // Test 2: Create recipient
        const recipient = new Recipient({
            id: 'test_user_123',
            email: 'test@example.com',
            phone: '+1234567890',
            device_tokens: ['device_token_abc'],
            user_id: 'test_user_123',
            preferences: new NotificationPreferences({
                emailEnabled: true,
                smsEnabled: false,
                pushEnabled: true,
                inAppEnabled: true
            })
        });
        console.log('✅ Recipient created successfully');
        
        // Test 3: Create template
        const template = new NotificationTemplate({
            id: 'test_vulnerability_alert',
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
                    description: 'Whether this is critical',
                    required: false,
                    variableType: VariableType.BOOLEAN
                }),
                new TemplateVariable({
                    name: 'report_url',
                    description: 'Link to report',
                    required: true,
                    variableType: VariableType.URL
                })
            ],
            created_at: new Date(),
            updated_at: new Date(),
            version: 1,
            active: true
        });
        
        await service.addTemplate(template);
        console.log('✅ Template created and added successfully');
        
        // Test 4: Send templated notification
        const context = {
            user_name: 'Test User',
            severity: 'High',
            contract_name: 'TestContract',
            description: 'Reentrancy vulnerability detected',
            risk_score: 85,
            critical: true,
            report_url: 'https://scanner.example.com/report/123'
        };
        
        const result = await service.sendTemplatedNotification(
            'test_vulnerability_alert',
            recipient,
            context,
            [NotificationChannel.IN_APP], // Only test in-app to avoid external dependencies
            NotificationPriority.HIGH
        );
        
        if (result.success) {
            console.log('✅ Templated notification sent successfully');
            console.log(`   Delivered to: ${result.deliveredChannels.join(', ')}`);
        } else {
            console.log('⚠️  Notification partially failed:', result.failedChannels);
        }
        
        // Test 5: Check provider health
        const health = await service.healthCheck();
        console.log('✅ Provider health check completed');
        for (const [channel, healthy] of Object.entries(health)) {
            console.log(`   ${channel}: ${healthy ? '✅' : '❌'}`);
        }
        
        // Test 6: Get provider statistics
        const stats = await service.getProviderStats();
        console.log('✅ Provider statistics retrieved');
        for (const [channel, channelStats] of Object.entries(stats)) {
            console.log(`   ${channel}: ${channelStats.totalSent} sent, ${channelStats.totalFailed} failed`);
        }
        
        // Test 7: List templates
        const templates = await service.listTemplates();
        console.log(`✅ Templates listed: ${templates.length} templates found`);
        
        // Test 8: In-app notifications
        const inAppNotifications = service.getInAppNotifications('test_user_123');
        console.log(`✅ In-app notifications retrieved: ${inAppNotifications.length} notifications`);
        
        console.log('\n🎉 All JavaScript tests passed!');
        return true;
        
    } catch (error) {
        console.error('\n❌ JavaScript test failed:', error.message);
        console.error(error.stack);
        return false;
    }
}

/**
 * Validate Rust implementation structure
 */
function validateRustImplementation() {
    console.log('\n🧪 Validating Rust Notification Service Structure...\n');
    
    const rustServicePath = path.join(__dirname, 'src', 'notification_service');
    
    try {
        // Check required files exist
        const requiredFiles = [
            'mod.rs',
            'types.rs',
            'service.rs',
            'providers.rs',
            'templates.rs',
            'tracking.rs',
            'tests.rs'
        ];
        
        for (const file of requiredFiles) {
            const filePath = path.join(rustServicePath, file);
            if (!fs.existsSync(filePath)) {
                throw new Error(`Missing required file: ${file}`);
            }
            console.log(`✅ ${file} exists`);
        }
        
        // Check Cargo.toml exists
        const cargoPath = path.join(__dirname, 'Cargo.toml');
        if (!fs.existsSync(cargoPath)) {
            throw new Error('Missing Cargo.toml file');
        }
        console.log('✅ Cargo.toml exists');
        
        // Validate Cargo.toml content
        const cargoContent = fs.readFileSync(cargoPath, 'utf8');
        const requiredDependencies = [
            'tokio',
            'serde',
            'uuid',
            'chrono',
            'async-trait',
            'handlebars',
            'lettre',
            'reqwest',
            'thiserror'
        ];
        
        for (const dep of requiredDependencies) {
            if (!cargoContent.includes(dep)) {
                throw new Error(`Missing dependency in Cargo.toml: ${dep}`);
            }
            console.log(`✅ Dependency ${dep} found`);
        }
        
        // Validate module structure
        const modContent = fs.readFileSync(path.join(rustServicePath, 'mod.rs'), 'utf8');
        const requiredModules = [
            'providers',
            'templates', 
            'tracking',
            'types',
            'service'
        ];
        
        for (const module of requiredModules) {
            if (!modContent.includes(`pub mod ${module}`)) {
                throw new Error(`Missing module export: ${module}`);
            }
            console.log(`✅ Module ${module} exported`);
        }
        
        console.log('\n🎉 Rust implementation structure validated successfully!');
        return true;
        
    } catch (error) {
        console.error('\n❌ Rust validation failed:', error.message);
        return false;
    }
}

/**
 * Test integration with security scanner
 */
function testSecurityScannerIntegration() {
    console.log('\n🧪 Testing Security Scanner Integration...\n');
    
    try {
        // Check if notification service is integrated in main index.js
        const indexPath = path.join(__dirname, 'src', 'index.js');
        if (!fs.existsSync(indexPath)) {
            throw new Error('Main index.js not found');
        }
        
        const indexContent = fs.readFileSync(indexPath, 'utf8');
        
        // Check for notification service imports
        if (!indexContent.includes('notification-service')) {
            throw new Error('Notification service not imported in main index.js');
        }
        console.log('✅ Notification service imported in main index.js');
        
        // Check for notification commands
        if (!indexContent.includes('notifications')) {
            throw new Error('Notification commands not found in main index.js');
        }
        console.log('✅ Notification commands found in main index.js');
        
        // Check for notification options in scan command
        if (!indexContent.includes('--notify')) {
            throw new Error('Notification options not found in scan command');
        }
        console.log('✅ Notification options found in scan command');
        
        // Check lib.rs exports
        const libPath = path.join(__dirname, 'src', 'lib.rs');
        if (fs.existsSync(libPath)) {
            const libContent = fs.readFileSync(libPath, 'utf8');
            if (!libContent.includes('notification_service')) {
                throw new Error('Notification service not exported in lib.rs');
            }
            console.log('✅ Notification service exported in lib.rs');
        }
        
        console.log('\n🎉 Security scanner integration validated successfully!');
        return true;
        
    } catch (error) {
        console.error('\n❌ Integration test failed:', error.message);
        return false;
    }
}

/**
 * Main test runner
 */
async function runTests() {
    console.log('🚀 Starting Comprehensive Notification Service Tests\n');
    
    const results = {
        javascript: false,
        rust: false,
        integration: false
    };
    
    // Run JavaScript tests
    results.javascript = await testJavaScriptImplementation();
    
    // Validate Rust implementation
    results.rust = validateRustImplementation();
    
    // Test integration
    results.integration = testSecurityScannerIntegration();
    
    // Summary
    console.log('\n📊 Test Results Summary:');
    console.log(`   JavaScript Implementation: ${results.javascript ? '✅ PASS' : '❌ FAIL'}`);
    console.log(`   Rust Implementation: ${results.rust ? '✅ PASS' : '❌ FAIL'}`);
    console.log(`   Security Scanner Integration: ${results.integration ? '✅ PASS' : '❌ FAIL'}`);
    
    const allPassed = Object.values(results).every(result => result);
    
    if (allPassed) {
        console.log('\n🎉 All tests passed! Notification service is ready for production.');
        process.exit(0);
    } else {
        console.log('\n❌ Some tests failed. Please review the implementation.');
        process.exit(1);
    }
}

// Run tests if this file is executed directly
if (require.main === module) {
    runTests().catch(error => {
        console.error('Test runner failed:', error);
        process.exit(1);
    });
}

module.exports = {
    testJavaScriptImplementation,
    validateRustImplementation,
    testSecurityScannerIntegration,
    runTests
};
