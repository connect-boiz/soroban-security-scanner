//! Delivery tracking system for notifications

const {
  DeliveryStatus,
  NotificationChannel,
  DeliveryTracking,
  ChannelStats,
  DeliveryStats,
  HourlyStats,
  ProviderStats
} = require('./types');

class TrackingError extends Error {
  constructor(message, code) {
    super(message);
    this.name = 'TrackingError';
    this.code = code;
  }
}

/**
 * Delivery metrics collector
 */
class DeliveryMetrics {
  constructor() {
    this.totalSent = new Map();
    this.totalDelivered = new Map();
    this.totalFailed = new Map();
    this.deliveryTimes = new Map();
    this.lastUpdated = new Date();
  }

  /**
   * Record a delivery
   */
  recordDelivery(tracking) {
    const channel = tracking.channel;

    // Update counters
    this.totalSent.set(channel, (this.totalSent.get(channel) || 0) + 1);

    switch (tracking.status) {
      case DeliveryStatus.DELIVERED:
        this.totalDelivered.set(channel, (this.totalDelivered.get(channel) || 0) + 1);

        // Calculate delivery time if we have timestamps
        if (tracking.deliveredAt && tracking.lastAttempt) {
          const deliveryTimeMs = tracking.deliveredAt - tracking.lastAttempt;
          const times = this.deliveryTimes.get(channel) || [];
          times.push(deliveryTimeMs);
          this.deliveryTimes.set(channel, times);
        }
        break;
      case DeliveryStatus.FAILED:
        this.totalFailed.set(channel, (this.totalFailed.get(channel) || 0) + 1);
        break;
    }

    this.lastUpdated = new Date();
  }

  /**
   * Get success rate for a channel
   */
  successRate(channel) {
    const sent = this.totalSent.get(channel) || 0;
    const delivered = this.totalDelivered.get(channel) || 0;

    if (sent === 0) {
      return 0;
    }

    return delivered / sent;
  }

  /**
   * Get average delivery time for a channel
   */
  averageDeliveryTime(channel) {
    const times = this.deliveryTimes.get(channel) || [];

    if (times.length === 0) {
      return 0;
    }

    const sum = times.reduce((acc, time) => acc + time, 0);
    return Math.round(sum / times.length);
  }

  /**
   * Get total statistics
   */
  getTotalStats() {
    const stats = {};

    for (const channel of Object.values(NotificationChannel)) {
      stats[channel] = new ChannelStats({
        totalSent: this.totalSent.get(channel) || 0,
        totalDelivered: this.totalDelivered.get(channel) || 0,
        totalFailed: this.totalFailed.get(channel) || 0,
        successRate: this.successRate(channel),
        averageDeliveryTimeMs: this.averageDeliveryTime(channel)
      });
    }

    return stats;
  }
}

/**
 * Tracking storage (in-memory implementation)
 */
class TrackingStorage {
  constructor() {
    this.trackings = new Map(); // key: "notificationId:recipientId:channel"
  }

  /**
   * Generate storage key
   */
  generateKey(notificationId, recipientId, channel) {
    return `${notificationId}:${recipientId}:${channel}`;
  }

  /**
   * Store tracking record
   */
  storeTracking(tracking) {
    const key = this.generateKey(tracking.notificationId, tracking.recipientId, tracking.channel);
    this.trackings.set(key, tracking);
    return tracking;
  }

  /**
   * Get tracking record
   */
  getTracking(notificationId, recipientId, channel) {
    const key = this.generateKey(notificationId, recipientId, channel);
    return this.trackings.get(key) || null;
  }

  /**
   * Update tracking record
   */
  updateTracking(tracking) {
    const key = this.generateKey(tracking.notificationId, tracking.recipientId, tracking.channel);
    this.trackings.set(key, tracking);
    return tracking;
  }

  /**
   * Get all tracking for a notification
   */
  getNotificationTracking(notificationId) {
    const results = [];
    for (const [key, tracking] of this.trackings) {
      if (key.startsWith(`${notificationId}:`)) {
        results.push(tracking);
      }
    }
    return results;
  }

  /**
   * Get tracking for a recipient
   */
  getRecipientTracking(recipientId) {
    const results = [];
    for (const [key, tracking] of this.trackings) {
      if (key.includes(`:${recipientId}:`)) {
        results.push(tracking);
      }
    }
    return results;
  }

  /**
   * Get tracking in time period
   */
  getTrackingInPeriod(startTime, endTime) {
    const results = [];
    for (const tracking of this.trackings.values()) {
      if (tracking.lastAttempt >= startTime && tracking.lastAttempt <= endTime) {
        results.push(tracking);
      }
    }
    return results;
  }

  /**
   * Get failed trackings
   */
  getFailedTrackings() {
    const results = [];
    for (const tracking of this.trackings.values()) {
      if (tracking.status === DeliveryStatus.FAILED) {
        results.push(tracking);
      }
    }
    return results;
  }

  /**
   * Clean up old records
   */
  cleanupOldRecords(cutoffDate) {
    let removed = 0;
    for (const [key, tracking] of this.trackings) {
      if (tracking.lastAttempt < cutoffDate) {
        this.trackings.delete(key);
        removed++;
      }
    }
    return removed;
  }
}

/**
 * Delivery tracking manager
 */
class DeliveryTracker {
  constructor() {
    this.storage = new TrackingStorage();
    this.metrics = new DeliveryMetrics();
  }

  /**
   * Record a delivery attempt
   */
  async recordDelivery(tracking) {
    if (!(tracking instanceof DeliveryTracking)) {
      tracking = new DeliveryTracking(tracking);
    }

    // Store tracking record
    this.storage.storeTracking(tracking);

    // Update metrics
    this.metrics.recordDelivery(tracking);

    return tracking;
  }

  /**
   * Update delivery status
   */
  async updateStatus(notificationId, recipientId, channel, status, errorMessage = null) {
    const tracking = this.storage.getTracking(notificationId, recipientId, channel);
    
    if (!tracking) {
      throw new TrackingError('Tracking record not found', 'NOT_FOUND');
    }

    const updatedTracking = new DeliveryTracking({
      ...tracking,
      status,
      lastAttempt: new Date(),
      errorMessage,
      deliveredAt: status === DeliveryStatus.DELIVERED ? new Date() : tracking.deliveredAt
    });

    this.storage.updateTracking(updatedTracking);
    this.metrics.recordDelivery(updatedTracking);

    return updatedTracking;
  }

  /**
   * Get tracking information for a notification
   */
  async getTracking(notificationId, recipientId, channel) {
    return this.storage.getTracking(notificationId, recipientId, channel);
  }

  /**
   * Get all tracking for a notification
   */
  async getNotificationTracking(notificationId) {
    return this.storage.getNotificationTracking(notificationId);
  }

  /**
   * Get tracking for a recipient
   */
  async getRecipientTracking(recipientId) {
    return this.storage.getRecipientTracking(recipientId);
  }

  /**
   * Get delivery metrics
   */
  getMetrics() {
    return this.metrics;
  }

  /**
   * Get delivery statistics for a time period
   */
  async getDeliveryStats(startTime, endTime) {
    const trackings = this.storage.getTrackingInPeriod(startTime, endTime);
    
    const stats = new DeliveryStats({
      startTime,
      endTime,
      totalNotifications: trackings.length
    });

    // Group by channel and calculate statistics
    const channelGroups = new Map();
    for (const tracking of trackings) {
      const channel = tracking.channel;
      if (!channelGroups.has(channel)) {
        channelGroups.set(channel, []);
      }
      channelGroups.get(channel).push(tracking);
    }

    // Calculate channel statistics
    for (const [channel, channelTrackings] of channelGroups) {
      const sent = channelTrackings.length;
      const delivered = channelTrackings.filter(t => t.status === DeliveryStatus.DELIVERED).length;
      const failed = channelTrackings.filter(t => t.status === DeliveryStatus.FAILED).length;
      const successRate = sent > 0 ? delivered / sent : 0;

      // Calculate average delivery time
      const deliveryTimes = channelTrackings
        .filter(t => t.deliveredAt && t.lastAttempt)
        .map(t => t.deliveredAt - t.lastAttempt);
      const avgDeliveryTime = deliveryTimes.length > 0 
        ? Math.round(deliveryTimes.reduce((a, b) => a + b, 0) / deliveryTimes.length)
        : 0;

      stats.channelStats[channel] = new ChannelStats({
        totalSent: sent,
        totalDelivered: delivered,
        totalFailed: failed,
        successRate,
        averageDeliveryTimeMs: avgDeliveryTime
      });
    }

    return stats;
  }

  /**
   * Retry failed deliveries
   */
  async retryFailedDeliveries() {
    const failedTrackings = this.storage.getFailedTrackings();
    const retried = [];

    for (const tracking of failedTrackings) {
      if (tracking.attempts < 3) { // Max 3 attempts
        const updatedTracking = new DeliveryTracking({
          ...tracking,
          attempts: tracking.attempts + 1,
          status: DeliveryStatus.RETRYING,
          lastAttempt: new Date()
        });

        this.storage.updateTracking(updatedTracking);
        retried.push(updatedTracking);
      }
    }

    return retried;
  }

  /**
   * Clean up old tracking records
   */
  async cleanupOldRecords(olderThanDays = 30) {
    const cutoffDate = new Date();
    cutoffDate.setDate(cutoffDate.getDate() - olderThanDays);
    return this.storage.cleanupOldRecords(cutoffDate);
  }

  /**
   * Get provider statistics
   */
  getProviderStats() {
    const stats = {};
    const totalStats = this.metrics.getTotalStats();

    for (const [channel, channelStats] of Object.entries(totalStats)) {
      stats[channel] = new ProviderStats({
        channel,
        totalSent: channelStats.totalSent,
        totalFailed: channelStats.totalFailed,
        averageDeliveryTimeMs: channelStats.averageDeliveryTimeMs,
        lastSuccess: null, // Would be tracked in real implementation
        lastFailure: null  // Would be tracked in real implementation
      });
    }

    return stats;
  }
}

module.exports = {
  DeliveryTracker,
  DeliveryMetrics,
  TrackingStorage,
  TrackingError
};
