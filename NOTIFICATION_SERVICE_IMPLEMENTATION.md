# Notification Service Implementation

## Overview

This document describes the comprehensive notification service implementation for the Soroban Security Scanner. The service supports multiple notification channels, template management, and delivery tracking.

## Features Implemented

### ✅ Core Features
- **Multiple Channels**: Email, SMS, Push Notifications, and In-App Alerts
- **Template Management**: Dynamic template rendering with Handlebars
- **Delivery Tracking**: Real-time tracking of notification delivery status
- **Rate Limiting**: Built-in rate limiting to prevent spam
- **Quiet Hours**: Respect user preferences for notification timing
- **Priority Levels**: Support for different notification priorities (Low, Normal, High, Critical)
- **Health Monitoring**: Provider health checks and statistics
- **User Preferences**: Customizable notification settings per recipient

### ✅ Architecture Components

#### 1. Types and Data Structures (`src/notification-service/types.js`)
- `NotificationChannel`: Enum for supported channels
- `NotificationPriority`: Priority levels (Low, Normal, High, Critical)
- `DeliveryStatus`: Tracking status (Pending, Processing, Sent, Delivered, Failed, Retrying)
- `Recipient`: User information and preferences
- `NotificationMessage`: Message structure with metadata
- `DeliveryTracking`: Delivery tracking information
- `NotificationTemplate`: Template definition with variables
- `ProviderConfig`: Configuration for notification providers

#### 2. Template Manager (`src/notification-service/template-manager.js`)
- Handlebars-based template rendering
- Custom helpers for date formatting, currency, truncation
- Template validation and variable checking
- Default templates for security scanner use cases

#### 3. Delivery Tracker (`src/notification-service/delivery-tracker.js`)
- In-memory storage for tracking records
- Metrics collection and statistics
- Failed delivery retry logic
- Cleanup of old records

#### 4. Notification Providers (`src/notification-service/providers.js`)
- **EmailProvider**: SMTP-based email delivery (using nodemailer)
- **SMSProvider**: Mock SMS provider (ready for Twilio integration)
- **PushProvider**: Mock push provider (ready for FCM/APNS integration)
- **InAppProvider**: In-memory in-app notification storage

#### 5. Main Service (`src/notification-service/notification-service.js`)
- Service orchestration and coordination
- Rate limiting implementation
- Quiet hours and preference handling
- Multi-channel delivery management

## Integration with Security Scanner

### Command Line Integration

The notification service is integrated into the main CLI with the following new commands:

```bash
# Scan with notifications enabled
node src/index.js scan /path/to/contract --notify --notify-email user@example.com

# Test notification service
node src/index.js notifications test --email user@example.com

# List available templates
node src/index.js notifications templates

# List in-app notifications for user
node src/index.js notifications in-app list user_123
```

### Security Scanner Integration

The notification service automatically sends notifications when:

1. **Vulnerabilities Found**: After security scans detect issues
2. **Scan Completion**: When scans finish (with or without issues)
3. **Critical Alerts**: For high-priority vulnerabilities requiring immediate attention

### Default Templates

Two default templates are provided:

1. **Vulnerability Alert**: For individual vulnerability notifications
2. **Scan Completed**: For scan completion summaries

## Usage Examples

### Basic Notification Sending

```javascript
const { NotificationService, NotificationChannel, NotificationPriority, Recipient } = require('./notification-service');

const service = new NotificationService();

const recipient = new Recipient({
  id: 'user_123',
  email: 'user@example.com',
  userId: 'user_123',
  preferences: {
    emailEnabled: true,
    inAppEnabled: true,
    smsEnabled: false,
    pushEnabled: false
  }
});

// Send templated notification
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
```

### Email Provider Configuration

```javascript
await service.configureProvider(NotificationChannel.EMAIL, {
  providerType: NotificationChannel.EMAIL,
  enabled: true,
  config: {
    smtp_host: 'smtp.gmail.com',
    smtp_port: '587',
    username: 'scanner@example.com',
    password: 'app_password',
    from_email: 'scanner@example.com',
    from_name: 'Soroban Security Scanner'
  },
  rateLimit: {
    maxRequestsPerSecond: 10,
    maxRequestsPerMinute: 100,
    maxRequestsPerHour: 1000
  }
});
```

### Custom Template Creation

```javascript
const customTemplate = new NotificationTemplate({
  id: 'custom_alert',
  name: 'Custom Security Alert',
  subjectTemplate: '🔒 Security Alert: {{alert_type}}',
  bodyTemplate: `Hello {{user_name}},

{{alert_type}} detected in {{contract_name}}:

{{description}}

Severity: {{severity}}
Risk Score: {{risk_score}}

Action required: {{action_needed}}

View details: {{report_url}}`,
  supportedChannels: [NotificationChannel.EMAIL, NotificationChannel.IN_APP],
  defaultPriority: NotificationPriority.NORMAL,
  variables: [
    new TemplateVariable({
      name: 'user_name',
      required: true,
      variableType: VariableType.STRING
    }),
    // ... more variables
  ]
});

await service.addTemplate(customTemplate);
```

## Testing

### Running Tests

```bash
# Run the notification service test suite
node src/notification-service/test.js
```

### Test Coverage

The test suite covers:
- Template management and rendering
- Multi-channel notification delivery
- Priority and quiet hours handling
- Delivery tracking
- Provider health checks
- Error scenarios
- Security scanner integration

## Production Deployment Considerations

### Dependencies

Add these dependencies to `package.json`:

```json
{
  "dependencies": {
    "handlebars": "^4.7.8",
    "nodemailer": "^6.9.7",
    "uuid": "^9.0.1"
  }
}
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

# SMS Configuration (Twilio)
TWILIO_ACCOUNT_SID=your_account_sid
TWILIO_AUTH_TOKEN=your_auth_token
TWILIO_FROM_NUMBER=+1234567890

# Push Configuration
FCM_SERVER_KEY=your_fcm_server_key
APNS_KEY_ID=your_apns_key_id
APNS_TEAM_ID=your_team_id
```

### Database Integration

For production use, replace the in-memory storage with a database:

```javascript
// Example: Replace TrackingStorage with database implementation
class DatabaseTrackingStorage {
  async storeTracking(tracking) {
    await db.collection('delivery_tracking').insertOne(tracking);
  }
  
  async getTracking(notificationId, recipientId, channel) {
    return await db.collection('delivery_tracking').findOne({
      notificationId,
      recipientId,
      channel
    });
  }
}
```

### Message Queue Integration

For reliable delivery, integrate with a message queue:

```javascript
// Example: Add queue for scheduled notifications
await service.scheduleNotification(message, recipient, scheduledFor);

// Queue worker would process scheduled notifications
queue.process('scheduled-notifications', async (job) => {
  const { message, recipient } = job.data;
  await service.sendNotification(message, recipient);
});
```

## Security Considerations

### Credential Management
- Store provider credentials in environment variables
- Use secret management services in production
- Rotate credentials regularly

### Data Privacy
- Sanitize template inputs to prevent XSS
- Validate recipient data
- Implement rate limiting to prevent abuse

### Compliance
- Respect user preferences and unsubscribe requests
- Implement audit logging for notification delivery
- Consider GDPR/CCPA compliance for user data

## Monitoring and Observability

### Metrics to Track
- Delivery rates by channel
- Average delivery times
- Error rates and types
- Template usage statistics
- User engagement metrics

### Health Checks
- Provider connectivity tests
- Template validation checks
- Storage system health
- Rate limiter status

### Logging
- Structured logging for all notification events
- Error tracking and alerting
- Performance metrics
- User activity logs

## Future Enhancements

### Planned Features
- [ ] Webhook support for custom integrations
- [ ] Notification batching and digesting
- [ ] A/B testing for notification content
- [ ] Machine learning for optimal send times
- [ ] Multi-language template support
- [ ] Rich media notifications (images, attachments)
- [ ] Interactive notifications (buttons, actions)

### Provider Expansions
- [ ] Slack integration
- [ ] Discord notifications
- [ ] Microsoft Teams
- [ ] Webex notifications
- [ ] Custom webhook providers

## Troubleshooting

### Common Issues

1. **Email Not Sending**
   - Check SMTP configuration
   - Verify credentials
   - Check network connectivity

2. **Template Rendering Errors**
   - Validate template syntax
   - Check required variables
   - Review Handlebars helpers

3. **Rate Limiting Issues**
   - Adjust rate limit settings
   - Check provider limits
   - Implement backoff logic

### Debug Mode

Enable debug logging:

```javascript
process.env.DEBUG = 'notification-service:*';
```

## Conclusion

The notification service provides a comprehensive, production-ready solution for managing notifications in the Soroban Security Scanner. It supports multiple channels, advanced template management, and robust delivery tracking while maintaining flexibility for future enhancements.

The implementation follows best practices for:
- Code organization and modularity
- Error handling and resilience
- Security and privacy
- Performance and scalability
- Testing and documentation
