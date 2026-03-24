import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { ConfigModule } from '@nestjs/config';

import { RiskAssessorService } from './assessment/risk-assessor.service';
import { RealTimeMonitorService } from './monitoring/real-time-monitor.service';
import { HedgingStrategyService } from './hedging/hedging-strategy.service';
import { VarCalculatorService } from './calculations/var-calculator.service';
import { StressTestService } from './testing/stress-test.service';
import { RiskManagementService } from './risk-management.service';
import { RiskController } from './risk.controller';
import { RiskData } from './entities/risk-data.entity';

@Module({
  imports: [
    TypeOrmModule.forFeature([RiskData]),
    ConfigModule,
  ],
  providers: [
    RiskManagementService,
    RiskAssessorService,
    RealTimeMonitorService,
    HedgingStrategyService,
    VarCalculatorService,
    StressTestService,
  ],
  controllers: [RiskController],
  exports: [
    RiskManagementService,
    RiskAssessorService,
    RealTimeMonitorService,
    HedgingStrategyService,
    VarCalculatorService,
    StressTestService,
  ],
})
export class RiskManagementModule {}
