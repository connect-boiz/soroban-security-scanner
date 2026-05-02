//! Main notification service orchestrator

const { v4: uuidv4 } = require('uuid');
const {
  NotificationChannel,
  NotificationPriority,
  DeliveryStatus,
  Recipient,
  NotificationMessage,
  NotificationResult,
  ProviderConfig,
  RateLimit
} = require('./types');

const { TemplateManager, TemplateError } = require('./template-manager');
const { DeliveryTracker, TrackingError } = require('./delivery-tracker');
const { ProviderFactory, ProviderError } = require('./providers');

class ServiceError extends Error {
  constructor(message, code) {
    super(message);
    this.name = 'ServiceError';
    this.code = code;
  }
}

/**
 * Rate limiter for preventing spam
 */
class RateLimiter {
  constructor(maxRequestsPerSecond = 10, maxRequestsPerMinute = 100, maxRequestsPerHour = 1000) {
    this.maxRequestsPerSecond = maxRequestsPerSecond;
    this.maxRequestsPerMinute = maxRequestsPerMinute;
    this.maxRequestsPerHour = maxRequestsPerHour;
    this.requests = [];
  }

  /**
   * Check if request is allowed
   */
  checkLimit() {
    const now = Date.now();
    
    // Clean old requests
    this.requests = this.requests.filter(timestamp => 
      now - timestamp < 3600000 // 1 hour
    );

    // Check limits
    const requestsLastSecond = this.requests.filter(timestamp => now - timestamp < 1000).length;
    const requestsLastMinute = this.requests.filter(timestamp => now - timestamp < 60000).length;
    const requestsLastHour = this.requests.length;

    if (requestsLastSecond >= this.maxRequestsPerSecond ||
        requestsLastMinute >= this.maxRequestsPerMinute ||
        requestsLastHour >= this.maxRequestsPerHour) {
      return false;
    }

    this.requests.push(now);
    return true;
  }
}

/**
 * Main notification service
 */
class NotificationService {
  constructor() {
    this.templateManager = new TemplateManager();
    this.deliveryTracker = new DeliveryTracker();
    this.providers = new Map();
    this.rateLimiters = new Map();
    
    this.initializeDefaultProviders();
    this.templateManager.createDefaultTemplates();
  }

  /**
   * Initialize default providers
   */
  initializeDefaultProviders() {
    const defaultConfigs = ProviderFactory.getDefaultConfigs();
    
    for (const [channel, config] of Object.entries(defaultConfigs)) {
      try {
        const provider = ProviderFactory.createProvider(channel, config);
        this.providers.set(channel, provider);
        this.rateLimiters.set(channel, new RateLimiter());
      } catch (error) {
        console.warn(`Failed to initialize ${channel} provider:`, error.message);
      }
    }
  }

  /**
   * Send a notification
   */
  async sendNotification(message, recipient) {
    if (!(message instanceof NotificationMessage)) {
      message = new NotificationMessage(message);
    }
    if (!(recipient instanceof Recipient)) {
      recipient = new Recipient(recipient);
    }

    const deliveredChannels = [];
    const failedChannels = [];
    const trackingIds = [];

    // Check if we should send based on quiet hours and priority
    if (!this.shouldSendNotification(message, recipient)) {
      return new NotificationResult({
        notificationId: message.id,
        success: false,
        deliveredChannels: [],
        failedChannels: [[NotificationChannel.EMAIL, 'Notification deferred due to quiet hours']],
        trackingIds: []
      });
    }

    // Send through each requested channel
    for (const channel of message.channels) {
      try {
        // Check rate limits
        const rateLimiter = this.rateLimiters.get(channel);
        if (rateLimiter && !rateLimiter.checkLimit()) {
          failedChannels.push([channel, 'Rate limit exceeded']);
          continue;
        }

        // Check if recipient has this channel enabled
        if (!this.isChannelEnabledForRecipient(channel, recipient)) {
          failedChannels.push([channel, 'Channel disabled for recipient']);
          continue;
        }

        // Get provider for this channel
        const provider = this.providers.get(channel);
        if (!provider) {
          failedChannels.push([channel, `Provider not configured for ${channel}`]);
          continue;
        }

        // Send notification
        const tracking = await provider.sendNotification(message, recipient);
        
        // Record delivery
        await this.deliveryTracker.recordDelivery(tracking);
        
        deliveredChannels.push(channel);
        trackingIds.push(`${channel}:${tracking.recipientId}`);
      } catch (error) {
        failedChannels.push([channel, error.message]);
      }
    }

    const success = deliveredChannels.length > 0;

    return new NotificationResult({
      notificationId: message.id,
      success,
      deliveredChannels,
      failedChannels,
      trackingIds
    });
  }

  /**
   * Send notification using template
   */
  async sendTemplatedNotification(templateId, recipient, context, channels, priority) {
    if (!(recipient instanceof Recipient)) {
      recipient = new Recipient(recipient);
    }

    // Render template
    const rendered = this.templateManager.renderTemplate(templateId, context);
    
    const message = new NotificationMessage({
      id: uuidv4(),
      templateId,
      subject: rendered.subject,
      body: rendered.body,
      data: context,
      priority,
      channels,
      createdAt: new Date()
    });

    return this.sendNotification(message, recipient);
  }

  /**
   * Schedule a notification for future delivery
   */
  async scheduleNotification(message, recipient, scheduledFor) {
    if (!(message instanceof NotificationMessage)) {
      message = new NotificationMessage(message);
    }
    if (!(recipient instanceof Recipient)) {
      recipient = new Recipient(recipient);
    }

    const scheduledMessage = new NotificationMessage({
      ...message,
      scheduledFor
    });

    // In a real implementation, this would store in a database and use a job scheduler
    const jobId = uuidv4();
    
    // Mock implementation - just return the job ID
    console.log(`Notification scheduled for ${scheduledFor} with job ID: ${jobId}`);
    
    return jobId;
  }

  /**
   * Add a new template
   */
  async addTemplate(template) {
    try {
      return this.templateManager.addTemplate(template);
    } catch (error) {
      throw new ServiceError(`Template error: ${error.message}`, 'TEMPLATE_ERROR');
    }
  }

  /**
   * Update a template
   */
  async updateTemplate(template) {
    try {
      return this.templateManager.updateTemplate(template);
    } catch (error) {
      throw new ServiceError(`Template error: ${error.message}`, 'TEMPLATE_ERROR');
    }
  }

  /**
   * Get a template
   */
  async getTemplate(templateId) {
    return this.templateManager.getTemplate(templateId);
  }

  /**
   * List all templates
   */
  async listTemplates() {
    return this.templateManager.listTemplates();
  }

  /**
   * Configure a provider
   */
  async configureProvider(channel, config) {
    try {
      const provider = ProviderFactory.createProvider(channel, config);
      this.providers.set(channel, provider);
      
      // Update rate limiter if specified
      if (config.rateLimit) {
        this.rateLimiters.set(channel, new RateLimiter(
          config.rateLimit.maxRequestsPerSecond,
          config.rateLimit.maxRequestsPerMinute,
          config.rateLimit.maxRequestsPerHour
        ));
      }
      
      return true;
    } catch (error) {
      throw new ServiceError(`Provider configuration error: ${error.message}`, 'CONFIGURATION_ERROR');
    }
  }

  /**
   * Get delivery tracking
   */
  async getDeliveryTracking(notificationId, recipientId, channel) {
    try {
      return await this.deliveryTracker.getTracking(notificationId, recipientId, channel);
    } catch (error) {
      throw new ServiceError(`Tracking error: ${error.message}`, 'TRACKING_ERROR');
    }
  }

  /**
   * Get delivery statistics
   */
  async getDeliveryStats(startTime, endTime) {
    try {
      return await this.deliveryTracker.getDeliveryStats(startTime, endTime);
    } catch (error) {
      throw new ServiceError(`Tracking error: ${error.message}`, 'TRACKING_ERROR');
    }
  }

  /**
   * Health check for all providers
   */
  async healthCheck() {
    const healthStatus = {};
    
    for (const [channel, provider] of this.providers) {
      try {
        const health = await provider.healthCheck();
        healthStatus[channel] = health;
      } catch (error) {
        healthStatus[channel] = false;
      }
    }
    
    return healthStatus;
  }

  /**
   * Get provider statistics
   */
  async getProviderStats() {
    const stats = {};
    
    for (const [channel, provider] of this.providers) {
      try {
        const providerStats = await provider.getStats();
        stats[channel] = providerStats;
      } catch (error) {
        console.error(`Failed to get stats for ${channel}:`, error.message);
      }
    }
    
    return stats;
  }

  /**
   * Check if notification should be sent based on quiet hours and priority
   */
  shouldSendNotification(message, recipient) {
    // Always send critical priority notifications
    if (message.priority === NotificationPriority.CRITICAL) {
      return true;
    }

    // Check quiet hours
    if (recipient.preferences.quietHours) {
      const now = new Date();
      const currentHour = now.getHours();
      
      const { startHour, endHour } = recipient.preferences.quietHours;
      
      let inQuietHours;
      if (startHour <= endHour) {
        inQuietHours = currentHour >= startHour && currentHour < endHour;
      } else {
        inQuietHours = currentHour >= startHour || currentHour < endHour;
      }

      if (inQuietHours && message.priority !== NotificationPriority.CRITICAL) {
        return false;
      }
    }

    return true;
  }

  /**
   * Check if channel is enabled for recipient
   */
  isChannelEnabledForRecipient(channel, recipient) {
    switch (channel) {
      case NotificationChannel.EMAIL:
        return recipient.preferences.emailEnabled;
      case NotificationChannel.SMS:
        return recipient.preferences.smsEnabled;
      case NotificationChannel.PUSH:
        return recipient.preferences.pushEnabled;
      case NotificationChannel.IN_APP:
        return recipient.preferences.inAppEnabled;
      default:
        return false;
    }
  }

  /**
   * Get in-app notifications for a user
   */
  getInAppNotifications(userId) {
    const inAppProvider = this.providers.get(NotificationChannel.IN_APP);
    if (inAppProvider) {
      return inAppProvider.getUserNotifications(userId);
    }
    return [];
  }

  /**
   * Mark in-app notification as read
   */
  markInAppNotificationAsRead(userId, notificationId) {
    const inAppProvider = this.providers.get(NotificationChannel.IN_APP);
    if (inAppProvider) {
      return inAppProvider.markAsRead(userId, notificationId);
    }
    return false;
  }
}

module.exports = {
  NotificationService,
  ServiceError,
  RateLimiter
};
