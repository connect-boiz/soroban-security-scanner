//! Notification providers for different channels

use crate::notification_service::types::{
    NotificationMessage, Recipient, NotificationChannel, DeliveryStatus, 
    DeliveryTracking, ProviderConfig, NotificationResult
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// Trait for notification providers
#[async_trait]
pub trait NotificationProvider: Send + Sync {
    /// Send a notification
    async fn send_notification(
        &self,
        message: &NotificationMessage,
        recipient: &Recipient,
    ) -> Result<DeliveryTracking, ProviderError>;

    /// Get provider type
    fn channel(&self) -> NotificationChannel;

    /// Check if provider is healthy
    async fn health_check(&self) -> Result<bool, ProviderError>;

    /// Get provider statistics
    async fn get_stats(&self) -> ProviderStats;
}

/// Email notification provider
pub struct EmailProvider {
    config: ProviderConfig,
    smtp_client: Option<SmtpClient>,
}

impl EmailProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        let smtp_client = if config.enabled {
            Some(SmtpClient::new(&config)?)
        } else {
            None
        };

        Ok(Self { config, smtp_client })
    }
}

#[async_trait]
impl NotificationProvider for EmailProvider {
    async fn send_notification(
        &self,
        message: &NotificationMessage,
        recipient: &Recipient,
    ) -> Result<DeliveryTracking, ProviderError> {
        if !self.config.enabled {
            return Err(ProviderError::ProviderDisabled);
        }

        let email = recipient.email.as_ref()
            .ok_or_else(|| ProviderError::MissingRecipientData("email".to_string()))?;

        let smtp_client = self.smtp_client.as_ref()
            .ok_or_else(|| ProviderError::ProviderNotConfigured)?;

        let tracking_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Send email
        let result = smtp_client.send_email(
            email,
            message.subject.as_deref().unwrap_or("Notification"),
            &message.body,
        ).await;

        match result {
            Ok(external_id) => Ok(DeliveryTracking {
                notification_id: message.id.clone(),
                recipient_id: recipient.id.clone(),
                channel: NotificationChannel::Email,
                status: DeliveryStatus::Sent,
                attempts: 1,
                last_attempt: now,
                delivered_at: Some(now),
                error_message: None,
                external_id: Some(external_id),
            }),
            Err(e) => Ok(DeliveryTracking {
                notification_id: message.id.clone(),
                recipient_id: recipient.id.clone(),
                channel: NotificationChannel::Email,
                status: DeliveryStatus::Failed,
                attempts: 1,
                last_attempt: now,
                delivered_at: None,
                error_message: Some(e.to_string()),
                external_id: None,
            }),
        }
    }

    fn channel(&self) -> NotificationChannel {
        NotificationChannel::Email
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        if let Some(smtp_client) = &self.smtp_client {
            smtp_client.health_check().await
        } else {
            Ok(false)
        }
    }

    async fn get_stats(&self) -> ProviderStats {
        ProviderStats {
            channel: NotificationChannel::Email,
            total_sent: 0, // Would be tracked in real implementation
            total_failed: 0,
            average_delivery_time_ms: 0,
            last_success: None,
            last_failure: None,
        }
    }
}

/// SMS notification provider
pub struct SMSProvider {
    config: ProviderConfig,
    sms_client: Option<SmsClient>,
}

impl SMSProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        let sms_client = if config.enabled {
            Some(SmsClient::new(&config)?)
        } else {
            None
        };

        Ok(Self { config, sms_client })
    }
}

#[async_trait]
impl NotificationProvider for SMSProvider {
    async fn send_notification(
        &self,
        message: &NotificationMessage,
        recipient: &Recipient,
    ) -> Result<DeliveryTracking, ProviderError> {
        if !self.config.enabled {
            return Err(ProviderError::ProviderDisabled);
        }

        let phone = recipient.phone.as_ref()
            .ok_or_else(|| ProviderError::MissingRecipientData("phone".to_string()))?;

        let sms_client = self.sms_client.as_ref()
            .ok_or_else(|| ProviderError::ProviderNotConfigured)?;

        let tracking_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Truncate message for SMS (160 characters typical limit)
        let sms_body = if message.body.len() > 160 {
            format!("{}...", &message.body[..157])
        } else {
            message.body.clone()
        };

        let result = sms_client.send_sms(phone, &sms_body).await;

        match result {
            Ok(external_id) => Ok(DeliveryTracking {
                notification_id: message.id.clone(),
                recipient_id: recipient.id.clone(),
                channel: NotificationChannel::SMS,
                status: DeliveryStatus::Sent,
                attempts: 1,
                last_attempt: now,
                delivered_at: Some(now),
                error_message: None,
                external_id: Some(external_id),
            }),
            Err(e) => Ok(DeliveryTracking {
                notification_id: message.id.clone(),
                recipient_id: recipient.id.clone(),
                channel: NotificationChannel::SMS,
                status: DeliveryStatus::Failed,
                attempts: 1,
                last_attempt: now,
                delivered_at: None,
                error_message: Some(e.to_string()),
                external_id: None,
            }),
        }
    }

    fn channel(&self) -> NotificationChannel {
        NotificationChannel::SMS
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        if let Some(sms_client) = &self.sms_client {
            sms_client.health_check().await
        } else {
            Ok(false)
        }
    }

    async fn get_stats(&self) -> ProviderStats {
        ProviderStats {
            channel: NotificationChannel::SMS,
            total_sent: 0,
            total_failed: 0,
            average_delivery_time_ms: 0,
            last_success: None,
            last_failure: None,
        }
    }
}

/// Push notification provider
pub struct PushProvider {
    config: ProviderConfig,
    push_client: Option<PushClient>,
}

impl PushProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        let push_client = if config.enabled {
            Some(PushClient::new(&config)?)
        } else {
            None
        };

        Ok(Self { config, push_client })
    }
}

#[async_trait]
impl NotificationProvider for PushProvider {
    async fn send_notification(
        &self,
        message: &NotificationMessage,
        recipient: &Recipient,
    ) -> Result<DeliveryTracking, ProviderError> {
        if !self.config.enabled {
            return Err(ProviderError::ProviderDisabled);
        }

        let push_client = self.push_client.as_ref()
            .ok_or_else(|| ProviderError::ProviderNotConfigured)?;

        let tracking_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let mut results = Vec::new();
        
        // Send to all device tokens
        for device_token in &recipient.device_tokens {
            let result = push_client.send_push(
                device_token,
                message.subject.as_deref().unwrap_or("Notification"),
                &message.body,
                &message.data,
            ).await;

            match result {
                Ok(external_id) => results.push(Ok(external_id)),
                Err(e) => results.push(Err(e)),
            }
        }

        // Determine overall status based on results
        let status = if results.iter().any(|r| r.is_ok()) {
            DeliveryStatus::Sent
        } else {
            DeliveryStatus::Failed
        };

        let error_message = if results.iter().all(|r| r.is_err()) {
            Some(results.iter().filter_map(|r| r.as_ref().err())
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(", "))
        } else {
            None
        };

        let external_id = results.iter().find_map(|r| r.as_ref().ok().cloned());

        Ok(DeliveryTracking {
            notification_id: message.id.clone(),
            recipient_id: recipient.id.clone(),
            channel: NotificationChannel::Push,
            status,
            attempts: 1,
            last_attempt: now,
            delivered_at: if status == DeliveryStatus::Sent { Some(now) } else { None },
            error_message,
            external_id,
        })
    }

    fn channel(&self) -> NotificationChannel {
        NotificationChannel::Push
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        if let Some(push_client) = &self.push_client {
            push_client.health_check().await
        } else {
            Ok(false)
        }
    }

    async fn get_stats(&self) -> ProviderStats {
        ProviderStats {
            channel: NotificationChannel::Push,
            total_sent: 0,
            total_failed: 0,
            average_delivery_time_ms: 0,
            last_success: None,
            last_failure: None,
        }
    }
}

/// In-app notification provider
pub struct InAppProvider {
    config: ProviderConfig,
    storage: InAppStorage,
}

impl InAppProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        Ok(Self {
            config,
            storage: InAppStorage::new(),
        })
    }
}

#[async_trait]
impl NotificationProvider for InAppProvider {
    async fn send_notification(
        &self,
        message: &NotificationMessage,
        recipient: &Recipient,
    ) -> Result<DeliveryTracking, ProviderError> {
        if !self.config.enabled {
            return Err(ProviderError::ProviderDisabled);
        }

        let tracking_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Store in-app notification
        let notification = InAppNotification {
            id: tracking_id.clone(),
            user_id: recipient.user_id.clone().unwrap_or_else(|| recipient.id.clone()),
            title: message.subject.clone().unwrap_or_else(|| "Notification".to_string()),
            body: message.body.clone(),
            data: message.data.clone(),
            priority: message.priority.clone(),
            read: false,
            created_at: now,
            expires_at: message.scheduled_for,
        };

        let result = self.storage.store_notification(notification).await;

        match result {
            Ok(_) => Ok(DeliveryTracking {
                notification_id: message.id.clone(),
                recipient_id: recipient.id.clone(),
                channel: NotificationChannel::InApp,
                status: DeliveryStatus::Delivered,
                attempts: 1,
                last_attempt: now,
                delivered_at: Some(now),
                error_message: None,
                external_id: Some(tracking_id),
            }),
            Err(e) => Ok(DeliveryTracking {
                notification_id: message.id.clone(),
                recipient_id: recipient.id.clone(),
                channel: NotificationChannel::InApp,
                status: DeliveryStatus::Failed,
                attempts: 1,
                last_attempt: now,
                delivered_at: None,
                error_message: Some(e.to_string()),
                external_id: None,
            }),
        }
    }

    fn channel(&self) -> NotificationChannel {
        NotificationChannel::InApp
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        self.storage.health_check().await
    }

    async fn get_stats(&self) -> ProviderStats {
        ProviderStats {
            channel: NotificationChannel::InApp,
            total_sent: 0,
            total_failed: 0,
            average_delivery_time_ms: 0,
            last_success: None,
            last_failure: None,
        }
    }
}

/// Provider statistics
#[derive(Debug, Clone)]
pub struct ProviderStats {
    pub channel: NotificationChannel,
    pub total_sent: u64,
    pub total_failed: u64,
    pub average_delivery_time_ms: u64,
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,
    pub last_failure: Option<chrono::DateTime<chrono::Utc>>,
}

/// Provider errors
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Provider is disabled")]
    ProviderDisabled,
    
    #[error("Provider not configured")]
    ProviderNotConfigured,
    
    #[error("Missing recipient data: {0}")]
    MissingRecipientData(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Authentication error")]
    AuthenticationError,
    
    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),
}

// Mock implementations for client types (would be real implementations in production)
#[derive(Debug)]
struct SmtpClient;

impl SmtpClient {
    fn new(_config: &ProviderConfig) -> Result<Self, ProviderError> {
        // In real implementation, would configure SMTP client
        Ok(Self)
    }
    
    async fn send_email(&self, _to: &str, _subject: &str, _body: &str) -> Result<String, ProviderError> {
        // Mock implementation
        Ok("email_id_123".to_string())
    }
    
    async fn health_check(&self) -> Result<bool, ProviderError> {
        Ok(true)
    }
}

#[derive(Debug)]
struct SmsClient;

impl SmsClient {
    fn new(_config: &ProviderConfig) -> Result<Self, ProviderError> {
        // In real implementation, would configure SMS client (Twilio, etc.)
        Ok(Self)
    }
    
    async fn send_sms(&self, _to: &str, _body: &str) -> Result<String, ProviderError> {
        // Mock implementation
        Ok("sms_id_456".to_string())
    }
    
    async fn health_check(&self) -> Result<bool, ProviderError> {
        Ok(true)
    }
}

#[derive(Debug)]
struct PushClient;

impl PushClient {
    fn new(_config: &ProviderConfig) -> Result<Self, ProviderError> {
        // In real implementation, would configure push client (FCM, APNS, etc.)
        Ok(Self)
    }
    
    async fn send_push(
        &self, 
        _token: &str, 
        _title: &str, 
        _body: &str, 
        _data: &HashMap<String, String>
    ) -> Result<String, ProviderError> {
        // Mock implementation
        Ok("push_id_789".to_string())
    }
    
    async fn health_check(&self) -> Result<bool, ProviderError> {
        Ok(true)
    }
}

#[derive(Debug)]
struct InAppStorage;

impl InAppStorage {
    fn new() -> Self {
        Self
    }
    
    async fn store_notification(&self, _notification: InAppNotification) -> Result<(), ProviderError> {
        // Mock implementation - would store in database
        Ok(())
    }
    
    async fn health_check(&self) -> Result<bool, ProviderError> {
        Ok(true)
    }
}

#[derive(Debug)]
struct InAppNotification {
    id: String,
    user_id: String,
    title: String,
    body: String,
    data: HashMap<String, String>,
    priority: crate::notification_service::types::NotificationPriority,
    read: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}
