//! Notification providers for different channels

use crate::notification_service::types::{
    NotificationMessage, Recipient, NotificationChannel, DeliveryStatus, 
    DeliveryTracking, ProviderConfig, NotificationResult, ProviderStats, ProviderError
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;
use lettre::{Message, SmtpTransport, Transport};
use reqwest::Client;

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

/// SMTP client for email delivery
pub struct SmtpClient {
    transport: SmtpTransport,
    from_email: String,
    from_name: String,
}

impl SmtpClient {
    pub fn new(config: &ProviderConfig) -> Result<Self, ProviderError> {
        let smtp_host = config.config.get("smtp_host")
            .ok_or_else(|| ProviderError::InvalidConfiguration("smtp_host missing".to_string()))?;
        let smtp_port = config.config.get("smtp_port")
            .and_then(|p| p.parse().ok())
            .unwrap_or(587);
        let username = config.config.get("username")
            .ok_or_else(|| ProviderError::InvalidConfiguration("username missing".to_string()))?;
        let password = config.config.get("password")
            .ok_or_else(|| ProviderError::InvalidConfiguration("password missing".to_string()))?;
        let from_email = config.config.get("from_email")
            .ok_or_else(|| ProviderError::InvalidConfiguration("from_email missing".to_string()))?;
        let from_name = config.config.get("from_name")
            .unwrap_or("Soroban Security Scanner");

        let creds = lettre::transport::smtp::authentication::Credentials::new(username.clone(), password.clone());
        
        let transport = SmtpTransport::relay(smtp_host)
            .map_err(|e| ProviderError::InvalidConfiguration(format!("SMTP configuration error: {}", e)))?
            .port(smtp_port)
            .credentials(creds)
            .build();

        Ok(Self {
            transport,
            from_email: from_email.clone(),
            from_name: from_name.to_string(),
        })
    }

    pub async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<String, ProviderError> {
        let email = Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from_email).parse().unwrap())
            .to(to.parse().unwrap())
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| ProviderError::InvalidConfiguration(format!("Email format error: {}", e)))?;

        self.transport.send(&email)
            .map(|_| Uuid::new_v4().to_string())
            .map_err(|e| ProviderError::NetworkError(format!("SMTP send error: {}", e)))
    }

    pub async fn health_check(&self) -> Result<bool, ProviderError> {
        // Simple health check - try to connect to SMTP server
        match self.transport.test_connection() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// SMS client for SMS delivery
pub struct SmsClient {
    client: Client,
    account_sid: String,
    auth_token: String,
    from_number: String,
}

impl SmsClient {
    pub fn new(config: &ProviderConfig) -> Result<Self, ProviderError> {
        let account_sid = config.config.get("account_sid")
            .ok_or_else(|| ProviderError::InvalidConfiguration("account_sid missing".to_string()))?;
        let auth_token = config.config.get("auth_token")
            .ok_or_else(|| ProviderError::InvalidConfiguration("auth_token missing".to_string()))?;
        let from_number = config.config.get("from_number")
            .ok_or_else(|| ProviderError::InvalidConfiguration("from_number missing".to_string()))?;

        Ok(Self {
            client: Client::new(),
            account_sid: account_sid.clone(),
            auth_token: auth_token.clone(),
            from_number: from_number.to_string(),
        })
    }

    pub async fn send_sms(&self, to: &str, body: &str) -> Result<String, ProviderError> {
        let url = format!("https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json", self.account_sid);
        
        let form_data = HashMap::from([
            ("To", to),
            ("From", &self.from_number),
            ("Body", body),
        ]);

        let response = self.client
            .post(&url)
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .form(&form_data)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(format!("SMS send error: {}", e)))?;

        if response.status().is_success() {
            Ok(Uuid::new_v4().to_string())
        } else {
            Err(ProviderError::NetworkError(format!("SMS API error: {}", response.status())))
        }
    }
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


/// Push notification client
pub struct PushClient {
    fcm_key: Option<String>,
    apns_key: Option<String>,
    client: Client,
}

impl PushClient {
    pub fn new(config: &ProviderConfig) -> Result<Self, ProviderError> {
        let fcm_key = config.config.get("fcm_server_key").cloned();
        let apns_key = config.config.get("apns_key_id").cloned();

        Ok(Self {
            fcm_key,
            apns_key,
            client: Client::new(),
        })
    }

    pub async fn send_push(
        &self,
        token: &str,
        title: &str,
        body: &str,
        data: &HashMap<String, String>,
    ) -> Result<String, ProviderError> {
        // For now, mock implementation - would integrate with FCM/APNS in production
        Ok(Uuid::new_v4().to_string())
    }

    pub async fn health_check(&self) -> Result<bool, ProviderError> {
        Ok(true)
    }
}

/// In-app notification storage
pub struct InAppStorage {
    notifications: std::sync::Arc<tokio::sync::RwLock<Vec<InAppNotification>>>,
}

#[derive(Debug, Clone)]
pub struct InAppNotification {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub body: String,
    pub data: HashMap<String, String>,
    pub priority: NotificationPriority,
    pub read: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl InAppStorage {
    pub fn new() -> Self {
        Self {
            notifications: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    pub async fn store_notification(&self, notification: InAppNotification) -> Result<(), ProviderError> {
        let mut notifications = self.notifications.write().await;
        notifications.push(notification);
        Ok(())
    }

    pub async fn get_user_notifications(&self, user_id: &str) -> Vec<InAppNotification> {
        let notifications = self.notifications.read().await;
        notifications
            .iter()
            .filter(|n| n.user_id == user_id)
            .cloned()
            .collect()
    }

    pub async fn health_check(&self) -> Result<bool, ProviderError> {
        Ok(true)
    }
}
