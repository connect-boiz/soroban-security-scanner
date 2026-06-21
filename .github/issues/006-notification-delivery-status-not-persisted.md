# Issue 6: [Notification Service] Channel Delivery Status Not Persisted Across Service Restarts

## Description

The `NotificationService` in `src/notification_service/service.rs` tracks delivery status for each notification channel (email, SMS, push, in-app) using an in-memory `DeliveryTracker`. When the service restarts (e.g., during deployment, scaling events, or crashes), all delivery tracking data is lost. This means undelivered notifications are silently dropped without retry, and there is no way to audit past notification delivery success rates after a restart. The `DeliveryStats` report that users expect to query for the last 30 days becomes empty after every service restart. The `get_delivery_tracking()` method returns `None` for all previously tracked notifications, even if their delivery was still pending. This is especially critical for security vulnerability alerts where guaranteed delivery is essential.

## Acceptance Criteria

- [ ] Persist delivery tracking records to a database table (see `DATABASE_SCHEMA.md` for notification-related schema)
- [ ] Implement a delivery status recovery process on service startup that re-queues any notifications with `Pending` or `InProgress` status
- [ ] Add a configurable delivery retry policy (max 3 retries with exponential backoff: 5min, 30min, 2h)
- [ ] Create a database migration (`006_add_notification_delivery_tracking.sql`) with indexes on `notification_id`, `channel`, `status`, and `created_at`
- [ ] Update the frontend's `InAppNotification` component to show delivery status for each notification channel
- [ ] Add a new `/api/v1/notifications/delivery-stats` REST endpoint backed by persistent storage

## Additional Context

Key files: `src/notification_service/service.rs`, `src/notification_service/types.rs`, `src/notification_service/tracking.rs`, `src/database/models.rs`, `frontend/components/notifications/InAppNotification.tsx`.
