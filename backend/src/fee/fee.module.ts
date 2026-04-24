import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { ConfigModule } from '@nestjs/config';
import { FeeController } from './fee.controller';
import { FeeService } from './services/fee.service';
import { FeeCalculatorService } from './services/fee-calculator.service';
import { Fee } from './entities/fee.entity';
import { UserBalance } from './entities/user-balance.entity';

@Module({
  imports: [
    TypeOrmModule.forFeature([Fee, UserBalance]),
    ConfigModule,
  ],
  controllers: [FeeController],
  providers: [FeeService, FeeCalculatorService],
  exports: [FeeService, FeeCalculatorService],
})
export class FeeModule {}
