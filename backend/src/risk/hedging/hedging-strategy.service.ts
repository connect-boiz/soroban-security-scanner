import { Injectable, Logger } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { Redis } from '@nestjs/redis';

import { RiskData } from '../entities/risk-data.entity';
import { HedgingStrategyDto } from '../dto/risk-report.dto';

@Injectable()
export class HedgingStrategyService {
  private readonly logger = new Logger(HedgingStrategyService.name);
  private readonly HEDGING_INSTRUMENTS = {
    futures: ['ES', 'NQ', 'YM', 'RTY', 'CL', 'GC', 'SI'],
    options: ['SPX', 'NDX', 'DJX', 'RUT'],
    etfs: ['SPY', 'QQQ', 'IWM', 'VTI', 'GLD', 'TLT'],
    swaps: ['Interest Rate Swap', 'Currency Swap', 'Commodity Swap'],
  };

  constructor(
    private readonly configService: ConfigService,
    private readonly redis: Redis,
  ) {}

  async generateHedgingStrategies(portfolio: any, riskMetrics: any): Promise<HedgingStrategyDto[]> {
    try {
      const strategies: HedgingStrategyDto[] = [];
      
      // Delta hedging strategy
      const deltaHedge = await this.generateDeltaHedgingStrategy(portfolio, riskMetrics);
      if (deltaHedge) strategies.push(deltaHedge);
      
      // Volatility hedging strategy
      const volatilityHedge = await this.generateVolatilityHedgingStrategy(portfolio, riskMetrics);
      if (volatilityHedge) strategies.push(volatilityHedge);
      
      // Beta hedging strategy
      const betaHedge = await this.generateBetaHedgingStrategy(portfolio, riskMetrics);
      if (betaHedge) strategies.push(betaHedge);
      
      // Currency hedging strategy
      const currencyHedge = await this.generateCurrencyHedgingStrategy(portfolio, riskMetrics);
      if (currencyHedge) strategies.push(currencyHedge);
      
      // Duration hedging strategy
      const durationHedge = await this.generateDurationHedgingStrategy(portfolio, riskMetrics);
      if (durationHedge) strategies.push(durationHedge);
      
      // Sort by effectiveness
      strategies.sort((a, b) => b.effectiveness - a.effectiveness);
      
      this.logger.log(`Generated ${strategies.length} hedging strategies for portfolio`);
      
      return strategies;
    } catch (error) {
      this.logger.error('Failed to generate hedging strategies:', error);
      throw new Error('Hedging strategy generation failed');
    }
  }

  private async generateDeltaHedgingStrategy(portfolio: any, riskMetrics: any): Promise<HedgingStrategyDto | null> {
    const portfolioDelta = this.calculatePortfolioDelta(portfolio);
    const portfolioValue = portfolio.totalValue;
    
    if (Math.abs(portfolioDelta) < 0.1) {
      return null; // No significant delta exposure
    }
    
    // Use futures for delta hedging
    const hedgeInstrument = this.selectOptimalFutures(portfolio);
    const hedgeRatio = Math.abs(portfolioDelta);
    const effectiveness = this.calculateDeltaHedgingEffectiveness(portfolioDelta, hedgeRatio);
    const cost = this.calculateHedgingCost(portfolioValue, hedgeRatio, hedgeInstrument);
    
    return {
      name: 'Delta Hedging with Futures',
      type: 'delta',
      effectiveness,
      cost,
      hedgeRatio,
      instruments: [hedgeInstrument],
    };
  }

  private async generateVolatilityHedgingStrategy(portfolio: any, riskMetrics: any): Promise<HedgingStrategyDto | null> {
    const portfolioVolatility = riskMetrics.volatility || 0.02;
    const vegaExposure = this.calculatePortfolioVega(portfolio);
    
    if (Math.abs(vegaExposure) < 1000) {
      return null; // No significant volatility exposure
    }
    
    // Use options for volatility hedging
    const hedgeInstrument = this.selectOptimalOptions(portfolio);
    const hedgeRatio = Math.abs(vegaExposure) / 10000; // Normalize
    const effectiveness = this.calculateVolatilityHedgingEffectiveness(vegaExposure, hedgeRatio);
    const cost = this.calculateHedgingCost(portfolio.totalValue, hedgeRatio, hedgeInstrument);
    
    return {
      name: 'Volatility Hedging with Options',
      type: 'volatility',
      effectiveness,
      cost,
      hedgeRatio,
      instruments: [hedgeInstrument],
    };
  }

  private async generateBetaHedgingStrategy(portfolio: any, riskMetrics: any): Promise<HedgingStrategyDto | null> {
    const portfolioBeta = riskMetrics.beta || 1.0;
    const betaDeviation = Math.abs(portfolioBeta - 1.0);
    
    if (betaDeviation < 0.1) {
      return null; // No significant beta deviation
    }
    
    // Use index futures/ETFs for beta hedging
    const hedgeInstrument = portfolioBeta > 1.0 ? 'SPY' : 'TLT';
    const hedgeRatio = betaDeviation;
    const effectiveness = this.calculateBetaHedgingEffectiveness(portfolioBeta, hedgeRatio);
    const cost = this.calculateHedgingCost(portfolio.totalValue, hedgeRatio, hedgeInstrument);
    
    return {
      name: 'Beta Hedging with Index ETFs',
      type: 'beta',
      effectiveness,
      cost,
      hedgeRatio,
      instruments: [hedgeInstrument],
    };
  }

  private async generateCurrencyHedgingStrategy(portfolio: any, riskMetrics: any): Promise<HedgingStrategyDto | null> {
    const currencyExposure = this.calculateCurrencyExposure(portfolio);
    
    if (currencyExposure < 0.05) {
      return null; // No significant currency exposure
    }
    
    // Use currency forwards or options
    const hedgeInstrument = 'Currency Forward';
    const hedgeRatio = currencyExposure;
    const effectiveness = this.calculateCurrencyHedgingEffectiveness(currencyExposure, hedgeRatio);
    const cost = this.calculateHedgingCost(portfolio.totalValue, hedgeRatio, hedgeInstrument);
    
    return {
      name: 'Currency Hedging with Forwards',
      type: 'currency',
      effectiveness,
      cost,
      hedgeRatio,
      instruments: [hedgeInstrument],
    };
  }

  private async generateDurationHedgingStrategy(portfolio: any, riskMetrics: any): Promise<HedgingStrategyDto | null> {
    const durationExposure = this.calculateDurationExposure(portfolio);
    
    if (Math.abs(durationExposure) < 1.0) {
      return null; // No significant duration exposure
    }
    
    // Use interest rate swaps or bond futures
    const hedgeInstrument = durationExposure > 0 ? 'Interest Rate Swap' : 'Bond Futures';
    const hedgeRatio = Math.abs(durationExposure) / 10; // Normalize
    const effectiveness = this.calculateDurationHedgingEffectiveness(durationExposure, hedgeRatio);
    const cost = this.calculateHedgingCost(portfolio.totalValue, hedgeRatio, hedgeInstrument);
    
    return {
      name: 'Duration Hedging with Swaps',
      type: 'duration',
      effectiveness,
      cost,
      hedgeRatio,
      instruments: [hedgeInstrument],
    };
  }

  private calculatePortfolioDelta(portfolio: any): number {
    let totalDelta = 0;
    
    portfolio.positions?.forEach((position: any) => {
      const positionDelta = this.calculatePositionDelta(position);
      totalDelta += positionDelta;
    });
    
    return totalDelta;
  }

  private calculatePositionDelta(position: any): number {
    // Simplified delta calculation
    const positionValue = position.size * position.currentPrice;
    
    switch (position.type) {
      case 'stock':
        return positionValue / 100; // Normalize
      case 'option':
        return positionValue * 0.5; // Assume 0.5 delta
      case 'future':
        return positionValue / 50; // Normalize
      default:
        return positionValue / 100;
    }
  }

  private calculatePortfolioVega(portfolio: any): number {
    let totalVega = 0;
    
    portfolio.positions?.forEach((position: any) => {
      if (position.type === 'option') {
        const positionValue = position.size * position.currentPrice;
        totalVega += positionValue * 0.1; // Assume 0.1 vega per option
      }
    });
    
    return totalVega;
  }

  private calculateCurrencyExposure(portfolio: any): number {
    // Simplified currency exposure calculation
    let foreignExposure = 0;
    const totalValue = portfolio.totalValue;
    
    portfolio.positions?.forEach((position: any) => {
      // Assume 20% of non-USD positions have currency risk
      if (position.currency && position.currency !== 'USD') {
        foreignExposure += (position.size * position.currentPrice) * 0.2;
      }
    });
    
    return foreignExposure / totalValue;
  }

  private calculateDurationExposure(portfolio: any): number {
    let totalDuration = 0;
    let totalValue = 0;
    
    portfolio.positions?.forEach((position: any) => {
      const positionValue = position.size * position.currentPrice;
      
      if (position.type === 'bond' || position.type === 'fixed_income') {
        const duration = position.duration || 5.0; // Default 5 years
        totalDuration += positionValue * duration;
      }
      
      totalValue += positionValue;
    });
    
    return totalValue > 0 ? totalDuration / totalValue : 0;
  }

  private selectOptimalFutures(portfolio: any): string {
    // Select most appropriate futures contract based on portfolio composition
    const hasEquity = portfolio.positions?.some((p: any) => p.type === 'stock');
    const hasCommodities = portfolio.positions?.some((p: any) => p.type === 'commodity');
    
    if (hasCommodities) return 'CL'; // Crude oil futures
    if (hasEquity) return 'ES'; // S&P 500 futures
    return 'SPY'; // Default to SPY ETF
  }

  private selectOptimalOptions(portfolio: any): string {
    // Select most appropriate options based on portfolio
    const hasLargeCap = portfolio.positions?.some((p: any) => p.size > 1000);
    
    if (hasLargeCap) return 'SPX'; // S&P 500 index options
    return 'SPY'; // SPY ETF options
  }

  private calculateDeltaHedgingEffectiveness(portfolioDelta: number, hedgeRatio: number): number {
    // Simulate hedging effectiveness based on hedge ratio
    const baseEffectiveness = 0.85;
    const ratioBonus = Math.min(hedgeRatio * 0.1, 0.1);
    return Math.min(baseEffectiveness + ratioBonus, 0.95);
  }

  private calculateVolatilityHedgingEffectiveness(vegaExposure: number, hedgeRatio: number): number {
    // Volatility hedging is typically less effective
    const baseEffectiveness = 0.70;
    const ratioBonus = Math.min(hedgeRatio * 0.15, 0.15);
    return Math.min(baseEffectiveness + ratioBonus, 0.85);
  }

  private calculateBetaHedgingEffectiveness(portfolioBeta: number, hedgeRatio: number): number {
    // Beta hedging effectiveness
    const baseEffectiveness = 0.80;
    const deviation = Math.abs(portfolioBeta - 1.0);
    const deviationBonus = Math.min(deviation * 0.1, 0.1);
    return Math.min(baseEffectiveness + deviationBonus, 0.90);
  }

  private calculateCurrencyHedgingEffectiveness(currencyExposure: number, hedgeRatio: number): number {
    // Currency hedging is usually quite effective
    const baseEffectiveness = 0.88;
    const ratioBonus = Math.min(hedgeRatio * 0.08, 0.08);
    return Math.min(baseEffectiveness + ratioBonus, 0.95);
  }

  private calculateDurationHedgingEffectiveness(durationExposure: number, hedgeRatio: number): number {
    // Duration hedging effectiveness
    const baseEffectiveness = 0.82;
    const exposureBonus = Math.min(Math.abs(durationExposure) * 0.02, 0.1);
    return Math.min(baseEffectiveness + exposureBonus, 0.92);
  }

  private calculateHedgingCost(portfolioValue: number, hedgeRatio: number, instrument: string): number {
    // Calculate hedging cost based on instrument type and hedge ratio
    const baseCosts = {
      'ES': 0.001, // 0.1% annually
      'SPY': 0.0009, // 0.09% annually
      'SPX': 0.0015, // 0.15% annually
      'Currency Forward': 0.002, // 0.2% annually
      'Interest Rate Swap': 0.0012, // 0.12% annually
      'Bond Futures': 0.0008, // 0.08% annually
    };
    
    const baseCost = baseCosts[instrument] || 0.001;
    return portfolioValue * baseCost * hedgeRatio;
  }

  async implementHedgingStrategy(portfolioId: string, strategy: HedgingStrategyDto): Promise<any> {
    try {
      const implementation = {
        portfolioId,
        strategy: strategy.name,
        type: strategy.type,
        instruments: strategy.instruments,
        hedgeRatio: strategy.hedgeRatio,
        expectedCost: strategy.cost,
        expectedEffectiveness: strategy.effectiveness,
        status: 'pending',
        timestamp: new Date().toISOString(),
      };
      
      // Queue implementation
      await this.redis.lpush('hedging:implementations', JSON.stringify(implementation));
      
      // Update portfolio hedging status
      await this.redis.set(`hedging:${portfolioId}`, JSON.stringify({
        active: true,
        strategy: strategy.name,
        implementation,
      }));
      
      this.logger.log(`Queued hedging strategy implementation for portfolio ${portfolioId}`);
      
      return implementation;
    } catch (error) {
      this.logger.error('Failed to implement hedging strategy:', error);
      throw new Error('Hedging strategy implementation failed');
    }
  }

  async evaluateHedgingEffectiveness(portfolioId: string, strategyId: string): Promise<any> {
    try {
      // Get portfolio metrics before and after hedging
      const beforeMetrics = await this.getHistoricalMetrics(portfolioId, 'before_hedge');
      const afterMetrics = await this.getHistoricalMetrics(portfolioId, 'after_hedge');
      
      if (!beforeMetrics || !afterMetrics) {
        throw new Error('Insufficient data for hedging evaluation');
      }
      
      // Calculate risk reduction
      const riskReduction = this.calculateRiskReduction(beforeMetrics, afterMetrics);
      
      // Calculate cost vs benefit
      const costBenefit = this.calculateCostBenefitAnalysis(beforeMetrics, afterMetrics, strategyId);
      
      const evaluation = {
        portfolioId,
        strategyId,
        riskReduction,
        costBenefit,
        effectiveness: riskReduction.percentage / 100, // Convert to effectiveness score
        recommendation: riskReduction.percentage > 25 ? 'maintain' : 'reconsider',
        evaluationDate: new Date().toISOString(),
      };
      
      // Cache evaluation
      await this.redis.setex(`evaluation:${strategyId}`, 86400, JSON.stringify(evaluation));
      
      return evaluation;
    } catch (error) {
      this.logger.error('Failed to evaluate hedging effectiveness:', error);
      throw new Error('Hedging evaluation failed');
    }
  }

  private calculateRiskReduction(beforeMetrics: any, afterMetrics: any): any {
    const varReduction = ((beforeMetrics.var1d - afterMetrics.var1d) / beforeMetrics.var1d) * 100;
    const volatilityReduction = ((beforeMetrics.volatility - afterMetrics.volatility) / beforeMetrics.volatility) * 100;
    const riskScoreReduction = ((beforeMetrics.riskScore - afterMetrics.riskScore) / beforeMetrics.riskScore) * 100;
    
    return {
      varReduction,
      volatilityReduction,
      riskScoreReduction,
      percentage: (varReduction + volatilityReduction + riskScoreReduction) / 3,
    };
  }

  private calculateCostBenefitAnalysis(beforeMetrics: any, afterMetrics: any, strategyId: string): any {
    // Simplified cost-benefit analysis
    const riskReductionValue = beforeMetrics.portfolioValue * 0.01; // 1% of portfolio value
    const hedgingCost = afterMetrics.hedgingCost || 0;
    
    return {
      benefit: riskReductionValue,
      cost: hedgingCost,
      ratio: riskReductionValue / Math.max(hedgingCost, 1),
      netBenefit: riskReductionValue - hedgingCost,
    };
  }

  private async getHistoricalMetrics(portfolioId: string, period: string): Promise<any> {
    const cacheKey = `metrics:${portfolioId}:${period}`;
    const cached = await this.redis.get(cacheKey);
    return cached ? JSON.parse(cached) : null;
  }

  async getActiveHedgingStrategies(portfolioId: string): Promise<any[]> {
    const implementations = await this.redis.lrange('hedging:implementations', 0, -1);
    return implementations
      .map(impl => JSON.parse(impl))
      .filter(impl => impl.portfolioId === portfolioId && impl.status === 'active');
  }

  async optimizeHedgingPortfolio(portfolioId: string): Promise<any> {
    try {
      // Get current portfolio and risk metrics
      const portfolio = await this.redis.get(`portfolio:active:${portfolioId}`);
      const metrics = await this.redis.get(`metrics:${portfolioId}`);
      
      if (!portfolio || !metrics) {
        throw new Error('Portfolio or metrics not found');
      }
      
      // Generate optimal hedging mix
      const strategies = await this.generateHedgingStrategies(
        JSON.parse(portfolio),
        JSON.parse(metrics)
      );
      
      // Select top 3 most effective strategies
      const optimalMix = strategies.slice(0, 3);
      
      // Calculate combined effectiveness and cost
      const combinedEffectiveness = optimalMix.reduce((sum, s) => sum + s.effectiveness, 0) / optimalMix.length;
      const combinedCost = optimalMix.reduce((sum, s) => sum + s.cost, 0);
      
      const optimization = {
        portfolioId,
        recommendedStrategies: optimalMix,
        combinedEffectiveness,
        combinedCost,
        expectedRiskReduction: combinedEffectiveness * 0.3, // Assume 30% risk reduction
        optimizationDate: new Date().toISOString(),
      };
      
      // Cache optimization result
      await this.redis.setex(`optimization:${portfolioId}`, 3600, JSON.stringify(optimization));
      
      return optimization;
    } catch (error) {
      this.logger.error('Failed to optimize hedging portfolio:', error);
      throw new Error('Hedging optimization failed');
    }
  }
}
