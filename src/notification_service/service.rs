//! Main notification service orchestrator

use crate::notification_service::{
    providers::{NotificationProvider, EmailProvider, SMSProvider, PushProvider, InAppProvider},
    templates::TemplateManager,
    tracking::DeliveryTracker,
    types::{
        NotificationMessage, Recipient, NotificationChannel, NotificationResult,
        TemplateContext, ProviderConfig, NotificationPriority
    },
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Main notification service
#[derive(Debug)]
pub struct NotificationService {
    template_manager: Arc<RwLock<TemplateManager>>,
    delivery_tracker: Arc<RwLock<DeliveryTracker>>,
    providers: HashMap<NotificationChannel, Arc<dyn NotificationProvider>>,
    rate_limiters: HashMap<NotificationChannel, RateLimiter>,
}

impl NotificationService {
    /// Create a new notification service
    pub fn new() -> Result<Self, ServiceError> {
        let template_manager = Arc::new(RwLock::new(TemplateManager::new()?));
        let delivery_tracker = Arc::new(RwLock::new(DeliveryTracker::new()));
        let mut providers: HashMap<NotificationChannel, Arc<dyn NotificationProvider>> = HashMap::new();
        let mut rate_limiters: HashMap<NotificationChannel, RateLimiter> = HashMap::new();

        // Initialize default providers (disabled by default)
        let default_configs = Self::get_default_provider_configs();
        
        for (channel, config) in default_configs {
            let provider = match channel {
                NotificationChannel::Email => Arc::new(EmailProvider::new(config)?),
                NotificationChannel::SMS => Arc::new(SMSProvider::new(config)?),
                NotificationChannel::Push => Arc::new(PushProvider::new(config)?),
                NotificationChannel::InApp => Arc::new(InAppProvider::new(config)?),
            };
            
            providers.insert(channel.clone(), provider);
            rate_limiters.insert(channel, RateLimiter::new(100, 1000, 10000)); // Default limits
        }

        Ok(Self {
            template_manager,
            delivery_tracker,
            providers,
            rate_limiters,
        })
    }

    /// Send a notification
    pub async fn send_notification(
        &self,
        message: NotificationMessage,
        recipient: Recipient,
    ) -> Result<NotificationResult, ServiceError> {
        let mut delivered_channels = Vec::new();
        let mut failed_channels = Vec::new();
        let mut tracking_ids = Vec::new();

        // Check if we should send based on quiet hours and priority
        if !self.should_send_notification(&message, &recipient) {
            return Ok(NotificationResult {
                notification_id: message.id.clone(),
                success: false,
                delivered_channels: Vec::new(),
                failed_channels: vec![(NotificationChannel::Email, "Notification deferred due to quiet hours".to_string())],
                tracking_ids: Vec::new(),
            });
        }

        // Send through each requested channel
        for channel in &message.channels {
            // Check rate limits
            if !self.rate_limiters.get(channel).map_or(true, |limiter| limiter.check_limit()) {
                failed_channels.push((channel.clone(), "Rate limit exceeded".to_string()));
                continue;
            }

            // Check if recipient has this channel enabled
            if !self.is_channel_enabled_for_recipient(channel, &recipient) {
                failed_channels.push((channel.clone(), "Channel disabled for recipient".to_string()));
                continue;
            }

            // Get provider for this channel
            let provider = self.providers.get(channel)
                .ok_or_else(|| ServiceError::ProviderNotFound(channel.clone()))?;

            // Send notification
            match provider.send_notification(&message, &recipient).await {
                Ok(tracking) => {
                    // Record delivery
                    let mut tracker = self.delivery_tracker.write().await;
                    tracker.record_delivery(tracking.clone()).await?;
                    
                    delivered_channels.push(channel.clone());
                    tracking_ids.push(format!("{}:{}", channel, tracking.recipient_id));
                }
                Err(e) => {
                    failed_channels.push((channel.clone(), e.to_string()));
                }
            }
        }

        let success = !delivered_channels.is_empty();

        Ok(NotificationResult {
            notification_id: message.id,
            success,
            delivered_channels,
            failed_channels,
            tracking_ids,
        })
    }

    /// Send notification using template
    pub async fn send_templated_notification(
        &self,
        template_id: &str,
        recipient: Recipient,
        context: TemplateContext,
        channels: Vec<NotificationChannel>,
        priority: NotificationPriority,
    ) -> Result<NotificationResult, ServiceError> {
        // Render template
        let template_manager = self.template_manager.read().await;
        let rendered = template_manager.render_template(template_id, &context)?;
        
        let message = NotificationMessage {
            id: Uuid::new_v4().to_string(),
            template_id: Some(template_id.to_string()),
            subject: rendered.subject,
            body: rendered.body,
            data: context.clone(),
            priority,
            channels,
            created_at: Utc::now(),
            scheduled_for: None,
        };

        drop(template_manager);
        self.send_notification(message, recipient).await
    }

    /// Schedule a notification for future delivery
    pub async fn schedule_notification(
        &self,
        message: NotificationMessage,
        recipient: Recipient,
        scheduled_for: chrono::DateTime<chrono::Utc>,
    ) -> Result<String, ServiceError> {
        let scheduled_message = NotificationMessage {
            scheduled_for: Some(scheduled_for),
            ..message
        };

        // In a real implementation, this would store in a database and use a job scheduler
        let job_id = Uuid::new_v4().to_string();
        
        // For now, just return the job ID (mock implementation)
        Ok(job_id)
    }

    /// Add a new template
    pub async fn add_template(&self, template: crate::notification_service::templates::NotificationTemplate) -> Result<(), ServiceError> {
        let mut template_manager = self.template_manager.write().await;
        template_manager.add_template(template).map_err(ServiceError::TemplateError)?;
        Ok(())
    }

    /// Update a template
    pub async fn update_template(&self, template: crate::notification_service::templates::NotificationTemplate) -> Result<(), ServiceError> {
        let mut template_manager = self.template_manager.write().await;
        template_manager.update_template(template).map_err(ServiceError::TemplateError)?;
        Ok(())
    }

    /// Get a template
    pub async fn get_template(&self, template_id: &str) -> Result<Option<crate::notification_service::templates::NotificationTemplate>, ServiceError> {
        let template_manager = self.template_manager.read().await;
        Ok(template_manager.get_template(template_id).cloned())
    }

    /// List all templates
    pub async fn list_templates(&self) -> Result<Vec<crate::notification_service::templates::NotificationTemplate>, ServiceError> {
        let template_manager = self.template_manager.read().await;
        Ok(template_manager.list_templates().into_iter().cloned().collect())
    }

    /// Configure a provider
    pub async fn configure_provider(&mut self, channel: NotificationChannel, config: ProviderConfig) -> Result<(), ServiceError> {
        let provider = match channel {
            NotificationChannel::Email => Arc::new(EmailProvider::new(config)?),
            NotificationChannel::SMS => Arc::new(SMSProvider::new(config)?),
            NotificationChannel::Push => Arc::new(PushProvider::new(config)?),
            NotificationChannel::InApp => Arc::new(InAppProvider::new(config)?),
        };

        self.providers.insert(channel.clone(), provider);
        Ok(())
    }

    /// Get delivery tracking
    pub async fn get_delivery_tracking(
        &self,
        notification_id: &str,
        recipient_id: &str,
        channel: NotificationChannel,
    ) -> Result<Option<crate::notification_service::tracking::DeliveryTracking>, ServiceError> {
        let tracker = self.delivery_tracker.read().await;
        tracker.get_tracking(notification_id, recipient_id, channel).await.map_err(ServiceError::TrackingError)
    }

    /// Get delivery statistics
    pub async fn get_delivery_stats(
        &self,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<crate::notification_service::tracking::DeliveryStats, ServiceError> {
        let tracker = self.delivery_tracker.read().await;
        tracker.get_delivery_stats(start_time, end_time).await.map_err(ServiceError::TrackingError)
    }

    /// Health check for all providers
    pub async fn health_check(&self) -> HashMap<NotificationChannel, bool> {
        let mut health_status = HashMap::new();
        
        for (channel, provider) in &self.providers {
            let health = provider.health_check().await.unwrap_or(false);
            health_status.insert(channel.clone(), health);
        }
        
        health_status
    }

    /// Get provider statistics
    pub async fn get_provider_stats(&self) -> HashMap<NotificationChannel, crate::notification_service::providers::ProviderStats> {
        let mut stats = HashMap::new();
        
        for (channel, provider) in &self.providers {
            let provider_stats = provider.get_stats().await;
            stats.insert(channel.clone(), provider_stats);
        }
        
        stats
    }

    /// Check if notification should be sent based on quiet hours and priority
    fn should_send_notification(&self, message: &NotificationMessage, recipient: &Recipient) -> bool {
        // Always send critical priority notifications
        if message.priority >= NotificationPriority::Critical {
            return true;
        }

        // Check quiet hours
        if let Some(quiet_hours) = &recipient.preferences.quiet_hours {
            let now = Utc::now();
            // In a real implementation, would convert to recipient's timezone
            let current_hour = now.hour() as u8;
            
            let in_quiet_hours = if quiet_hours.start_hour <= quiet_hours.end_hour {
                current_hour >= quiet_hours.start_hour && current_hour < quiet_hours.end_hour
            } else {
                current_hour >= quiet_hours.start_hour || current_hour < quiet_hours.end_hour
            };

            if in_quiet_hours && message.priority < recipient.preferences.max_priority {
                return false;
            }
        }

        true
    }

    /// Check if channel is enabled for recipient
    fn is_channel_enabled_for_recipient(&self, channel: &NotificationChannel, recipient: &Recipient) -> bool {
        match channel {
            NotificationChannel::Email => recipient.preferences.email_enabled,
            NotificationChannel::SMS => recipient.preferences.sms_enabled,
            NotificationChannel::Push => recipient.preferences.push_enabled,
            NotificationChannel::InApp => recipient.preferences.in_app_enabled,
        }
    }

    /// Get default provider configurations
    fn get_default_provider_configs() -> HashMap<NotificationChannel, ProviderConfig> {
        let mut configs = HashMap::new();
        
        configs.insert(NotificationChannel::Email, ProviderConfig {
            provider_type: NotificationChannel::Email,
            config: HashMap::new(),
            enabled: false, // Disabled by default
            rate_limit: None,
        });
        
        configs.insert(NotificationChannel::SMS, ProviderConfig {
            provider_type: NotificationChannel::SMS,
            config: HashMap::new(),
            enabled: false, // Disabled by default
            rate_limit: None,
        });
        
        configs.insert(NotificationChannel::Push, ProviderConfig {
            provider_type: NotificationChannel::Push,
            config: HashMap::new(),
            enabled: false, // Disabled by default
            rate_limit: None,
        });
        
        configs.insert(NotificationChannel::InApp, ProviderConfig {
            provider_type: NotificationChannel::InApp,
            config: HashMap::new(),
            enabled: true, // Enabled by default
            rate_limit: None,
        });
        
        configs
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new().expect("Failed to create NotificationService")
    }
}

/// Rate limiter for preventing spam
#[derive(Debug, Clone)]
struct RateLimiter {
    requests_per_second: u32,
    requests_per_minute: u32,
    requests_per_hour: u32,
    // In a real implementation, would track actual request timestamps
}

impl RateLimiter {
    fn new(requests_per_second: u32, requests_per_minute: u32, requests_per_hour: u32) -> Self {
        Self {
            requests_per_second,
            requests_per_minute,
            requests_per_hour,
        }
    }

    fn check_limit(&self) -> bool {
        // Mock implementation - always returns true
        // In a real implementation, would track actual usage
        true
    }
}

/// Service errors
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Provider not found: {0}")]
    ProviderNotFound(NotificationChannel),
    
    #[error("Template error: {0}")]
    TemplateError(#[from] crate::notification_service::templates::TemplateError),
    
    #[error("Tracking error: {0}")]
    TrackingError(#[from] crate::notification_service::tracking::TrackingError),
    
    #[error("Provider error: {0}")]
    ProviderError(#[from] crate::notification_service::providers::ProviderError),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Invalid notification: {0}")]
    InvalidNotification(String),
}

/// Trait for notification services
#[async_trait]
pub trait NotificationServiceTrait: Send + Sync {
    async fn send_notification(
        &self,
        message: NotificationMessage,
        recipient: Recipient,
    ) -> Result<NotificationResult, ServiceError>;
    
    async fn send_templated_notification(
        &self,
        template_id: &str,
        recipient: Recipient,
        context: TemplateContext,
        channels: Vec<NotificationChannel>,
        priority: NotificationPriority,
    ) -> Result<NotificationResult, ServiceError>;
}

#[async_trait]
impl NotificationServiceTrait for NotificationService {
    async fn send_notification(
        &self,
        message: NotificationMessage,
        recipient: Recipient,
    ) -> Result<NotificationResult, ServiceError> {
        self.send_notification(message, recipient).await
    }
    
    async fn send_templated_notification(
        &self,
        template_id: &str,
        recipient: Recipient,
        context: TemplateContext,
        channels: Vec<NotificationChannel>,
        priority: NotificationPriority,
    ) -> Result<NotificationResult, ServiceError> {
        self.send_templated_notification(template_id, recipient, context, channels, priority).await
    }
}
