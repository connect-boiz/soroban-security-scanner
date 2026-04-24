import { Module } from '@nestjs/common';
import { ConfigModule, ConfigService } from '@nestjs/config';
import { TypeOrmModule } from '@nestjs/typeorm';
import { RedisModule } from '@nestjs/redis';
import { ThrottlerModule } from '@nestjs/throttler';
import { JwtModule } from '@nestjs/jwt';
import { BullModule } from '@nestjs/bullmq';
import { ScheduleModule } from '@nestjs/schedule';

import { HealthModule } from './health/health.module';
import { ScanModule } from './scan/scan.module';
import { UserModule } from './user/user.module';
import { AuthModule } from './auth/auth.module';
import { DatabaseModule } from './database/database.module';
import { RiskManagementModule } from './risk/risk-management.module';
import { ApiKeyModule } from './api-key/api-key.module';
import { WebhookModule } from './webhook/webhook.module';
import { EscrowModule } from './escrow/escrow.module';

@Module({
  imports: [
    // Configuration
    ConfigModule.forRoot({
      isGlobal: true,
      envFilePath: ['.env.local', '.env'],
    }),

    // Schedule
    ScheduleModule.forRoot(),

    // Database
    DatabaseModule,

    // Redis
    RedisModule.forRootAsync({
      imports: [ConfigModule],
      useFactory: async (configService: ConfigService) => ({
        config: {
          url: configService.get<string>('REDIS_URL', 'redis://localhost:6379'),
          keyPrefix: configService.get<string>('REDIS_KEY_PREFIX', 'soroban_scanner:'),
        },
      }),
      inject: [ConfigService],
    }),

    // BullMQ Queueing
    BullModule.forRootAsync({
      imports: [ConfigModule],
      useFactory: async (configService: ConfigService) => ({
        connection: {
          host: configService.get<string>('REDIS_HOST', 'localhost'),
          port: configService.get<number>('REDIS_PORT', 6379),
        },
      }),
      inject: [ConfigService],
    }),

    // Rate limiting
    ThrottlerModule.forRoot([
      {
        name: 'default',
        ttl: 60000, // 1 minute
        limit: 100, // 100 requests per minute
      },
      {
        name: 'vulnerability-reporting',
        ttl: 60000, // 1 minute
        limit: 10, // 10 vulnerability reports per minute
      },
      {
        name: 'escrow-creation',
        ttl: 60000, // 1 minute
        limit: 5, // 5 escrow creations per minute
      },
      {
        name: 'batch-operations',
        ttl: 300000, // 5 minutes
        limit: 3, // 3 batch operations per 5 minutes
      },
      {
        name: 'scan-operations',
        ttl: 60000, // 1 minute
        limit: 20, // 20 scans per minute
      },
    ]),

    // JWT
    JwtModule.registerAsync({
      imports: [ConfigModule],
      useFactory: async (configService: ConfigService) => ({
        secret: configService.get<string>('JWT_SECRET'),
        signOptions: {
          expiresIn: configService.get<string>('JWT_EXPIRES_IN', '7d'),
        },
      }),
      inject: [ConfigService],
    }),

    // Feature modules
    HealthModule,
    ScanModule,
    UserModule,
    AuthModule,
    RiskManagementModule,
    ApiKeyModule,
    WebhookModule,
    EscrowModule,
  ],
  controllers: [],
  providers: [],
})
export class AppModule {}
