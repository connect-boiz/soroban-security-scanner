import { Injectable, Logger } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { InjectQueue } from '@nestjs/bullmq';
import { Queue } from 'bullmq';
import { ConfigService } from '@nestjs/config';
import * as crypto from 'crypto';
import { Webhook } from './entities/webhook.entity';
import { NotificationQueue } from './entities/notification-queue.entity';
import { CreateWebhookDto } from './dto/create-webhook.dto';
import { TestWebhookDto } from './dto/test-webhook.dto';
import { Scan } from '../scan/entities/scan.entity';

@Injectable()
export class WebhookService {
  private readonly logger = new Logger(WebhookService.name);

  constructor(
    @InjectRepository(Webhook)
    private readonly webhookRepository: Repository<Webhook>,
    @InjectRepository(NotificationQueue)
    private readonly notificationQueueRepository: Repository<NotificationQueue>,
    @InjectQueue('notification-queue')
    private readonly notificationQueue: Queue,
    private readonly configService: ConfigService,
  ) {}

  async createWebhook(createWebhookDto: CreateWebhookDto, userId: string): Promise<Webhook> {
    const webhook = this.webhookRepository.create({
      ...createWebhookDto,
      userId,
      config: {
        ...createWebhookDto.config,
        secret: createWebhookDto.config?.secret || crypto.randomBytes(32).toString('hex'),
      },
    });

    return await this.webhookRepository.save(webhook);
  }

  async getWebhooks(userId: string): Promise<Webhook[]> {
    return await this.webhookRepository.find({
      where: { userId },
      order: { createdAt: 'DESC' },
    });
  }

  async getWebhook(id: string, userId: string): Promise<Webhook> {
    const webhook = await this.webhookRepository.findOne({
      where: { id, userId },
    });

    if (!webhook) {
      throw new Error('Webhook not found');
    }

    return webhook;
  }

  async updateWebhook(id: string, userId: string, updateData: Partial<CreateWebhookDto>): Promise<Webhook> {
    const webhook = await this.getWebhook(id, userId);
    
    Object.assign(webhook, updateData);
    
    return await this.webhookRepository.save(webhook);
  }

  async deleteWebhook(id: string, userId: string): Promise<void> {
    const webhook = await this.getWebhook(id, userId);
    await this.webhookRepository.remove(webhook);
  }

  async testWebhook(id: string, userId: string, testDto?: TestWebhookDto): Promise<void> {
    const webhook = await this.getWebhook(id, userId);
    
    const testPayload = this.formatTestPayload(webhook, testDto);
    
    await this.notificationQueue.add('send-webhook', {
      webhookId: webhook.id,
      payload: testPayload,
      isTest: true,
    }, {
      attempts: 3,
      backoff: {
        type: 'exponential',
        delay: 2000,
      },
    });
  }

  async sendScanNotification(scan: Scan, vulnerabilities: any[]): Promise<void> {
    if (!vulnerabilities || vulnerabilities.length === 0) {
      return;
    }

    const webhooks = await this.webhookRepository.find({
      where: { userId: scan.userId, enabled: true },
    });

    for (const webhook of webhooks) {
      const shouldSend = this.shouldSendNotification(webhook, vulnerabilities);
      if (!shouldSend) {
        continue;
      }

      const deduplicationKey = this.generateDeduplicationKey(scan, webhook);
      
      // Check for deduplication
      const existingNotification = await this.notificationQueueRepository.findOne({
        where: {
          deduplicationKey,
          status: 'sent',
          createdAt: {
            // Check for notifications sent in the last 24 hours
            $gte: new Date(Date.now() - 24 * 60 * 60 * 1000),
          },
        },
      });

      if (existingNotification) {
        this.logger.log(`Skipping duplicate notification for webhook ${webhook.id}`);
        continue;
      }

      const payload = this.formatScanPayload(scan, vulnerabilities, webhook);
      
      // Queue notification
      await this.notificationQueue.add('send-webhook', {
        webhookId: webhook.id,
        scanId: scan.id,
        userId: scan.userId,
        payload,
        deduplicationKey,
      }, {
        attempts: 3,
        backoff: {
          type: 'exponential',
          delay: 2000,
        },
        delay: 1000, // Small delay to ensure scan is fully completed
      });
    }
  }

  private shouldSendNotification(webhook: Webhook, vulnerabilities: any[]): boolean {
    if (webhook.severityFilter === 'all') {
      return true;
    }

    return vulnerabilities.some(vuln => {
      switch (vuln.severity) {
        case 'critical':
          return ['critical', 'high', 'medium', 'low'].includes(webhook.severityFilter);
        case 'high':
          return ['high', 'medium', 'low'].includes(webhook.severityFilter);
        case 'medium':
          return ['medium', 'low'].includes(webhook.severityFilter);
        case 'low':
          return webhook.severityFilter === 'low';
        default:
          return false;
      }
    });
  }

  private generateDeduplicationKey(scan: Scan, webhook: Webhook): string {
    // Create a deduplication key based on scan content hash and webhook ID
    const contentHash = crypto.createHash('sha256')
      .update(scan.code + JSON.stringify(scan.options))
      .digest('hex')
      .substring(0, 16);
    
    return `${webhook.id}_${contentHash}`;
  }

  private formatScanPayload(scan: Scan, vulnerabilities: any[], webhook: Webhook): any {
    const baseUrl = this.configService.get<string>('BASE_URL', 'http://localhost:3000');
    const reportUrl = `${baseUrl}/scans/${scan.id}`;
    
    const criticalCount = vulnerabilities.filter(v => v.severity === 'critical').length;
    const highCount = vulnerabilities.filter(v => v.severity === 'high').length;
    const mediumCount = vulnerabilities.filter(v => v.severity === 'medium').length;
    const lowCount = vulnerabilities.filter(v => v.severity === 'low').length;

    const basePayload = {
      scan_id: scan.id,
      timestamp: scan.updatedAt,
      summary: {
        total_vulnerabilities: vulnerabilities.length,
        critical: criticalCount,
        high: highCount,
        medium: mediumCount,
        low: lowCount,
        risk_score: scan.metrics?.riskScore || 0,
      },
      report_url: reportUrl,
      vulnerabilities: vulnerabilities.slice(0, 5).map(v => ({
        type: v.type,
        severity: v.severity,
        description: v.description,
        location: v.location,
      })),
    };

    if (webhook.type === 'slack') {
      return {
        text: `🚨 Security Scan Alert - ${vulnerabilities.length} vulnerabilities detected`,
        attachments: [{
          color: criticalCount > 0 ? 'danger' : highCount > 0 ? 'warning' : 'good',
          fields: [
            { title: 'Scan ID', value: scan.id, short: true },
            { title: 'Risk Score', value: `${scan.metrics?.riskScore || 0}`, short: true },
            { title: 'Critical', value: criticalCount.toString(), short: true },
            { title: 'High', value: highCount.toString(), short: true },
            { title: 'Medium', value: mediumCount.toString(), short: true },
            { title: 'Low', value: lowCount.toString(), short: true },
          ],
          actions: [{
            type: 'button',
            text: 'View Full Report',
            url: reportUrl,
          }],
        }],
      };
    } else if (webhook.type === 'discord') {
      return {
        content: `🚨 Security Scan Alert - ${vulnerabilities.length} vulnerabilities detected`,
        embeds: [{
          title: 'Security Scan Results',
          color: criticalCount > 0 ? 0xFF0000 : highCount > 0 ? 0xFFA500 : 0x00FF00,
          fields: [
            { name: 'Scan ID', value: scan.id, inline: true },
            { name: 'Risk Score', value: `${scan.metrics?.riskScore || 0}`, inline: true },
            { name: 'Critical', value: criticalCount.toString(), inline: true },
            { name: 'High', value: highCount.toString(), inline: true },
            { name: 'Medium', value: mediumCount.toString(), inline: true },
            { name: 'Low', value: lowCount.toString(), inline: true },
          ],
          url: reportUrl,
        }],
      };
    }

    return basePayload;
  }

  private formatTestPayload(webhook: Webhook, testDto?: TestWebhookDto): any {
    const message = testDto?.message || 'Test notification from Soroban Security Scanner';
    
    if (webhook.type === 'slack') {
      return {
        text: '✅ Webhook Test Successful',
        attachments: [{
          color: 'good',
          fields: [
            { title: 'Webhook Name', value: webhook.name, short: true },
            { title: 'Message', value: message, short: false },
          ],
        }],
      };
    } else if (webhook.type === 'discord') {
      return {
        content: '✅ Webhook Test Successful',
        embeds: [{
          title: 'Webhook Test',
          color: 0x00FF00,
          fields: [
            { name: 'Webhook Name', value: webhook.name, inline: true },
            { name: 'Message', value: message, inline: false },
          ],
        }],
      };
    }

    return {
      type: 'test',
      message,
      webhook_name: webhook.name,
      timestamp: new Date().toISOString(),
    };
  }

  async getNotificationStats(userId: string): Promise<any> {
    const webhooks = await this.getWebhooks(userId);
    
    const stats = {
      totalWebhooks: webhooks.length,
      activeWebhooks: webhooks.filter(w => w.enabled).length,
      totalSent: webhooks.reduce((sum, w) => sum + w.successCount, 0),
      totalFailed: webhooks.reduce((sum, w) => sum + w.failureCount, 0),
      recentActivity: await this.getRecentActivity(userId),
    };

    return stats;
  }

  private async getRecentActivity(userId: string): Promise<any[]> {
    const recent = await this.notificationQueueRepository.find({
      where: { userId },
      order: { createdAt: 'DESC' },
      take: 10,
    });

    return recent.map(notification => ({
      id: notification.id,
      webhookId: notification.webhookId,
      status: notification.status,
      createdAt: notification.createdAt,
      sentAt: notification.sentAt,
    }));
  }
}
