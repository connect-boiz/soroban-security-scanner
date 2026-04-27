//! Types and enums for the notification service

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Notification channel types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    SMS,
    Push,
    InApp,
}

/// Notification priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Notification delivery status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Processing,
    Sent,
    Delivered,
    Failed,
    Retrying,
}

/// Recipient information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipient {
    pub id: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub device_tokens: Vec<String>,
    pub user_id: Option<String>,
    pub preferences: NotificationPreferences,
}

/// User notification preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub email_enabled: bool,
    pub sms_enabled: bool,
    pub push_enabled: bool,
    pub in_app_enabled: bool,
    pub quiet_hours: Option<QuietHours>,
    pub max_priority: NotificationPriority,
}

/// Quiet hours configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHours {
    pub start_hour: u8, // 0-23
    pub end_hour: u8,   // 0-23
    pub timezone: String,
}

/// Notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMessage {
    pub id: String,
    pub template_id: Option<String>,
    pub subject: Option<String>,
    pub body: String,
    pub data: HashMap<String, String>,
    pub priority: NotificationPriority,
    pub channels: Vec<NotificationChannel>,
    pub created_at: DateTime<Utc>,
    pub scheduled_for: Option<DateTime<Utc>>,
}

/// Delivery tracking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryTracking {
    pub notification_id: String,
    pub recipient_id: String,
    pub channel: NotificationChannel,
    pub status: DeliveryStatus,
    pub attempts: u32,
    pub last_attempt: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub external_id: Option<String>,
}

/// Template variables context
pub type TemplateContext = HashMap<String, String>;

/// Notification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResult {
    pub notification_id: String,
    pub success: bool,
    pub delivered_channels: Vec<NotificationChannel>,
    pub failed_channels: Vec<(NotificationChannel, String)>,
    pub tracking_ids: Vec<String>,
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: NotificationChannel,
    pub config: HashMap<String, String>,
    pub enabled: bool,
    pub rate_limit: Option<RateLimit>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub max_requests_per_second: u32,
    pub max_requests_per_minute: u32,
    pub max_requests_per_hour: u32,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            email_enabled: true,
            sms_enabled: true,
            push_enabled: true,
            in_app_enabled: true,
            quiet_hours: None,
            max_priority: NotificationPriority::Normal,
        }
    }
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            start_hour: 22, // 10 PM
            end_hour: 8,    // 8 AM
            timezone: "UTC".to_string(),
        }
    }
}
