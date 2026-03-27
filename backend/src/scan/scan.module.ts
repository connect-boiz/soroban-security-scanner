import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { ThrottlerGuard } from '@nestjs/throttler';
import { APP_GUARD } from '@nestjs/core';

import { BullModule } from '@nestjs/bullmq';
import { ScanController } from './scan.controller';
import { ScanService } from './scan.service';
import { ScanProgressGateway } from './scan-progress.gateway';
import { ScanProcessor } from './scan.processor';
import { GarbageCollectorService } from './garbage-collector.service';
import { Scan } from './entities/scan.entity';
import { Vulnerability } from './entities/vulnerability.entity';
import { ConfigModule } from '@nestjs/config';
import { WebhookModule } from '../webhook/webhook.module';

@Module({
  imports: [
    TypeOrmModule.forFeature([Scan, Vulnerability]),
    BullModule.registerQueue({
      name: 'scan-queue',
      defaultJobOptions: {
        attempts: 3,
        backoff: {
          type: 'exponential',
          delay: 5000,
        },
      },
    }),
    ConfigModule,
    WebhookModule,
  ],
  controllers: [ScanController],
  providers: [
    ScanService,
    ScanProgressGateway,
    ScanProcessor,
    GarbageCollectorService,
    {
      provide: APP_GUARD,
      useClass: ThrottlerGuard,
    },
  ],
  exports: [ScanService],
})
export class ScanModule {}
