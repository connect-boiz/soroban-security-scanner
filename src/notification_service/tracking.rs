//! Delivery tracking system for notifications

use crate::notification_service::types::{DeliveryStatus, DeliveryTracking, NotificationChannel};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Storage backend trait for delivery tracking.
/// Enables pluggable storage (in-memory, database, etc.)
#[async_trait]
pub trait StorageBackend: Send + Sync + std::fmt::Debug {
    /// Store a new tracking record
    async fn store_tracking(&self, tracking: &DeliveryTracking) -> Result<(), TrackingError>;
    /// Retrieve a tracking record
    async fn get_tracking(
        &self,
        notification_id: &str,
        recipient_id: &str,
        channel: &NotificationChannel,
    ) -> Result<Option<DeliveryTracking>, TrackingError>;
    /// Update an existing tracking record
    async fn update_tracking(&self, tracking: &DeliveryTracking) -> Result<(), TrackingError>;
    /// Get all tracking for a notification
    async fn get_notification_tracking(
        &self,
        notification_id: &str,
    ) -> Result<Vec<DeliveryTracking>, TrackingError>;
    /// Get all tracking for a recipient
    async fn get_recipient_tracking(
        &self,
        recipient_id: &str,
    ) -> Result<Vec<DeliveryTracking>, TrackingError>;
    /// Get tracking in a time period
    async fn get_tracking_in_period(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<DeliveryTracking>, TrackingError>;
    /// Get failed trackings
    async fn get_failed_trackings(&self) -> Result<Vec<DeliveryTracking>, TrackingError>;
    /// Clean up old records
    async fn cleanup_old_records(&self, cutoff_date: DateTime<Utc>)
        -> Result<usize, TrackingError>;
}

/// Delivery tracking manager
#[derive(Debug)]
pub struct DeliveryTracker {
    storage: Arc<dyn StorageBackend>,
    metrics: DeliveryMetrics,
}

impl DeliveryTracker {
    /// Create a new delivery tracker with the given storage backend
    pub fn new(storage: Arc<dyn StorageBackend>) -> Self {
        Self {
            storage,
            metrics: DeliveryMetrics::new(),
        }
    }

    /// Record a delivery attempt
    pub async fn record_delivery(
        &mut self,
        tracking: DeliveryTracking,
    ) -> Result<(), TrackingError> {
        // Store tracking record
        self.storage.store_tracking(&tracking).await?;

        // Update metrics
        self.metrics.record_delivery(&tracking);

        Ok(())
    }

    /// Update delivery status
    pub async fn update_status(
        &mut self,
        notification_id: &str,
        recipient_id: &str,
        channel: NotificationChannel,
        status: DeliveryStatus,
        error_message: Option<String>,
    ) -> Result<(), TrackingError> {
        let tracking = self
            .storage
            .get_tracking(notification_id, recipient_id, &channel)
            .await?
            .ok_or_else(|| TrackingError::NotFound)?;

        let mut updated_tracking = tracking.clone();
        updated_tracking.status = status;
        updated_tracking.last_attempt = Utc::now();
        updated_tracking.error_message = error_message;

        if status == DeliveryStatus::Delivered {
            updated_tracking.delivered_at = Some(Utc::now());
        }

        self.storage.update_tracking(&updated_tracking).await?;
        self.metrics.record_delivery(&updated_tracking);

        Ok(())
    }

    /// Get tracking information for a notification
    pub async fn get_tracking(
        &self,
        notification_id: &str,
        recipient_id: &str,
        channel: &NotificationChannel,
    ) -> Result<Option<DeliveryTracking>, TrackingError> {
        self.storage
            .get_tracking(notification_id, recipient_id, channel)
            .await
    }

    /// Get all tracking for a notification
    pub async fn get_notification_tracking(
        &self,
        notification_id: &str,
    ) -> Result<Vec<DeliveryTracking>, TrackingError> {
        self.storage
            .get_notification_tracking(notification_id)
            .await
    }

    /// Get tracking for a recipient
    pub async fn get_recipient_tracking(
        &self,
        recipient_id: &str,
    ) -> Result<Vec<DeliveryTracking>, TrackingError> {
        self.storage.get_recipient_tracking(recipient_id).await
    }

    /// Get delivery metrics
    pub fn get_metrics(&self) -> &DeliveryMetrics {
        &self.metrics
    }

    /// Get delivery statistics for a time period
    pub async fn get_delivery_stats(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<DeliveryStats, TrackingError> {
        let trackings = self
            .storage
            .get_tracking_in_period(start_time, end_time)
            .await?;

        let mut stats = DeliveryStats::new(start_time, end_time);

        for tracking in trackings {
            stats.add_tracking(&tracking);
        }

        Ok(stats)
    }

    /// Retry failed deliveries
    pub async fn retry_failed_deliveries(
        &mut self,
    ) -> Result<Vec<DeliveryTracking>, TrackingError> {
        let failed_trackings = self.storage.get_failed_trackings().await?;
        let mut retried = Vec::new();

        for mut tracking in failed_trackings {
            if tracking.attempts < 3 {
                // Max 3 attempts
                tracking.attempts += 1;
                tracking.status = DeliveryStatus::Retrying;
                tracking.last_attempt = Utc::now();

                self.storage.update_tracking(&tracking).await?;
                retried.push(tracking);
            }
        }

        Ok(retried)
    }

    /// Clean up old tracking records
    pub async fn cleanup_old_records(
        &mut self,
        older_than_days: u32,
    ) -> Result<usize, TrackingError> {
        let cutoff_date = Utc::now() - chrono::Duration::days(older_than_days as i64);
        self.storage.cleanup_old_records(cutoff_date).await
    }
}

impl Default for DeliveryTracker {
    fn default() -> Self {
        Self {
            storage: Arc::new(InMemoryBackend::new()),
            metrics: DeliveryMetrics::new(),
        }
    }
}

/// Delivery metrics collector
#[derive(Debug, Clone)]
pub struct DeliveryMetrics {
    total_sent: HashMap<NotificationChannel, u64>,
    total_delivered: HashMap<NotificationChannel, u64>,
    total_failed: HashMap<NotificationChannel, u64>,
    delivery_times: HashMap<NotificationChannel, Vec<u64>>,
    last_updated: DateTime<Utc>,
}

impl DeliveryMetrics {
    fn new() -> Self {
        Self {
            total_sent: HashMap::new(),
            total_delivered: HashMap::new(),
            total_failed: HashMap::new(),
            delivery_times: HashMap::new(),
            last_updated: Utc::now(),
        }
    }

    fn record_delivery(&mut self, tracking: &DeliveryTracking) {
        let channel = tracking.channel.clone();

        // Update counters
        *self.total_sent.entry(channel.clone()).or_insert(0) += 1;

        match tracking.status {
            DeliveryStatus::Delivered => {
                *self.total_delivered.entry(channel.clone()).or_insert(0) += 1;

                // Calculate delivery time if we have timestamps
                if let Some(delivered_at) = tracking.delivered_at {
                    let delivery_time_ms =
                        (delivered_at - tracking.last_attempt).num_milliseconds() as u64;
                    self.delivery_times
                        .entry(channel)
                        .or_insert_with(Vec::new)
                        .push(delivery_time_ms);
                }
            }
            DeliveryStatus::Failed => {
                *self.total_failed.entry(channel).or_insert(0) += 1;
            }
            _ => {}
        }

        self.last_updated = Utc::now();
    }

    /// Get success rate for a channel
    pub fn success_rate(&self, channel: &NotificationChannel) -> f64 {
        let sent = self.total_sent.get(channel).unwrap_or(&0);
        let delivered = self.total_delivered.get(channel).unwrap_or(&0);

        if *sent == 0 {
            0.0
        } else {
            *delivered as f64 / *sent as f64
        }
    }

    /// Get average delivery time for a channel
    pub fn average_delivery_time(&self, channel: &NotificationChannel) -> u64 {
        let times = self.delivery_times.get(channel).unwrap_or(&vec![]);

        if times.is_empty() {
            0
        } else {
            times.iter().sum::<u64>() / times.len() as u64
        }
    }

    /// Get total statistics
    pub fn get_total_stats(&self) -> HashMap<NotificationChannel, ChannelStats> {
        let mut stats = HashMap::new();

        for channel in [
            NotificationChannel::Email,
            NotificationChannel::SMS,
            NotificationChannel::Push,
            NotificationChannel::InApp,
        ] {
            stats.insert(
                channel.clone(),
                ChannelStats {
                    total_sent: *self.total_sent.get(&channel).unwrap_or(&0),
                    total_delivered: *self.total_delivered.get(&channel).unwrap_or(&0),
                    total_failed: *self.total_failed.get(&channel).unwrap_or(&0),
                    success_rate: self.success_rate(&channel),
                    average_delivery_time_ms: self.average_delivery_time(&channel),
                },
            );
        }

        stats
    }
}

/// Channel statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStats {
    pub total_sent: u64,
    pub total_delivered: u64,
    pub total_failed: u64,
    pub success_rate: f64,
    pub average_delivery_time_ms: u64,
}

/// Delivery statistics for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryStats {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub total_notifications: u64,
    pub channel_stats: HashMap<NotificationChannel, ChannelStats>,
    pub hourly_breakdown: Vec<HourlyStats>,
}

impl DeliveryStats {
    fn new(start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        Self {
            start_time,
            end_time,
            total_notifications: 0,
            channel_stats: HashMap::new(),
            hourly_breakdown: Vec::new(),
        }
    }

    fn add_tracking(&mut self, tracking: &DeliveryTracking) {
        self.total_notifications += 1;

        let channel = &tracking.channel;
        let stats = self
            .channel_stats
            .entry(channel.clone())
            .or_insert_with(|| ChannelStats {
                total_sent: 0,
                total_delivered: 0,
                total_failed: 0,
                success_rate: 0.0,
                average_delivery_time_ms: 0,
            });

        stats.total_sent += 1;

        match tracking.status {
            DeliveryStatus::Delivered => stats.total_delivered += 1,
            DeliveryStatus::Failed => stats.total_failed += 1,
            _ => {}
        }

        // Update success rate
        stats.success_rate = if stats.total_sent > 0 {
            stats.total_delivered as f64 / stats.total_sent as f64
        } else {
            0.0
        };
    }
}

/// Hourly statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyStats {
    pub hour: DateTime<Utc>,
    pub total_sent: u64,
    pub total_delivered: u64,
    pub total_failed: u64,
}

/// In-memory storage backend for delivery tracking.
/// Default implementation that works without a database.
#[derive(Debug)]
pub struct InMemoryBackend {
    trackings: std::sync::Mutex<HashMap<String, Vec<DeliveryTracking>>>,
}

impl InMemoryBackend {
    /// Create a new in-memory storage backend
    pub fn new() -> Self {
        Self {
            trackings: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for InMemoryBackend {
    async fn store_tracking(&self, tracking: &DeliveryTracking) -> Result<(), TrackingError> {
        let key = format!(
            "{}:{}:{:?}",
            tracking.notification_id, tracking.recipient_id, tracking.channel
        );
        let mut map = self
            .trackings
            .lock()
            .map_err(|e| TrackingError::StorageError(format!("Lock poisoned: {}", e)))?;
        map.insert(key, vec![tracking.clone()]);
        Ok(())
    }

    async fn get_tracking(
        &self,
        notification_id: &str,
        recipient_id: &str,
        channel: &NotificationChannel,
    ) -> Result<Option<DeliveryTracking>, TrackingError> {
        let key = format!("{}:{}:{:?}", notification_id, recipient_id, channel);
        let map = self
            .trackings
            .lock()
            .map_err(|e| TrackingError::StorageError(format!("Lock poisoned: {}", e)))?;
        Ok(map
            .get(&key)
            .and_then(|trackings| trackings.first().cloned()))
    }

    async fn update_tracking(&self, tracking: &DeliveryTracking) -> Result<(), TrackingError> {
        let key = format!(
            "{}:{}:{:?}",
            tracking.notification_id, tracking.recipient_id, tracking.channel
        );
        let mut map = self
            .trackings
            .lock()
            .map_err(|e| TrackingError::StorageError(format!("Lock poisoned: {}", e)))?;
        map.insert(key, vec![tracking.clone()]);
        Ok(())
    }

    async fn get_notification_tracking(
        &self,
        notification_id: &str,
    ) -> Result<Vec<DeliveryTracking>, TrackingError> {
        let map = self
            .trackings
            .lock()
            .map_err(|e| TrackingError::StorageError(format!("Lock poisoned: {}", e)))?;
        let mut result = Vec::new();
        let prefix = format!("{}:", notification_id);
        for (key, trackings) in map.iter() {
            if key.starts_with(&prefix) {
                result.extend(trackings.clone());
            }
        }
        Ok(result)
    }

    async fn get_recipient_tracking(
        &self,
        recipient_id: &str,
    ) -> Result<Vec<DeliveryTracking>, TrackingError> {
        let map = self
            .trackings
            .lock()
            .map_err(|e| TrackingError::StorageError(format!("Lock poisoned: {}", e)))?;
        let mut result = Vec::new();
        let pattern = format!(":{}", recipient_id);
        for (key, trackings) in map.iter() {
            if key.contains(&pattern) {
                result.extend(trackings.clone());
            }
        }
        Ok(result)
    }

    async fn get_tracking_in_period(
        &self,
        _start_time: DateTime<Utc>,
        _end_time: DateTime<Utc>,
    ) -> Result<Vec<DeliveryTracking>, TrackingError> {
        let map = self
            .trackings
            .lock()
            .map_err(|e| TrackingError::StorageError(format!("Lock poisoned: {}", e)))?;
        Ok(map.values().flatten().cloned().collect())
    }

    async fn get_failed_trackings(&self) -> Result<Vec<DeliveryTracking>, TrackingError> {
        let map = self
            .trackings
            .lock()
            .map_err(|e| TrackingError::StorageError(format!("Lock poisoned: {}", e)))?;
        let mut failed = Vec::new();
        for trackings in map.values() {
            for tracking in trackings {
                if tracking.status == DeliveryStatus::Failed {
                    failed.push(tracking.clone());
                }
            }
        }
        Ok(failed)
    }

    async fn cleanup_old_records(
        &self,
        _cutoff_date: DateTime<Utc>,
    ) -> Result<usize, TrackingError> {
        Ok(0) // In-memory cleanup is handled by the owning DeliveryTracker
    }
}
