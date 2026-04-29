//! Types and enums for the notification service

/**
 * Notification channel types
 */
const NotificationChannel = {
  EMAIL: 'email',
  SMS: 'sms',
  PUSH: 'push',
  IN_APP: 'in_app'
};

/**
 * Notification priority levels
 */
const NotificationPriority = {
  LOW: 'low',
  NORMAL: 'normal',
  HIGH: 'high',
  CRITICAL: 'critical'
};

/**
 * Notification delivery status
 */
const DeliveryStatus = {
  PENDING: 'pending',
  PROCESSING: 'processing',
  SENT: 'sent',
  DELIVERED: 'delivered',
  FAILED: 'failed',
  RETRYING: 'retrying'
};

/**
 * Variable types for template validation
 */
const VariableType = {
  STRING: 'string',
  NUMBER: 'number',
  EMAIL: 'email',
  PHONE: 'phone',
  URL: 'url',
  DATETIME: 'datetime',
  BOOLEAN: 'boolean',
  CUSTOM: 'custom'
};

/**
 * Recipient information
 */
class Recipient {
  constructor(data) {
    this.id = data.id;
    this.email = data.email || null;
    this.phone = data.phone || null;
    this.deviceTokens = data.deviceTokens || [];
    this.userId = data.userId || null;
    this.preferences = new NotificationPreferences(data.preferences || {});
  }
}

/**
 * User notification preferences
 */
class NotificationPreferences {
  constructor(data) {
    this.emailEnabled = data.emailEnabled !== false;
    this.smsEnabled = data.smsEnabled !== false;
    this.pushEnabled = data.pushEnabled !== false;
    this.inAppEnabled = data.inAppEnabled !== false;
    this.quietHours = data.quietHours ? new QuietHours(data.quietHours) : null;
    this.maxPriority = data.maxPriority || NotificationPriority.NORMAL;
  }
}

/**
 * Quiet hours configuration
 */
class QuietHours {
  constructor(data) {
    this.startHour = data.startHour || 22; // 10 PM
    this.endHour = data.endHour || 8;     // 8 AM
    this.timezone = data.timezone || 'UTC';
  }
}

/**
 * Notification message
 */
class NotificationMessage {
  constructor(data) {
    this.id = data.id;
    this.templateId = data.templateId || null;
    this.subject = data.subject || null;
    this.body = data.body;
    this.data = data.data || {};
    this.priority = data.priority || NotificationPriority.NORMAL;
    this.channels = data.channels || [NotificationChannel.IN_APP];
    this.createdAt = data.createdAt || new Date();
    this.scheduledFor = data.scheduledFor || null;
  }
}

/**
 * Delivery tracking information
 */
class DeliveryTracking {
  constructor(data) {
    this.notificationId = data.notificationId;
    this.recipientId = data.recipientId;
    this.channel = data.channel;
    this.status = data.status || DeliveryStatus.PENDING;
    this.attempts = data.attempts || 0;
    this.lastAttempt = data.lastAttempt || new Date();
    this.deliveredAt = data.deliveredAt || null;
    this.errorMessage = data.errorMessage || null;
    this.externalId = data.externalId || null;
  }
}

/**
 * Notification result
 */
class NotificationResult {
  constructor(data) {
    this.notificationId = data.notificationId;
    this.success = data.success || false;
    this.deliveredChannels = data.deliveredChannels || [];
    this.failedChannels = data.failedChannels || [];
    this.trackingIds = data.trackingIds || [];
  }
}

/**
 * Template definition
 */
class NotificationTemplate {
  constructor(data) {
    this.id = data.id;
    this.name = data.name;
    this.description = data.description || null;
    this.subjectTemplate = data.subjectTemplate || null;
    this.bodyTemplate = data.bodyTemplate;
    this.supportedChannels = data.supportedChannels || [NotificationChannel.EMAIL];
    this.defaultPriority = data.defaultPriority || NotificationPriority.NORMAL;
    this.variables = (data.variables || []).map(v => new TemplateVariable(v));
    this.createdAt = data.createdAt || new Date();
    this.updatedAt = data.updatedAt || new Date();
    this.version = data.version || 1;
    this.active = data.active !== false;
  }
}

/**
 * Template variable definition
 */
class TemplateVariable {
  constructor(data) {
    this.name = data.name;
    this.description = data.description || null;
    this.required = data.required || false;
    this.defaultValue = data.defaultValue || null;
    this.variableType = data.variableType || VariableType.STRING;
  }
}

/**
 * Provider configuration
 */
class ProviderConfig {
  constructor(data) {
    this.providerType = data.providerType;
    this.config = data.config || {};
    this.enabled = data.enabled !== false;
    this.rateLimit = data.rateLimit ? new RateLimit(data.rateLimit) : null;
  }
}

/**
 * Rate limiting configuration
 */
class RateLimit {
  constructor(data) {
    this.maxRequestsPerSecond = data.maxRequestsPerSecond || 10;
    this.maxRequestsPerMinute = data.maxRequestsPerMinute || 100;
    this.maxRequestsPerHour = data.maxRequestsPerHour || 1000;
  }
}

/**
 * Rendered template result
 */
class TemplateRender {
  constructor(data) {
    this.subject = data.subject || null;
    this.body = data.body;
    this.templateId = data.templateId;
  }
}

/**
 * Channel statistics
 */
class ChannelStats {
  constructor(data) {
    this.totalSent = data.totalSent || 0;
    this.totalDelivered = data.totalDelivered || 0;
    this.totalFailed = data.totalFailed || 0;
    this.successRate = data.successRate || 0;
    this.averageDeliveryTimeMs = data.averageDeliveryTimeMs || 0;
  }
}

/**
 * Delivery statistics for a time period
 */
class DeliveryStats {
  constructor(data) {
    this.startTime = data.startTime || new Date();
    this.endTime = data.endTime || new Date();
    this.totalNotifications = data.totalNotifications || 0;
    this.channelStats = data.channelStats || {};
    this.hourlyBreakdown = data.hourlyBreakdown || [];
  }
}

/**
 * Hourly statistics
 */
class HourlyStats {
  constructor(data) {
    this.hour = data.hour || new Date();
    this.totalSent = data.totalSent || 0;
    this.totalDelivered = data.totalDelivered || 0;
    this.totalFailed = data.totalFailed || 0;
  }
}

/**
 * Provider statistics
 */
class ProviderStats {
  constructor(data) {
    this.channel = data.channel;
    this.totalSent = data.totalSent || 0;
    this.totalFailed = data.totalFailed || 0;
    this.averageDeliveryTimeMs = data.averageDeliveryTimeMs || 0;
    this.lastSuccess = data.lastSuccess || null;
    this.lastFailure = data.lastFailure || null;
  }
}

module.exports = {
  NotificationChannel,
  NotificationPriority,
  DeliveryStatus,
  VariableType,
  Recipient,
  NotificationPreferences,
  QuietHours,
  NotificationMessage,
  DeliveryTracking,
  NotificationResult,
  NotificationTemplate,
  TemplateVariable,
  ProviderConfig,
  RateLimit,
  TemplateRender,
  ChannelStats,
  DeliveryStats,
  HourlyStats,
  ProviderStats
};
