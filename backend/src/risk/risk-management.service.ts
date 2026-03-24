import { Injectable } from '@nestjs/common';

import { RiskAssessorService } from './assessment/risk-assessor.service';
import { RealTimeMonitorService } from './monitoring/real-time-monitor.service';
import { HedgingStrategyService } from './hedging/hedging-strategy.service';
import { VarCalculatorService } from './calculations/var-calculator.service';
import { StressTestService } from './testing/stress-test.service';

@Injectable()
export class RiskManagementService {
  constructor(
    private readonly riskAssessorService: RiskAssessorService,
    private readonly realTimeMonitorService: RealTimeMonitorService,
    private readonly hedgingStrategyService: HedgingStrategyService,
    private readonly varCalculatorService: VarCalculatorService,
    private readonly stressTestService: StressTestService,
  ) {}

  async assessRisk(assessmentDto: any): Promise<any> {
    return this.riskAssessorService.assessRisk(assessmentDto);
  }

  async getRiskAlerts(portfolioId: string): Promise<any[]> {
    return this.realTimeMonitorService.getRecentAlerts(portfolioId);
  }

  async getRiskMetrics(portfolioId: string): Promise<any> {
    return this.realTimeMonitorService.getRealTimeMetrics(portfolioId);
  }

  async runStressTests(portfolioData: any): Promise<any> {
    const riskMetrics = await this.assessRisk({ portfolio: portfolioData });
    return this.stressTestService.runStressTests(portfolioData, riskMetrics.metrics);
  }

  async calculateVar(portfolioId: string, confidence: number, horizon: number): Promise<any> {
    // Get historical returns for the portfolio
    const portfolioReturns = await this.getPortfolioReturns(portfolioId);
    return this.varCalculatorService.calculateVar(portfolioReturns, confidence, horizon);
  }

  async generateHedgingStrategies(portfolioData: any): Promise<any> {
    const riskMetrics = await this.assessRisk({ portfolio: portfolioData });
    return this.hedgingStrategyService.generateHedgingStrategies(portfolioData, riskMetrics.metrics);
  }

  private async getPortfolioReturns(portfolioId: string): Promise<number[]> {
    // Simulate historical returns - in production, this would fetch from database
    const returns: number[] = [];
    const baseReturn = 0.0008; // Daily return ~20% annual
    const volatility = 0.02; // 2% daily volatility
    
    for (let i = 0; i < 252; i++) { // 252 trading days
      const randomShock = this.normalRandom() * volatility;
      returns.push(baseReturn + randomShock);
    }
    
    return returns;
  }

  private normalRandom(): number {
    // Box-Muller transform for normal distribution
    const u1 = Math.random();
    const u2 = Math.random();
    return Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
  }
}
