# Notification Service Documentation

## Overview

The Notification Service is a comprehensive notification system for the Soroban Security Scanner that supports multiple notification channels, template management, and delivery tracking.

## Features

- **Multiple Channels**: Email, SMS, Push Notifications, and In-App Alerts
- **Template Management**: Dynamic template rendering with Handlebars
- **Delivery Tracking**: Real-time tracking of notification delivery status
- **Rate Limiting**: Built-in rate limiting to prevent spam
- **Quiet Hours**: Respect user preferences for notification timing
- **Priority Levels**: Support for different notification priorities
- **Health Monitoring**: Provider health checks and statistics

## Quick Start

```rust
use soroban_security_scanner::notification_service::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create notification service
    let service = NotificationService::new()?;
    
    // Create a recipient
    let recipient = Recipient {
        id: "user_123".to_string(),
        email: Some("user@example.com".to_string()),
        phone: Some("+1234567890".to_string()),
        device_tokens: vec!["device_token_abc".to_string()],
        user_id: Some("user_123".to_string()),
        preferences: NotificationPreferences::default(),
    };
    
    // Send a simple notification
    let message = NotificationMessage {
        id: "notif_001".to_string(),
        template_id: None,
        subject: Some("Security Alert".to_string()),
        body: "A new vulnerability was detected in your smart contract.".to_string(),
        data: HashMap::new(),
        priority: NotificationPriority::High,
        channels: vec![NotificationChannel::Email, NotificationChannel::InApp],
        created_at: chrono::Utc::now(),
        scheduled_for: None,
    };
    
    let result = service.send_notification(message, recipient).await?;
    println!("Notification sent: {}", result.success);
    
    Ok(())
}
```

## Template Management

### Creating Templates

```rust
use soroban_security_scanner::notification_service::*;

let template = NotificationTemplate {
    id: "vulnerability_alert".to_string(),
    name: "Vulnerability Alert".to_string(),
    description: Some("Template for vulnerability notifications".to_string()),
    subject_template: Some("🚨 {{severity}} Vulnerability Found in {{contract_name}}".to_string()),
    body_template: r#"
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

Best regards,
Soroban Security Scanner
"#.to_string(),
    supported_channels: vec![NotificationChannel::Email, NotificationChannel::InApp],
    default_priority: NotificationPriority::High,
    variables: vec![
        TemplateVariable {
            name: "user_name".to_string(),
            description: Some("Recipient name".to_string()),
            required: true,
            default_value: None,
            variable_type: VariableType::String,
        },
        TemplateVariable {
            name: "severity".to_string(),
            description: Some("Vulnerability severity".to_string()),
            required: true,
            default_value: None,
            variable_type: VariableType::String,
        },
        TemplateVariable {
            name: "contract_name".to_string(),
            description: Some("Name of the contract".to_string()),
            required: true,
            default_value: None,
            variable_type: VariableType::String,
        },
        TemplateVariable {
            name: "vulnerability_type".to_string(),
            description: Some("Type of vulnerability".to_string()),
            required: true,
            default_value: None,
            variable_type: VariableType::String,
        },
        TemplateVariable {
            name: "description".to_string(),
            description: Some("Vulnerability description".to_string()),
            required: true,
            default_value: None,
            variable_type: VariableType::String,
        },
        TemplateVariable {
            name: "risk_score".to_string(),
            description: Some("Risk score (0-100)".to_string()),
            required: true,
            default_value: None,
            variable_type: VariableType::Number,
        },
        TemplateVariable {
            name: "critical".to_string(),
            description: Some("Whether this is critical".to_string()),
            required: false,
            default_value: Some("false".to_string()),
            variable_type: VariableType::Boolean,
        },
        TemplateVariable {
            name: "report_url".to_string(),
            description: Some("Link to full report".to_string()),
            required: true,
            default_value: None,
            variable_type: VariableType::Url,
        },
    ],
    created_at: chrono::Utc::now(),
    updated_at: chrono::Utc::now(),
    version: 1,
    active: true,
};

service.add_template(template).await?;
```

### Using Templates

```rust
let mut context = HashMap::new();
context.insert("user_name".to_string(), "Alice".to_string());
context.insert("severity".to_string(), "Critical".to_string());
context.insert("contract_name".to_string(), "TokenContract".to_string());
context.insert("vulnerability_type".to_string(), "Reentrancy".to_string());
context.insert("description".to_string(), "External call vulnerability found".to_string());
context.insert("risk_score".to_string(), "95".to_string());
context.insert("critical".to_string(), "true".to_string());
context.insert("report_url".to_string(), "https://scanner.example.com/report/123".to_string());

let result = service.send_templated_notification(
    "vulnerability_alert",
    recipient,
    context,
    vec![NotificationChannel::Email, NotificationChannel::InApp],
    NotificationPriority::High,
).await?;
```

## Provider Configuration

### Email Provider

```rust
let mut email_config = HashMap::new();
email_config.insert("smtp_host".to_string(), "smtp.gmail.com".to_string());
email_config.insert("smtp_port".to_string(), "587".to_string());
email_config.insert("username".to_string(), "scanner@example.com".to_string());
email_config.insert("password".to_string(), "app_password".to_string());
email_config.insert("from_email".to_string(), "scanner@example.com".to_string());
email_config.insert("from_name".to_string(), "Soroban Security Scanner".to_string());

let provider_config = ProviderConfig {
    provider_type: NotificationChannel::Email,
    config: email_config,
    enabled: true,
    rate_limit: Some(RateLimit {
        max_requests_per_second: 10,
        max_requests_per_minute: 100,
        max_requests_per_hour: 1000,
    }),
};

service.configure_provider(NotificationChannel::Email, provider_config).await?;
```

### SMS Provider

```rust
let mut sms_config = HashMap::new();
sms_config.insert("provider".to_string(), "twilio".to_string());
sms_config.insert("account_sid".to_string(), "your_account_sid".to_string());
sms_config.insert("auth_token".to_string(), "your_auth_token".to_string());
sms_config.insert("from_number".to_string(), "+1234567890".to_string());

let provider_config = ProviderConfig {
    provider_type: NotificationChannel::SMS,
    config: sms_config,
    enabled: true,
    rate_limit: Some(RateLimit {
        max_requests_per_second: 1,
        max_requests_per_minute: 10,
        max_requests_per_hour: 100,
    }),
};

service.configure_provider(NotificationChannel::SMS, provider_config).await?;
```

### Push Notification Provider

```rust
let mut push_config = HashMap::new();
push_config.insert("fcm_server_key".to_string(), "your_fcm_key".to_string());
push_config.insert("apns_key_id".to_string(), "your_apns_key_id".to_string());
push_config.insert("apns_team_id".to_string(), "your_team_id".to_string());

let provider_config = ProviderConfig {
    provider_type: NotificationChannel::Push,
    config: push_config,
    enabled: true,
    rate_limit: Some(RateLimit {
        max_requests_per_second: 50,
        max_requests_per_minute: 500,
        max_requests_per_hour: 5000,
    }),
};

service.configure_provider(NotificationChannel::Push, provider_config).await?;
```

## Delivery Tracking

### Check Notification Status

```rust
let tracking = service.get_delivery_tracking(
    "notif_001",
    "user_123",
    NotificationChannel::Email,
).await?;

if let Some(tracking_info) = tracking {
    println!("Status: {:?}", tracking_info.status);
    println!("Attempts: {}", tracking_info.attempts);
    println!("Delivered at: {:?}", tracking_info.delivered_at);
}
```

### Get Delivery Statistics

```rust
let start_time = chrono::Utc::now() - chrono::Duration::hours(24);
let end_time = chrono::Utc::now();

let stats = service.get_delivery_stats(start_time, end_time).await?;
println!("Total notifications: {}", stats.total_notifications);

for (channel, channel_stats) in stats.channel_stats {
    println!("{}: {} sent, {} delivered, {:.2}% success rate", 
        format!("{:?}", channel),
        channel_stats.total_sent,
        channel_stats.total_delivered,
        channel_stats.success_rate * 100.0
    );
}
```

## User Preferences

### Quiet Hours Configuration

```rust
let recipient = Recipient {
    id: "user_456".to_string(),
    email: Some("user@example.com".to_string()),
    phone: Some("+1234567890".to_string()),
    device_tokens: vec!["device_token_xyz".to_string()],
    user_id: Some("user_456".to_string()),
    preferences: NotificationPreferences {
        email_enabled: true,
        sms_enabled: false,  // Disable SMS
        push_enabled: true,
        in_app_enabled: true,
        quiet_hours: Some(QuietHours {
            start_hour: 22,  // 10 PM
            end_hour: 8,     // 8 AM
            timezone: "America/New_York".to_string(),
        }),
        max_priority: NotificationPriority::High,  // Only High+ during quiet hours
    },
};
```

## Health Monitoring

```rust
// Check provider health
let health_status = service.health_check().await;
for (channel, healthy) in health_status {
    println!("{:?}: {}", channel, if healthy { "Healthy" } else { "Unhealthy" });
}

// Get provider statistics
let stats = service.get_provider_stats().await;
for (channel, provider_stats) in stats {
    println!("{:?}: {} sent, {} failed, avg {}ms delivery time",
        channel,
        provider_stats.total_sent,
        provider_stats.total_failed,
        provider_stats.average_delivery_time_ms
    );
}
```

## Integration with Security Scanner

### Example Integration

```rust
use soroban_security_scanner::*;

pub async fn send_vulnerability_notifications(
    service: &NotificationService,
    scan_result: &ScanResult,
    recipients: Vec<Recipient>,
) -> Result<(), Box<dyn std::error::Error>> {
    
    for recipient in recipients {
        if scan_result.has_issues() {
            let (critical, high, medium) = scan_result.severity_count();
            
            let mut context = HashMap::new();
            context.insert("user_name".to_string(), "User".to_string());
            context.insert("file_path".to_string(), scan_result.file_path.clone());
            context.insert("critical_count".to_string(), critical.to_string());
            context.insert("high_count".to_string(), high.to_string());
            context.insert("medium_count".to_string(), medium.to_string());
            context.insert("total_issues".to_string(), 
                (critical + high + medium).to_string());

            let priority = if critical > 0 {
                NotificationPriority::Critical
            } else if high > 0 {
                NotificationPriority::High
            } else {
                NotificationPriority::Normal
            };

            let result = service.send_templated_notification(
                "scan_results",
                recipient,
                context,
                vec![NotificationChannel::Email, NotificationChannel::InApp],
                priority,
            ).await?;

            if !result.success {
                eprintln!("Failed to send notification to {:?}", result.failed_channels);
            }
        }
    }
    
    Ok(())
}
```

## Best Practices

1. **Template Organization**: Keep templates organized by purpose and use descriptive names
2. **Error Handling**: Always handle notification failures gracefully
3. **Rate Limiting**: Configure appropriate rate limits for each provider
4. **User Preferences**: Respect user notification preferences and quiet hours
5. **Monitoring**: Regularly check provider health and delivery statistics
6. **Security**: Store provider credentials securely and rotate them regularly
7. **Testing**: Test templates thoroughly before using them in production

## Error Handling

The notification service uses structured error handling:

```rust
match service.send_notification(message, recipient).await {
    Ok(result) => {
        if result.success {
            println!("Notification sent successfully");
        } else {
            eprintln!("Partial failure: {:?}", result.failed_channels);
        }
    }
    Err(e) => {
        eprintln!("Notification failed: {}", e);
        // Handle different error types appropriately
        match e {
            ServiceError::RateLimitExceeded => {
                // Implement retry logic with exponential backoff
            }
            ServiceError::ProviderNotFound(channel) => {
                // Configure the missing provider
            }
            ServiceError::TemplateError(te) => {
                // Fix template issues
            }
            _ => {
                // Log and handle other errors
            }
        }
    }
}
```

## Testing

Run the notification service tests:

```bash
cargo test notification_service
```

The test suite covers:
- Template management and rendering
- Multi-channel notification delivery
- Priority and quiet hours handling
- Delivery tracking
- Provider health checks
- Error scenarios
