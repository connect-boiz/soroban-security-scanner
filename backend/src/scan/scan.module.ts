import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { ThrottlerGuard } from '@nestjs/throttler';
import { APP_GUARD } from '@nestjs/core';

import { ScanController } from './scan.controller';
import { ScanService } from './scan.service';
import { Scan } from './entities/scan.entity';
import { Vulnerability } from './entities/vulnerability.entity';

@Module({
  imports: [TypeOrmModule.forFeature([Scan, Vulnerability])],
  controllers: [ScanController],
  providers: [
    ScanService,
    {
      provide: APP_GUARD,
      useClass: ThrottlerGuard,
    },
  ],
  exports: [ScanService],
})
export class ScanModule {}
