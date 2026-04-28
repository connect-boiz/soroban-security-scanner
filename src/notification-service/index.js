//! Notification Service for Soroban Security Scanner
//! 
//! This module provides a comprehensive notification system supporting:
//! - Email notifications
//! - SMS notifications  
//! - Push notifications
//! - In-app alerts
//! - Template management
//! - Delivery tracking

const NotificationService = require('./notification-service');
const TemplateManager = require('./template-manager');
const DeliveryTracker = require('./delivery-tracker');
const { NotificationProvider } = require('./providers');

module.exports = {
  NotificationService,
  TemplateManager,
  DeliveryTracker,
  NotificationProvider,
  // Export types/enums
  NotificationChannel: require('./types').NotificationChannel,
  NotificationPriority: require('./types').NotificationPriority,
  DeliveryStatus: require('./types').DeliveryStatus,
  Recipient: require('./types').Recipient,
  NotificationMessage: require('./types').NotificationMessage,
  NotificationResult: require('./types').NotificationResult,
  NotificationTemplate: require('./types').NotificationTemplate,
  NotificationPreferences: require('./types').NotificationPreferences,
  QuietHours: require('./types').QuietHours,
  ProviderConfig: require('./types').ProviderConfig,
  RateLimit: require('./types').RateLimit,
};
