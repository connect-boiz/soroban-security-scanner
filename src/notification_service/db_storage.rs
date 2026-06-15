//! Database-backed delivery tracking storage backend.
//!
//! This module provides persistent delivery tracking using PostgreSQL.
//! Available when the `database` feature is enabled.

use crate::database::Database;
use crate::database::models::{
    CreateDeliveryTrackingRequest, DeliveryDbStatus, UpdateDeliveryStatusRequest,
};
use crate::notification_service::tracking::StorageBackend;
use crate::notification_service::types::{
    DeliveryStatus, DeliveryTracking, NotificationChannel, TrackingError,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Database-backed storage backend for delivery tracking.
/// Persists delivery status to PostgreSQL so it survives restarts.
#[derive(Debug, Clone)]
pub struct DatabaseBackend {
    db: Arc<Database>,
}

impl DatabaseBackend {
    /// Create a new database-backed storage backend
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Helper: convert a DB model DeliveryDbStatus to the notification service DeliveryStatus
    fn db_status_to_delivery_status(status: &DeliveryDbStatus) -> DeliveryStatus {
        match status {
            DeliveryDbStatus::Pending => DeliveryStatus::Pending,
            DeliveryDbStatus::Processing => DeliveryStatus::Processing,
            DeliveryDbStatus::Sent => DeliveryStatus::Sent,
            DeliveryDbStatus::Delivered => DeliveryStatus::Delivered,
            DeliveryDbStatus::Failed => DeliveryStatus::Failed,
            DeliveryDbStatus::Retrying => DeliveryStatus::Retrying,
        }
    }

    /// Helper: convert a notification service DeliveryStatus to the DB model DeliveryDbStatus
    fn delivery_status_to_db_status(status: &DeliveryStatus) -> DeliveryDbStatus {
        match status {
            DeliveryStatus::Pending => DeliveryDbStatus::Pending,
            DeliveryStatus::Processing => DeliveryDbStatus::Processing,
            DeliveryStatus::Sent => DeliveryDbStatus::Sent,
            DeliveryStatus::Delivered => DeliveryDbStatus::Delivered,
            DeliveryStatus::Failed => DeliveryDbStatus::Failed,
            DeliveryStatus::Retrying => DeliveryDbStatus::Retrying,
        }
    }

    /// Helper: convert a NotificationChannel to a string slug
    fn channel_to_string(channel: &NotificationChannel) -> String {
        match channel {
            NotificationChannel::Email => "email".to_string(),
            NotificationChannel::SMS => "sms".to_string(),
            NotificationChannel::Push => "push".to_string(),
            NotificationChannel::InApp => "in_app".to_string(),
        }
    }

    /// Helper: convert a string slug to a NotificationChannel
    fn string_to_channel(s: &str) -> Option<NotificationChannel> {
        match s {
            "email" => Some(NotificationChannel::Email),
            "sms" => Some(NotificationChannel::SMS),
            "push" => Some(NotificationChannel::Push),
            "in_app" => Some(NotificationChannel::InApp),
            _ => None,
        }
    }

    /// Helper: map database row to DeliveryTracking
    fn db_row_to_delivery_tracking(
        row: &crate::database::models::NotificationDeliveryTracking,
    ) -> Result<DeliveryTracking, TrackingError> {
        let channel = Self::string_to_channel(&row.channel)
            .ok_or_else(|| TrackingError::InvalidData(format!("Unknown channel: {}", row.channel)))?;

        let status = Self::db_status_to_delivery_status(&row.status);

        Ok(DeliveryTracking {
            notification_id: row.notification_id.clone(),
            recipient_id: row.recipient_id.clone(),
            channel,
            status,
            attempts: row.attempts as u32,
            last_attempt: row.last_attempt_at.unwrap_or(row.updated_at),
            delivered_at: row.delivered_at,
            error_message: row.error_message.clone(),
            external_id: row.external_id.clone(),
        })
    }
}

#[async_trait]
impl StorageBackend for DatabaseBackend {
    async fn store_tracking(&self, tracking: &DeliveryTracking) -> Result<(), TrackingError> {
        let request = CreateDeliveryTrackingRequest {
            notification_id: tracking.notification_id.clone(),
            recipient_id: tracking.recipient_id.clone(),
            channel: Self::channel_to_string(&tracking.channel),
            metadata: None,
        };

        self.db
            .create_delivery_tracking(request)
            .await
            .map_err(|e| TrackingError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn get_tracking(
        &self,
        notification_id: &str,
        recipient_id: &str,
        channel: &NotificationChannel,
    ) -> Result<Option<DeliveryTracking>, TrackingError> {
        let channel_str = Self::channel_to_string(channel);
        let row = self
            .db
            .get_delivery_tracking(notification_id, recipient_id, &channel_str)
            .await
            .map_err(|e| TrackingError::StorageError(e.to_string()))?;

        match row {
            Some(r) => Ok(Some(Self::db_row_to_delivery_tracking(&r)?)),
            None => Ok(None),
        }
    }

    async fn update_tracking(&self, tracking: &DeliveryTracking) -> Result<(), TrackingError> {
        let channel_str = Self::channel_to_string(&tracking.channel);
        let db_status = Self::delivery_status_to_db_status(&tracking.status);

        let update = UpdateDeliveryStatusRequest {
            status: db_status,
            error_message: tracking.error_message.clone(),
            external_id: tracking.external_id.clone(),
            delivered_at: tracking.delivered_at,
        };

        self.db
            .update_delivery_status(&tracking.notification_id, &tracking.recipient_id, &channel_str, update)
            .await
            .map_err(|e| TrackingError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn get_notification_tracking(&self, notification_id: &str) -> Result<Vec<DeliveryTracking>, TrackingError> {
        let rows = self
            .db
            .get_notification_delivery_trackings(notification_id)
            .await
            .map_err(|e| TrackingError::StorageError(e.to_string()))?;

        rows.iter()
            .map(|r| Self::db_row_to_delivery_tracking(r))
            .collect()
    }

    async fn get_recipient_tracking(&self, recipient_id: &str) -> Result<Vec<DeliveryTracking>, TrackingError> {
        let rows = self
            .db
            .get_recipient_delivery_trackings(recipient_id)
            .await
            .map_err(|e| TrackingError::StorageError(e.to_string()))?;

        rows.iter()
            .map(|r| Self::db_row_to_delivery_tracking(r))
            .collect()
    }

    async fn get_tracking_in_period(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<DeliveryTracking>, TrackingError> {
        let rows = self
            .db
            .get_delivery_stats_in_period(start_time, end_time)
            .await
            .map_err(|e| TrackingError::StorageError(e.to_string()))?;

        rows.iter()
            .map(|r| Self::db_row_to_delivery_tracking(r))
            .collect()
    }

    async fn get_failed_trackings(&self) -> Result<Vec<DeliveryTracking>, TrackingError> {
        let rows = self
            .db
            .get_failed_delivery_trackings(3)
            .await
            .map_err(|e| TrackingError::StorageError(e.to_string()))?;

        rows.iter()
            .map(|r| Self::db_row_to_delivery_tracking(r))
            .collect()
    }

    async fn cleanup_old_records(&self, cutoff_date: DateTime<Utc>) -> Result<usize, TrackingError> {
        // Calculate retention days from the cutoff date
        let now = Utc::now();
        let days = (now - cutoff_date).num_days() as i32;
        if days <= 0 {
            return Ok(0);
        }

        self.db
            .cleanup_old_delivery_records(days)
            .await
            .map(|count| count as usize)
            .map_err(|e| TrackingError::StorageError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notification_service::types::DeliveryStatus;

    /// Test the channel string conversion helpers
    #[test]
    fn test_channel_conversion() {
        assert_eq!(DatabaseBackend::channel_to_string(&NotificationChannel::Email), "email");
        assert_eq!(DatabaseBackend::channel_to_string(&NotificationChannel::SMS), "sms");
        assert_eq!(DatabaseBackend::channel_to_string(&NotificationChannel::Push), "push");
        assert_eq!(DatabaseBackend::channel_to_string(&NotificationChannel::InApp), "in_app");

        assert_eq!(DatabaseBackend::string_to_channel("email"), Some(NotificationChannel::Email));
        assert_eq!(DatabaseBackend::string_to_channel("sms"), Some(NotificationChannel::SMS));
        assert_eq!(DatabaseBackend::string_to_channel("push"), Some(NotificationChannel::Push));
        assert_eq!(DatabaseBackend::string_to_channel("in_app"), Some(NotificationChannel::InApp));
        assert_eq!(DatabaseBackend::string_to_channel("unknown"), None);
    }

    /// Test the status conversion helpers
    #[test]
    fn test_status_conversion() {
        assert_eq!(
            DatabaseBackend::db_status_to_delivery_status(&DeliveryDbStatus::Pending),
            DeliveryStatus::Pending
        );
        assert_eq!(
            DatabaseBackend::db_status_to_delivery_status(&DeliveryDbStatus::Delivered),
            DeliveryStatus::Delivered
        );
        assert_eq!(
            DatabaseBackend::db_status_to_delivery_status(&DeliveryDbStatus::Failed),
            DeliveryStatus::Failed
        );

        assert_eq!(
            DatabaseBackend::delivery_status_to_db_status(&DeliveryStatus::Retrying),
            DeliveryDbStatus::Retrying
        );
        assert_eq!(
            DatabaseBackend::delivery_status_to_db_status(&DeliveryStatus::Sent),
            DeliveryDbStatus::Sent
        );
    }
}
