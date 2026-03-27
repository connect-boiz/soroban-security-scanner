import { Processor, Process } from '@nestjs/bullmq';
import { Job } from 'bullmq';
import { Logger } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import axios from 'axios';
import * as crypto from 'crypto';
import { Webhook } from './entities/webhook.entity';
import { NotificationQueue } from './entities/notification-queue.entity';

interface WebhookJobData {
  webhookId: string;
  scanId?: string;
  userId?: string;
  payload: any;
  isTest?: boolean;
  deduplicationKey?: string;
}

@Processor('notification-queue')
export class NotificationProcessor {
  private readonly logger = new Logger(NotificationProcessor.name);

  constructor(
    @InjectRepository(Webhook)
    private readonly webhookRepository: Repository<Webhook>,
    @InjectRepository(NotificationQueue)
    private readonly notificationQueueRepository: Repository<NotificationQueue>,
  ) {}

  @Process('send-webhook')
  async handleSendWebhook(job: Job<WebhookJobData>) {
    const { webhookId, scanId, userId, payload, isTest, deduplicationKey } = job.data;

    try {
      // Get webhook configuration
      const webhook = await this.webhookRepository.findOne({
        where: { id: webhookId },
      });

      if (!webhook) {
        this.logger.error(`Webhook ${webhookId} not found`);
        return;
      }

      if (!webhook.enabled && !isTest) {
        this.logger.log(`Webhook ${webhookId} is disabled, skipping`);
        return;
      }

      // Create notification queue record
      let notificationRecord: NotificationQueue;
      if (!isTest) {
        notificationRecord = this.notificationQueueRepository.create({
          webhookId,
          scanId,
          userId,
          payload: JSON.stringify(payload),
          deduplicationKey,
          status: 'pending',
        });
        await this.notificationQueueRepository.save(notificationRecord);
      }

      // Prepare webhook request
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'User-Agent': 'Soroban-Security-Scanner/1.0',
      };

      // Add signature if secret is configured
      if (webhook.config?.secret) {
        const signature = this.generateSignature(JSON.stringify(payload), webhook.config.secret);
        headers['X-Signature'] = signature;
      }

      // Send webhook
      const response = await axios.post(webhook.url, payload, {
        headers,
        timeout: 10000,
        maxRedirects: 3,
      });

      // Update webhook stats
      webhook.successCount += 1;
      webhook.lastSent = {
        timestamp: new Date(),
        scanId: scanId || 'test',
        status: 'sent',
      };
      await this.webhookRepository.save(webhook);

      // Update notification record
      if (!isTest && notificationRecord) {
        notificationRecord.status = 'sent';
        notificationRecord.sentAt = new Date();
        await this.notificationQueueRepository.save(notificationRecord);
      }

      this.logger.log(`Webhook ${webhookId} sent successfully to ${webhook.url}`);

    } catch (error) {
      this.logger.error(`Failed to send webhook ${webhookId}:`, error.message);

      // Update webhook stats
      const webhook = await this.webhookRepository.findOne({
        where: { id: webhookId },
      });
      
      if (webhook) {
        webhook.failureCount += 1;
        webhook.lastError = error.message;
        await this.webhookRepository.save(webhook);
      }

      // Update notification record
      if (!isTest && scanId && userId) {
        const notificationRecord = await this.notificationQueueRepository.findOne({
          where: { webhookId, scanId, userId },
        });
        
        if (notificationRecord) {
          notificationRecord.status = 'failed';
          notificationRecord.errorMessage = error.message;
          notificationRecord.attempts += 1;
          notificationRecord.nextAttemptAt = new Date(Date.now() + Math.pow(2, notificationRecord.attempts) * 5000);
          await this.notificationQueueRepository.save(notificationRecord);
        }
      }

      // Re-queue for retry if attempts < 3
      if (job.attemptsMade < 3) {
        throw error; // Let BullMQ handle retry
      }
    }
  }

  private generateSignature(payload: string, secret: string): string {
    return 'sha256=' + crypto
      .createHmac('sha256', secret)
      .update(payload)
      .digest('hex');
  }

  @Process('cleanup-notifications')
  async handleCleanupNotifications(job: Job) {
    const daysOld = job.data.daysOld || 30;
    const cutoffDate = new Date();
    cutoffDate.setDate(cutoffDate.getDate() - daysOld);

    try {
      const result = await this.notificationQueueRepository.delete({
        createdAt: {
          $lt: cutoffDate,
        },
        status: {
          $in: ['sent', 'failed'],
        },
      });

      this.logger.log(`Cleaned up ${result.affected} old notification records`);
    } catch (error) {
      this.logger.error('Failed to cleanup notifications:', error.message);
    }
  }
}
