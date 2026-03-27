import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { BullModule } from '@nestjs/bullmq';
import { WebhookService } from './webhook.service';
import { WebhookController } from './webhook.controller';
import { NotificationProcessor } from './notification.processor';
import { Webhook } from './entities/webhook.entity';
import { NotificationQueue } from './entities/notification-queue.entity';

@Module({
  imports: [
    TypeOrmModule.forFeature([Webhook, NotificationQueue]),
    BullModule.registerQueue({
      name: 'notification-queue',
      defaultJobOptions: {
        attempts: 3,
        backoff: {
          type: 'exponential',
          delay: 5000,
        },
        removeOnComplete: 100,
        removeOnFail: 50,
      },
    }),
  ],
  controllers: [WebhookController],
  providers: [WebhookService, NotificationProcessor],
  exports: [WebhookService],
})
export class WebhookModule {}
