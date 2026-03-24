import { Injectable, Logger } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { Redis } from '@nestjs/redis';

import { StressTestResultDto } from '../dto/risk-report.dto';

@Injectable()
export class StressTestService {
  private readonly logger = new Logger(StressTestService.name);
  private readonly STRESS_SCENARIOS = this.initializeStressScenarios();

  constructor(
    private readonly configService: ConfigService,
    private readonly redis: Redis,
  ) {}

  async runStressTests(portfolio: any, riskMetrics: any): Promise<{
    results: StressTestResultDto[];
    summary: {
      worstCaseLoss: number;
      worstCaseScenario: string;
      averageLoss: number;
      scenariosPassed: number;
      scenariosFailed: number;
      riskResilience: number;
    };
  }> {
    try {
      const results: StressTestResultDto[] = [];
      
      // Run all stress scenarios
      for (const scenario of this.STRESS_SCENARIOS) {
        const result = await this.runSingleStressTest(portfolio, scenario);
        results.push(result);
      }
      
      // Calculate summary statistics
      const summary = this.calculateStressTestSummary(results);
      
      // Cache results
      await this.cacheStressTestResults(portfolio.id, results, summary);
      
      this.logger.log(`Completed ${results.length} stress scenarios for portfolio ${portfolio.id}`);
      
      return {
        results,
        summary,
      };
    } catch (error) {
      this.logger.error('Stress testing failed:', error);
      throw new Error('Stress testing failed');
    }
  }

  async runCustomStressTest(
    portfolio: any,
    customScenario: any
  ): Promise<StressTestResultDto> {
    try {
      // Validate custom scenario
      this.validateCustomScenario(customScenario);
      
      // Run the custom stress test
      const result = await this.runSingleStressTest(portfolio, customScenario);
      
      // Store custom result
      await this.storeCustomStressTestResult(portfolio.id, customScenario, result);
      
      return result;
    } catch (error) {
      this.logger.error('Custom stress test failed:', error);
      throw new Error('Custom stress test failed');
    }
  }

  async runMonteCarloStressTest(
    portfolio: any,
    numSimulations: number = 10000
  ): Promise<{
    distribution: {
      mean: number;
      median: number;
      stdDev: number;
      percentiles: {
        p1: number;
        p5: number;
        p10: number;
        p25: number;
        p75: number;
        p90: number;
        p95: number;
        p99: number;
      };
    };
    tailRisk: {
      expectedShortfall: number;
      maximumLoss: number;
      tailProbability: number;
    };
    simulations: number[];
  }> {
    try {
      const simulations = [];
      
      // Run Monte Carlo simulations
      for (let i = 0; i < numSimulations; i++) {
        const simulatedScenario = this.generateRandomScenario();
        const result = await this.runSingleStressTest(portfolio, simulatedScenario);
        simulations.push(result.portfolioLoss);
      }
      
      // Calculate distribution statistics
      const distribution = this.calculateDistributionStatistics(simulations);
      
      // Calculate tail risk metrics
      const tailRisk = this.calculateTailRisk(simulations);
      
      this.logger.log(`Completed ${numSimulations} Monte Carlo stress simulations`);
      
      return {
        distribution,
        tailRisk,
        simulations,
      };
    } catch (error) {
      this.logger.error('Monte Carlo stress test failed:', error);
      throw new Error('Monte Carlo stress test failed');
    }
  }

  private async runSingleStressTest(portfolio: any, scenario: any): Promise<StressTestResultDto> {
    try {
      // Apply scenario shocks to portfolio
      const shockedPortfolio = this.applyScenarioShocks(portfolio, scenario);
      
      // Calculate portfolio loss
      const portfolioLoss = this.calculatePortfolioLoss(portfolio, shockedPortfolio);
      
      // Calculate loss percentage
      const lossPercentage = (portfolioLoss / portfolio.totalValue) * 100;
      
      // Estimate recovery time
      const recoveryTime = this.estimateRecoveryTime(scenario.severity, lossPercentage);
      
      // Identify risk factors impacted
      const riskFactors = this.identifyImpactedRiskFactors(scenario);
      
      return {
        scenario: scenario.name,
        portfolioLoss,
        lossPercentage,
        recoveryTime,
        riskFactors,
      };
    } catch (error) {
      this.logger.error(`Failed to run stress test for scenario ${scenario.name}:`, error);
      throw error;
    }
  }

  private applyScenarioShocks(portfolio: any, scenario: any): any {
    const shockedPositions = portfolio.positions.map((position: any) => {
      const shockedPosition = { ...position };
      
      // Apply market price shocks
      if (scenario.marketShocks?.price) {
        shockedPosition.currentPrice *= (1 + scenario.marketShocks.price);
      }
      
      // Apply volatility shocks
      if (scenario.marketShocks?.volatility) {
        shockedPosition.volatility = (shockedPosition.volatility || 0.02) * (1 + scenario.marketShocks.volatility);
      }
      
      // Apply liquidity shocks
      if (scenario.liquidityShocks) {
        shockedPosition.liquidity = (shockedPosition.liquidity || 1.0) * (1 - scenario.liquidityShocks);
      }
      
      // Apply credit spread shocks
      if (scenario.creditShocks?.spread) {
        shockedPosition.creditSpread = (shockedPosition.creditSpread || 0.01) * (1 + scenario.creditShocks.spread);
      }
      
      return shockedPosition;
    });
    
    return {
      ...portfolio,
      positions: shockedPositions,
      totalValue: this.calculatePortfolioValue(shockedPositions),
    };
  }

  private calculatePortfolioLoss(originalPortfolio: any, shockedPortfolio: any): number {
    return originalPortfolio.totalValue - shockedPortfolio.totalValue;
  }

  private calculatePortfolioValue(positions: any[]): number {
    return positions.reduce((total, position) => {
      return total + (position.size * position.currentPrice);
    }, 0);
  }

  private estimateRecoveryTime(severity: string, lossPercentage: number): number {
    // Estimate recovery time based on severity and loss magnitude
    const baseRecoveryTimes = {
      mild: 30,    // 30 days
      moderate: 90, // 90 days
      severe: 180, // 180 days
      extreme: 365, // 365 days
    };
    
    const baseTime = baseRecoveryTimes[severity] || 90;
    const lossMultiplier = Math.max(lossPercentage / 10, 1); // Scale by loss percentage
    
    return Math.round(baseTime * lossMultiplier);
  }

  private identifyImpactedRiskFactors(scenario: any): string[] {
    const riskFactors = [];
    
    if (scenario.marketShocks?.price) riskFactors.push('market_price');
    if (scenario.marketShocks?.volatility) riskFactors.push('market_volatility');
    if (scenario.liquidityShocks) riskFactors.push('liquidity');
    if (scenario.creditShocks?.spread) riskFactors.push('credit');
    if (scenario.interestRateShocks) riskFactors.push('interest_rate');
    if (scenario.currencyShocks) riskFactors.push('currency');
    if (scenario.commodityShocks) riskFactors.push('commodity');
    
    return riskFactors;
  }

  private calculateStressTestSummary(results: StressTestResultDto[]): any {
    const losses = results.map(result => result.portfolioLoss);
    const worstCaseLoss = Math.max(...losses);
    const worstCaseResult = results.find(result => result.portfolioLoss === worstCaseLoss);
    const averageLoss = losses.reduce((sum, loss) => sum + loss, 0) / losses.length;
    
    // Define failure threshold (e.g., >20% loss)
    const failureThreshold = 200000; // $200K loss
    const scenariosFailed = results.filter(result => result.portfolioLoss > failureThreshold).length;
    const scenariosPassed = results.length - scenariosFailed;
    
    // Calculate risk resilience score
    const riskResilience = Math.max(0, 1 - (worstCaseLoss / 1000000)); // Normalize by $1M
    
    return {
      worstCaseLoss,
      worstCaseScenario: worstCaseResult?.scenario || 'Unknown',
      averageLoss,
      scenariosPassed,
      scenariosFailed,
      riskResilience,
    };
  }

  private calculateDistributionStatistics(simulations: number[]): any {
    simulations.sort((a, b) => a - b);
    
    const mean = simulations.reduce((sum, val) => sum + val, 0) / simulations.length;
    const median = simulations[Math.floor(simulations.length / 2)];
    
    const variance = simulations.reduce((sum, val) => sum + Math.pow(val - mean, 2), 0) / simulations.length;
    const stdDev = Math.sqrt(variance);
    
    const percentiles = {
      p1: simulations[Math.floor(0.01 * simulations.length)],
      p5: simulations[Math.floor(0.05 * simulations.length)],
      p10: simulations[Math.floor(0.10 * simulations.length)],
      p25: simulations[Math.floor(0.25 * simulations.length)],
      p75: simulations[Math.floor(0.75 * simulations.length)],
      p90: simulations[Math.floor(0.90 * simulations.length)],
      p95: simulations[Math.floor(0.95 * simulations.length)],
      p99: simulations[Math.floor(0.99 * simulations.length)],
    };
    
    return {
      mean,
      median,
      stdDev,
      percentiles,
    };
  }

  private calculateTailRisk(simulations: number[]): any {
    const sortedSimulations = simulations.sort((a, b) => a - b);
    const tailSize = Math.floor(0.05 * simulations.length); // 5% tail
    const tail = sortedSimulations.slice(0, tailSize);
    
    const expectedShortfall = tail.length > 0 ? tail.reduce((sum, val) => sum + val, 0) / tail.length : 0;
    const maximumLoss = Math.min(...simulations);
    const tailProbability = tailSize / simulations.length;
    
    return {
      expectedShortfall,
      maximumLoss,
      tailProbability,
    };
  }

  private generateRandomScenario(): any {
    return {
      name: 'Random Scenario',
      marketShocks: {
        price: (Math.random() - 0.5) * 0.4, // -20% to +20%
        volatility: (Math.random() - 0.5) * 0.6, // -30% to +30%
      },
      liquidityShocks: Math.random() * 0.5, // 0% to 50% liquidity reduction
      creditShocks: {
        spread: Math.random() * 0.02, // 0% to 200bps spread increase
      },
      severity: this.getRandomSeverity(),
    };
  }

  private getRandomSeverity(): string {
    const severities = ['mild', 'moderate', 'severe', 'extreme'];
    const weights = [0.4, 0.3, 0.2, 0.1]; // Probability weights
    
    const random = Math.random();
    let cumulative = 0;
    
    for (let i = 0; i < severities.length; i++) {
      cumulative += weights[i];
      if (random < cumulative) {
        return severities[i];
      }
    }
    
    return 'moderate';
  }

  private validateCustomScenario(scenario: any): void {
    if (!scenario.name) {
      throw new Error('Scenario must have a name');
    }
    
    if (!scenario.marketShocks && !scenario.liquidityShocks && !scenario.creditShocks) {
      throw new Error('Scenario must have at least one shock component');
    }
    
    if (scenario.marketShocks?.price && Math.abs(scenario.marketShocks.price) > 0.5) {
      throw new Error('Price shock cannot exceed 50%');
    }
  }

  private async cacheStressTestResults(
    portfolioId: string,
    results: StressTestResultDto[],
    summary: any
  ): Promise<void> {
    const cacheKey = `stress_test:${portfolioId}`;
    const cacheData = {
      results,
      summary,
      timestamp: new Date().toISOString(),
    };
    
    await this.redis.setex(cacheKey, 7200, JSON.stringify(cacheData)); // 2 hours cache
  }

  private async storeCustomStressTestResult(
    portfolioId: string,
    scenario: any,
    result: StressTestResultDto
  ): Promise<void> {
    const customResult = {
      portfolioId,
      scenario,
      result,
      timestamp: new Date().toISOString(),
    };
    
    await this.redis.lpush('custom_stress_tests', JSON.stringify(customResult));
    await this.redis.ltrim('custom_stress_tests', 0, 999); // Keep last 1000
  }

  private initializeStressScenarios(): any[] {
    return [
      {
        name: '2008 Financial Crisis',
        severity: 'extreme',
        marketShocks: {
          price: -0.40, // 40% market drop
          volatility: 0.8, // 80% volatility increase
        },
        liquidityShocks: 0.6, // 60% liquidity reduction
        creditShocks: {
          spread: 0.05, // 500bps spread increase
        },
        description: 'Reproduction of 2008 financial crisis conditions',
      },
      {
        name: 'COVID-19 Market Crash',
        severity: 'severe',
        marketShocks: {
          price: -0.35, // 35% market drop
          volatility: 1.2, // 120% volatility increase
        },
        liquidityShocks: 0.4, // 40% liquidity reduction
        description: 'COVID-19 pandemic market impact',
      },
      {
        name: 'Interest Rate Shock',
        severity: 'moderate',
        interestRateShocks: 0.02, // 200bp rate increase
        marketShocks: {
          price: -0.15, // 15% bond price impact
        },
        description: 'Sudden interest rate increase',
      },
      {
        name: 'Currency Crisis',
        severity: 'severe',
        currencyShocks: {
          usd: -0.25, // 25% USD depreciation
          eur: 0.30,  // 30% EUR appreciation
        },
        marketShocks: {
          volatility: 0.5, // 50% volatility increase
        },
        description: 'Major currency crisis scenario',
      },
      {
        name: 'Commodity Price Collapse',
        severity: 'moderate',
        commodityShocks: {
          oil: -0.60, // 60% oil price drop
          gold: 0.20,  // 20% gold price increase
        },
        marketShocks: {
          volatility: 0.4, // 40% volatility increase
        },
        description: 'Commodity market disruption',
      },
      {
        name: 'Liquidity Crisis',
        severity: 'severe',
        liquidityShocks: 0.8, // 80% liquidity reduction
        marketShocks: {
          price: -0.20, // 20% price impact
          volatility: 0.6, // 60% volatility increase
        },
        description: 'Severe market liquidity freeze',
      },
      {
        name: 'Credit Event',
        severity: 'moderate',
        creditShocks: {
          spread: 0.03, // 300bps spread increase
        },
        marketShocks: {
          price: -0.10, // 10% market impact
        },
        description: 'Major credit default event',
      },
      {
        name: 'Geopolitical Shock',
        severity: 'moderate',
        marketShocks: {
          price: -0.25, // 25% market drop
          volatility: 0.7, // 70% volatility increase
        },
        currencyShocks: {
          safe_haven: 0.15, // 15% safe haven currency appreciation
        },
        description: 'Major geopolitical conflict',
      },
      {
        name: 'Tech Bubble Burst',
        severity: 'severe',
        marketShocks: {
          price: -0.45, // 45% tech sector drop
          volatility: 1.0, // 100% volatility increase
        },
        description: 'Technology sector bubble burst',
      },
      {
        name: 'Inflation Spike',
        severity: 'moderate',
        interestRateShocks: 0.015, // 150bp rate increase
        marketShocks: {
          price: -0.12, // 12% market impact
        },
        description: 'Unexpected inflation spike',
      },
    ];
  }

  async getStressTestHistory(portfolioId: string): Promise<any> {
    const cacheKey = `stress_test:${portfolioId}`;
    const cached = await this.redis.get(cacheKey);
    return cached ? JSON.parse(cached) : null;
  }

  async getAvailableScenarios(): Promise<any[]> {
    return this.STRESS_SCENARIOS.map(scenario => ({
      name: scenario.name,
      severity: scenario.severity,
      description: scenario.description,
    }));
  }

  async compareStressResults(portfolioId: string, comparisonPeriod: string = '30d'): Promise<any> {
    try {
      // Get current stress test results
      const currentResults = await this.getStressTestHistory(portfolioId);
      if (!currentResults) {
        throw new Error('No current stress test results found');
      }
      
      // Get historical results for comparison
      const historicalResults = await this.getHistoricalStressResults(portfolioId, comparisonPeriod);
      
      // Calculate comparison metrics
      const comparison = this.calculateStressComparison(currentResults, historicalResults);
      
      return comparison;
    } catch (error) {
      this.logger.error('Stress test comparison failed:', error);
      throw new Error('Stress test comparison failed');
    }
  }

  private async getHistoricalStressResults(portfolioId: string, period: string): Promise<any> {
    // Simplified historical data retrieval
    const cacheKey = `stress_history:${portfolioId}:${period}`;
    const cached = await this.redis.get(cacheKey);
    return cached ? JSON.parse(cached) : null;
  }

  private calculateStressComparison(current: any, historical: any): any {
    if (!historical) {
      return {
        trend: 'insufficient_data',
        worstCaseChange: 0,
        resilienceChange: 0,
        recommendation: 'Run more stress tests to establish trend',
      };
    }
    
    const worstCaseChange = ((current.summary.worstCaseLoss - historical.summary.worstCaseLoss) / historical.summary.worstCaseLoss) * 100;
    const resilienceChange = current.summary.riskResilience - historical.summary.riskResilience;
    
    let trend = 'stable';
    if (worstCaseChange > 10) trend = 'deteriorating';
    else if (worstCaseChange < -10) trend = 'improving';
    
    let recommendation = 'Continue monitoring';
    if (trend === 'deteriorating') recommendation = 'Review risk management strategies';
    else if (trend === 'improving') recommendation = 'Current strategies are effective';
    
    return {
      trend,
      worstCaseChange,
      resilienceChange,
      recommendation,
    };
  }
}
