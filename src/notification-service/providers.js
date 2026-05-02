//! Notification providers for different channels

const nodemailer = require('nodemailer');
const {
  NotificationChannel,
  DeliveryStatus,
  DeliveryTracking,
  ProviderStats,
  NotificationMessage,
  Recipient
} = require('./types');

class ProviderError extends Error {
  constructor(message, code) {
    super(message);
    this.name = 'ProviderError';
    this.code = code;
  }
}

/**
 * Base provider class
 */
class NotificationProvider {
  constructor(config) {
    this.config = config;
    this.stats = new ProviderStats({
      channel: config.providerType,
      totalSent: 0,
      totalFailed: 0,
      averageDeliveryTimeMs: 0
    });
  }

  /**
   * Send a notification (to be implemented by subclasses)
   */
  async sendNotification(message, recipient) {
    throw new Error('sendNotification must be implemented by subclass');
  }

  /**
   * Get provider type
   */
  channel() {
    return this.config.providerType;
  }

  /**
   * Check if provider is healthy
   */
  async healthCheck() {
    return this.config.enabled;
  }

  /**
   * Get provider statistics
   */
  async getStats() {
    return this.stats;
  }

  /**
   * Update statistics
   */
  updateStats(success, deliveryTime = 0) {
    if (success) {
      this.stats.totalSent++;
      if (deliveryTime > 0) {
        // Update average delivery time
        const totalTime = this.stats.averageDeliveryTimeMs * (this.stats.totalSent - 1) + deliveryTime;
        this.stats.averageDeliveryTimeMs = Math.round(totalTime / this.stats.totalSent);
      }
      this.stats.lastSuccess = new Date();
    } else {
      this.stats.totalFailed++;
      this.stats.lastFailure = new Date();
    }
  }
}

/**
 * Email notification provider
 */
class EmailProvider extends NotificationProvider {
  constructor(config) {
    super(config);
    this.transporter = null;
    this.initializeTransporter();
  }

  /**
   * Initialize SMTP transporter
   */
  initializeTransporter() {
    if (!this.config.enabled) {
      return;
    }

    const smtpConfig = this.config.config;
    if (!smtpConfig.smtp_host || !smtpConfig.username || !smtpConfig.password) {
      console.warn('Email provider not properly configured');
      return;
    }

    this.transporter = nodemailer.createTransporter({
      host: smtpConfig.smtp_host,
      port: parseInt(smtpConfig.smtp_port) || 587,
      secure: smtpConfig.secure === 'true',
      auth: {
        user: smtpConfig.username,
        pass: smtpConfig.password
      }
    });
  }

  /**
   * Send email notification
   */
  async sendNotification(message, recipient) {
    if (!this.config.enabled) {
      throw new ProviderError('Email provider is disabled', 'PROVIDER_DISABLED');
    }

    if (!recipient.email) {
      throw new ProviderError('Recipient email is required', 'MISSING_RECIPIENT_DATA');
    }

    if (!this.transporter) {
      throw new ProviderError('Email transporter not configured', 'PROVIDER_NOT_CONFIGURED');
    }

    const startTime = Date.now();
    const trackingId = `email_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

    try {
      const mailOptions = {
        from: `${this.config.config.from_name || 'Soroban Security Scanner'} <${this.config.config.from_email || this.config.config.username}>`,
        to: recipient.email,
        subject: message.subject || 'Notification',
        text: message.body,
        html: this.formatAsHtml(message.body)
      };

      const result = await this.transporter.sendMail(mailOptions);
      const deliveryTime = Date.now() - startTime;

      this.updateStats(true, deliveryTime);

      return new DeliveryTracking({
        notificationId: message.id,
        recipientId: recipient.id,
        channel: NotificationChannel.EMAIL,
        status: DeliveryStatus.SENT,
        attempts: 1,
        lastAttempt: new Date(),
        deliveredAt: new Date(),
        externalId: result.messageId
      });
    } catch (error) {
      const deliveryTime = Date.now() - startTime;
      this.updateStats(false, deliveryTime);

      return new DeliveryTracking({
        notificationId: message.id,
        recipientId: recipient.id,
        channel: NotificationChannel.EMAIL,
        status: DeliveryStatus.FAILED,
        attempts: 1,
        lastAttempt: new Date(),
        errorMessage: error.message
      });
    }
  }

  /**
   * Format message as HTML
   */
  formatAsHtml(text) {
    return text
      .replace(/\n/g, '<br>')
      .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.*?)\*/g, '<em>$1</em>');
  }

  /**
   * Health check for email provider
   */
  async healthCheck() {
    if (!this.config.enabled || !this.transporter) {
      return false;
    }

    try {
      await this.transporter.verify();
      return true;
    } catch (error) {
      console.error('Email provider health check failed:', error.message);
      return false;
    }
  }
}

/**
 * SMS notification provider (mock implementation)
 */
class SMSProvider extends NotificationProvider {
  constructor(config) {
    super(config);
    this.client = this.initializeClient();
  }

  /**
   * Initialize SMS client (mock)
   */
  initializeClient() {
    if (!this.config.enabled) {
      return null;
    }

    // In a real implementation, would initialize Twilio or other SMS service
    return {
      accountSid: this.config.config.account_sid,
      authToken: this.config.config.auth_token,
      fromNumber: this.config.config.from_number
    };
  }

  /**
   * Send SMS notification
   */
  async sendNotification(message, recipient) {
    if (!this.config.enabled) {
      throw new ProviderError('SMS provider is disabled', 'PROVIDER_DISABLED');
    }

    if (!recipient.phone) {
      throw new ProviderError('Recipient phone is required', 'MISSING_RECIPIENT_DATA');
    }

    if (!this.client) {
      throw new ProviderError('SMS client not configured', 'PROVIDER_NOT_CONFIGURED');
    }

    const startTime = Date.now();
    const trackingId = `sms_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

    try {
      // Truncate message for SMS (160 characters typical limit)
      const smsBody = message.body.length > 160 
        ? message.body.substring(0, 157) + '...' 
        : message.body;

      // Mock implementation - would use real SMS service
      console.log(`SMS sent to ${recipient.phone}: ${smsBody}`);
      
      const deliveryTime = Date.now() - startTime;
      this.updateStats(true, deliveryTime);

      return new DeliveryTracking({
        notificationId: message.id,
        recipientId: recipient.id,
        channel: NotificationChannel.SMS,
        status: DeliveryStatus.SENT,
        attempts: 1,
        lastAttempt: new Date(),
        deliveredAt: new Date(),
        externalId: trackingId
      });
    } catch (error) {
      const deliveryTime = Date.now() - startTime;
      this.updateStats(false, deliveryTime);

      return new DeliveryTracking({
        notificationId: message.id,
        recipientId: recipient.id,
        channel: NotificationChannel.SMS,
        status: DeliveryStatus.FAILED,
        attempts: 1,
        lastAttempt: new Date(),
        errorMessage: error.message
      });
    }
  }

  /**
   * Health check for SMS provider
   */
  async healthCheck() {
    return this.config.enabled && this.client !== null;
  }
}

/**
 * Push notification provider (mock implementation)
 */
class PushProvider extends NotificationProvider {
  constructor(config) {
    super(config);
    this.fcm = this.initializeFCM();
    this.apns = this.initializeAPNS();
  }

  /**
   * Initialize FCM client (mock)
   */
  initializeFCM() {
    if (!this.config.enabled || !this.config.config.fcm_server_key) {
      return null;
    }

    // In a real implementation, would initialize Firebase Cloud Messaging
    return {
      serverKey: this.config.config.fcm_server_key
    };
  }

  /**
   * Initialize APNS client (mock)
   */
  initializeAPNS() {
    if (!this.config.enabled || !this.config.config.apns_key_id) {
      return null;
    }

    // In a real implementation, would initialize Apple Push Notification Service
    return {
      keyId: this.config.config.apns_key_id,
      teamId: this.config.config.apns_team_id
    };
  }

  /**
   * Send push notification
   */
  async sendNotification(message, recipient) {
    if (!this.config.enabled) {
      throw new ProviderError('Push provider is disabled', 'PROVIDER_DISABLED');
    }

    if (!recipient.deviceTokens || recipient.deviceTokens.length === 0) {
      throw new ProviderError('No device tokens available', 'MISSING_RECIPIENT_DATA');
    }

    const startTime = Date.now();
    const trackingId = `push_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    let successCount = 0;
    let failureCount = 0;
    const errors = [];

    // Send to all device tokens
    for (const deviceToken of recipient.deviceTokens) {
      try {
        // Mock implementation - would use real push service
        console.log(`Push sent to device ${deviceToken}: ${message.subject}`);
        successCount++;
      } catch (error) {
        failureCount++;
        errors.push(error.message);
      }
    }

    const deliveryTime = Date.now() - startTime;
    const overallSuccess = successCount > 0;

    this.updateStats(overallSuccess, deliveryTime);

    return new DeliveryTracking({
      notificationId: message.id,
      recipientId: recipient.id,
      channel: NotificationChannel.PUSH,
      status: overallSuccess ? DeliveryStatus.SENT : DeliveryStatus.FAILED,
      attempts: 1,
      lastAttempt: new Date(),
      deliveredAt: overallSuccess ? new Date() : null,
      errorMessage: errors.length > 0 ? errors.join(', ') : null,
      externalId: overallSuccess ? trackingId : null
    });
  }

  /**
   * Health check for push provider
   */
  async healthCheck() {
    return this.config.enabled && (this.fcm !== null || this.apns !== null);
  }
}

/**
 * In-app notification provider
 */
class InAppProvider extends NotificationProvider {
  constructor(config) {
    super(config);
    this.storage = new Map(); // In-memory storage for demo
  }

  /**
   * Send in-app notification
   */
  async sendNotification(message, recipient) {
    if (!this.config.enabled) {
      throw new ProviderError('In-app provider is disabled', 'PROVIDER_DISABLED');
    }

    const startTime = Date.now();
    const trackingId = `inapp_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

    try {
      const notification = {
        id: trackingId,
        userId: recipient.userId || recipient.id,
        title: message.subject || 'Notification',
        body: message.body,
        data: message.data,
        priority: message.priority,
        read: false,
        createdAt: new Date(),
        expiresAt: message.scheduledFor
      };

      // Store in-app notification (in real implementation, would use database)
      const userNotifications = this.storage.get(notification.userId) || [];
      userNotifications.push(notification);
      this.storage.set(notification.userId, userNotifications);

      const deliveryTime = Date.now() - startTime;
      this.updateStats(true, deliveryTime);

      return new DeliveryTracking({
        notificationId: message.id,
        recipientId: recipient.id,
        channel: NotificationChannel.IN_APP,
        status: DeliveryStatus.DELIVERED,
        attempts: 1,
        lastAttempt: new Date(),
        deliveredAt: new Date(),
        externalId: trackingId
      });
    } catch (error) {
      const deliveryTime = Date.now() - startTime;
      this.updateStats(false, deliveryTime);

      return new DeliveryTracking({
        notificationId: message.id,
        recipientId: recipient.id,
        channel: NotificationChannel.IN_APP,
        status: DeliveryStatus.FAILED,
        attempts: 1,
        lastAttempt: new Date(),
        errorMessage: error.message
      });
    }
  }

  /**
   * Get in-app notifications for a user
   */
  getUserNotifications(userId) {
    return this.storage.get(userId) || [];
  }

  /**
   * Mark notification as read
   */
  markAsRead(userId, notificationId) {
    const notifications = this.storage.get(userId) || [];
    const notification = notifications.find(n => n.id === notificationId);
    if (notification) {
      notification.read = true;
      return true;
    }
    return false;
  }

  /**
   * Health check for in-app provider
   */
  async healthCheck() {
    return this.config.enabled;
  }
}

/**
 * Provider factory
 */
class ProviderFactory {
  /**
   * Create provider based on channel
   */
  static createProvider(channel, config) {
    switch (channel) {
      case NotificationChannel.EMAIL:
        return new EmailProvider(config);
      case NotificationChannel.SMS:
        return new SMSProvider(config);
      case NotificationChannel.PUSH:
        return new PushProvider(config);
      case NotificationChannel.IN_APP:
        return new InAppProvider(config);
      default:
        throw new ProviderError(`Unsupported channel: ${channel}`, 'UNSUPPORTED_CHANNEL');
    }
  }

  /**
   * Create default provider configurations
   */
  static getDefaultConfigs() {
    return {
      [NotificationChannel.EMAIL]: {
        providerType: NotificationChannel.EMAIL,
        config: {},
        enabled: false
      },
      [NotificationChannel.SMS]: {
        providerType: NotificationChannel.SMS,
        config: {},
        enabled: false
      },
      [NotificationChannel.PUSH]: {
        providerType: NotificationChannel.PUSH,
        config: {},
        enabled: false
      },
      [NotificationChannel.IN_APP]: {
        providerType: NotificationChannel.IN_APP,
        config: {},
        enabled: true
      }
    };
  }
}

module.exports = {
  NotificationProvider,
  EmailProvider,
  SMSProvider,
  PushProvider,
  InAppProvider,
  ProviderFactory,
  ProviderError
};
