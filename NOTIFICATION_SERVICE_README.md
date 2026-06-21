# Notification Service for Soroban Security Scanner

## 🚀 Quick Start

The Soroban Security Scanner now includes a comprehensive notification service that supports multiple channels, template management, and delivery tracking.

### Installation

```bash
npm install
```

### Basic Usage

```bash
# Scan with notifications
node src/index.js scan /path/to/contract --notify --notify-email user@example.com

# Test notification service
node src/index.js notifications test --email user@example.com

# List available templates
node src/index.js notifications templates
```

## 📋 Features

### ✅ Multiple Notification Channels
- **Email**: SMTP-based email delivery with HTML support
- **SMS**: Ready for Twilio integration (mock implementation included)
- **Push**: Ready for FCM/APNS integration (mock implementation included)
- **In-App**: Real-time in-app notifications with read/unread tracking

### ✅ Template Management
- Handlebars-based dynamic templates
- Custom helpers for formatting (dates, currency, truncation)
- Variable validation and type checking
- Built-in templates for security scanner use cases

### ✅ Delivery Tracking
- Real-time delivery status tracking
- Comprehensive metrics and statistics
- Failed delivery retry logic
- Performance monitoring

### ✅ Advanced Features
- Rate limiting to prevent spam
- Quiet hours respecting user preferences
- Priority-based delivery (Low, Normal, High, Critical)
- Health monitoring for all providers
- User preference management

## 🔧 Configuration

### Email Provider

```javascript
await service.configureProvider(NotificationChannel.EMAIL, {
  providerType: NotificationChannel.EMAIL,
  enabled: true,
  config: {
    smtp_host: 'smtp.gmail.com',
    smtp_port: '587',
    username: 'your-email@example.com',
    password: 'your-app-password',
    from_email: 'scanner@example.com',
    from_name: 'Soroban Security Scanner'
  }
});
```

### Environment Variables

```bash
# Email Configuration
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=scanner@example.com
SMTP_PASSWORD=app_password
SMTP_FROM_EMAIL=scanner@example.com
SMTP_FROM_NAME=Soroban Security Scanner
```

## 📝 Templates

### Default Templates

#### Vulnerability Alert
Used when individual vulnerabilities are detected:

```handlebars
🚨 {{severity}} Vulnerability Found in {{contract_name}}

Hello {{user_name}},

A {{severity}} vulnerability has been detected in your smart contract:

Contract: {{contract_name}}
Type: {{vulnerability_type}}
Description: {{description}}
Risk Score: {{risk_score}}

{{#if critical}}
⚠️ This is a critical vulnerability that requires immediate attention!
{{/if}}

Please review the full scan report at: {{report_url}}
```

#### Scan Completed
Used when security scans finish:

```handlebars
✅ Security Scan Completed for {{file_path}}

Hello {{user_name}},

Your security scan has completed:

File: {{file_path}}
Total Issues: {{total_issues}}
Critical: {{critical_count}}
High: {{high_count}}
Medium: {{medium_count}}

{{#if has_issues}}
⚠️ Issues were found that require your attention.
{{else}}
✅ No security issues were found. Your contract looks secure!
{{/if}}
```

### Custom Templates

```javascript
const customTemplate = new NotificationTemplate({
  id: 'custom_alert',
  name: 'Custom Security Alert',
  subjectTemplate: '🔒 Security Alert: {{alert_type}}',
  bodyTemplate: 'Your custom template content...',
  supportedChannels: [NotificationChannel.EMAIL, NotificationChannel.IN_APP],
  variables: [
    new TemplateVariable({
      name: 'alert_type',
      required: true,
      variableType: VariableType.STRING
    })
  ]
});

await service.addTemplate(customTemplate);
```

## 🎯 Usage Examples

### Programmatic Usage

```javascript
const { NotificationService, NotificationChannel, NotificationPriority, Recipient } = require('./notification-service');

const service = new NotificationService();

// Create recipient
const recipient = new Recipient({
  id: 'user_123',
  email: 'user@example.com',
  userId: 'user_123',
  preferences: {
    emailEnabled: true,
    inAppEnabled: true,
    quietHours: {
      startHour: 22,
      endHour: 8,
      timezone: 'UTC'
    }
  }
});

// Send notification
const result = await service.sendTemplatedNotification(
  'vulnerability_alert',
  recipient,
  {
    user_name: 'Alice',
    severity: 'High',
    contract_name: 'TokenContract',
    description: 'Reentrancy vulnerability detected',
    risk_score: 85,
    critical: true,
    report_url: 'https://scanner.example.com/report/123'
  },
  [NotificationChannel.EMAIL, NotificationChannel.IN_APP],
  NotificationPriority.HIGH
);

console.log('Notification sent:', result.success);
```

### Command Line Interface

```bash
# Scan with notifications
node src/index.js scan contracts/token.wasm --notify --notify-email dev@company.com

# Test notification service
node src/index.js notifications test --email test@example.com

# List templates
node src/index.js notifications templates

# View in-app notifications
node src/index.js notifications in-app list user_123
```

## 📊 Monitoring

### Health Checks

```javascript
// Check provider health
const health = await service.healthCheck();
console.log('Provider Health:', health);
// Output: { email: true, sms: false, push: false, in_app: true }

// Get provider statistics
const stats = await service.getProviderStats();
console.log('Provider Stats:', stats);
```

### Delivery Tracking

```javascript
// Get delivery statistics
const now = new Date();
const oneHourAgo = new Date(now.getTime() - 3600000);
const deliveryStats = await service.getDeliveryStats(oneHourAgo, now);

console.log('Total notifications:', deliveryStats.totalNotifications);
console.log('Channel breakdown:', deliveryStats.channelStats);
```

## 🧪 Testing

```bash
# Run notification service tests
node src/notification-service/test.js

# Test specific functionality
node src/notification-service/test.js --filter=template_management
```

## 🔒 Security Considerations

### Credential Management
- Store provider credentials in environment variables
- Use secret management services in production
- Rotate credentials regularly

### Rate Limiting
- Built-in rate limiting per provider
- Configurable limits (per second, minute, hour)
- Automatic backoff for failed deliveries

### Data Privacy
- Template input sanitization
- User preference respect
- Audit logging capabilities

## 🚀 Production Deployment

### Requirements
- Node.js 18+ 
- Redis or similar for job queue (recommended)
- Database for persistent storage (recommended)
- SMTP server for email delivery

### Recommended Setup
1. Use environment variables for all configuration
2. Set up proper secret management
3. Configure monitoring and alerting
4. Implement database persistence
5. Set up message queue for reliability

## 📚 API Reference

### NotificationService

#### Methods
- `sendNotification(message, recipient)` - Send direct notification
- `sendTemplatedNotification(templateId, recipient, context, channels, priority)` - Send templated notification
- `scheduleNotification(message, recipient, scheduledFor)` - Schedule future delivery
- `addTemplate(template)` - Add new template
- `getDeliveryStats(startTime, endTime)` - Get delivery statistics
- `healthCheck()` - Check provider health

### Classes
- `NotificationService` - Main service class
- `Recipient` - User information and preferences
- `NotificationMessage` - Message structure
- `NotificationTemplate` - Template definition
- `DeliveryTracking` - Delivery tracking information

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## 📄 License

MIT License - see LICENSE file for details.

## 🆘 Support

- 📖 [Full Documentation](./NOTIFICATION_SERVICE_IMPLEMENTATION.md)
- 🐛 [Report Issues](https://github.com/gbengaeben/soroban-security-scanner/issues)
- 💬 [Discussions](https://github.com/gbengaeben/soroban-security-scanner/discussions)

---

**Built with ❤️ for the Soroban ecosystem**
