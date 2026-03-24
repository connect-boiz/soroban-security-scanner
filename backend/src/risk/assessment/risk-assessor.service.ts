import { Injectable, Logger } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { ConfigService } from '@nestjs/config';
import { Redis } from '@nestjs/redis';

import { RiskData } from '../entities/risk-data.entity';
import { RiskAssessmentDto, RiskMetricsDto, RiskAlertDto } from '../dto/risk-assessment.dto';

@Injectable()
export class RiskAssessorService {
  private readonly logger = new Logger(RiskAssessorService.name);
  private readonly RISK_THRESHOLDS = {
    low: 0.3,
    medium: 0.6,
    high: 0.8,
    critical: 0.9,
  };

  constructor(
    @InjectRepository(RiskData)
    private readonly riskDataRepository: Repository<RiskData>,
    private readonly configService: ConfigService,
    private readonly redis: Redis,
  ) {}

  async assessRisk(assessmentDto: RiskAssessmentDto): Promise<{
    metrics: RiskMetricsDto;
    alerts: RiskAlertDto[];
    overallScore: number;
    riskLevel: 'low' | 'medium' | 'high' | 'critical';
  }> {
    const startTime = Date.now();
    
    try {
      const portfolio = assessmentDto.portfolio;
      const marketFactors = assessmentDto.marketFactors || this.getDefaultMarketFactors();
      
      // Calculate portfolio metrics
      const metrics = await this.calculateRiskMetrics(portfolio, marketFactors, assessmentDto.confidenceLevel || 0.95);
      
      // Generate risk alerts
      const alerts = await this.generateRiskAlerts(portfolio.id, metrics, assessmentDto.userId);
      
      // Calculate overall risk score
      const overallScore = this.calculateOverallRiskScore(metrics, alerts);
      
      // Determine risk level
      const riskLevel = this.determineRiskLevel(overallScore);
      
      // Store risk data
      await this.storeRiskData({
        userId: assessmentDto.userId,
        portfolioId: portfolio.id,
        riskType: 'market',
        riskScore: overallScore,
        exposure: portfolio.totalValue,
        metrics,
        marketFactors,
        severity: riskLevel,
        timestamp: new Date(),
      });
      
      // Cache results
      await this.cacheRiskAssessment(portfolio.id, {
        metrics,
        alerts,
        overallScore,
        riskLevel,
        timestamp: new Date(),
      });
      
      const processingTime = Date.now() - startTime;
      this.logger.log(`Risk assessment completed for portfolio ${portfolio.id} in ${processingTime}ms`);
      
      return {
        metrics,
        alerts,
        overallScore,
        riskLevel,
      };
    } catch (error) {
      this.logger.error('Risk assessment failed:', error);
      throw new Error('Risk assessment failed');
    }
  }

  private async calculateRiskMetrics(
    portfolio: any,
    marketFactors: any,
    confidenceLevel: number
  ): Promise<RiskMetricsDto> {
    // Calculate portfolio returns
    const returns = this.calculatePortfolioReturns(portfolio);
    
    // Calculate Value at Risk (VaR)
    const var1d = this.calculateVaR(returns, confidenceLevel, 1);
    const var10d = this.calculateVaR(returns, confidenceLevel, 10);
    const var30d = this.calculateVaR(returns, confidenceLevel, 30);
    
    // Calculate Expected Shortfall
    const expectedShortfall = this.calculateExpectedShortfall(returns, confidenceLevel);
    
    // Calculate beta
    const beta = this.calculateBeta(portfolio, marketFactors);
    
    // Calculate volatility
    const volatility = this.calculateVolatility(returns);
    
    // Calculate correlation
    const correlation = this.calculateCorrelation(portfolio, marketFactors);
    
    // Calculate concentration risk
    const concentration = this.calculateConcentrationRisk(portfolio);
    
    return {
      var1d,
      var10d,
      var30d,
      expectedShortfall,
      beta,
      volatility,
      correlation,
      concentration,
    };
  }

  private calculatePortfolioReturns(portfolio: any): number[] {
    // Simulate historical returns based on portfolio composition
    const returns: number[] = [];
    const baseReturn = 0.0008; // Daily return ~20% annual
    const volatility = 0.02; // 2% daily volatility
    
    for (let i = 0; i < 252; i++) { // 252 trading days
      const randomShock = this.normalRandom() * volatility;
      returns.push(baseReturn + randomShock);
    }
    
    return returns;
  }

  private calculateVaR(returns: number[], confidenceLevel: number, timeHorizon: number): number {
    const sortedReturns = returns.sort((a, b) => a - b);
    const index = Math.floor((1 - confidenceLevel) * sortedReturns.length);
    const dailyVaR = Math.abs(sortedReturns[index] || 0);
    
    // Scale by time horizon (square root of time rule)
    return dailyVaR * Math.sqrt(timeHorizon);
  }

  private calculateExpectedShortfall(returns: number[], confidenceLevel: number): number {
    const sortedReturns = returns.sort((a, b) => a - b);
    const cutoffIndex = Math.floor((1 - confidenceLevel) * sortedReturns.length);
    const tailReturns = sortedReturns.slice(0, cutoffIndex);
    
    if (tailReturns.length === 0) return 0;
    
    const averageLoss = tailReturns.reduce((sum, ret) => sum + ret, 0) / tailReturns.length;
    return Math.abs(averageLoss);
  }

  private calculateBeta(portfolio: any, marketFactors: any): number {
    // Simplified beta calculation
    const marketVolatility = marketFactors.volatility || 0.015;
    const portfolioVolatility = this.calculatePortfolioVolatility(portfolio);
    const correlation = 0.7; // Assumed correlation with market
    
    return (correlation * portfolioVolatility) / marketVolatility;
  }

  private calculateVolatility(returns: number[]): number {
    const mean = returns.reduce((sum, ret) => sum + ret, 0) / returns.length;
    const variance = returns.reduce((sum, ret) => sum + Math.pow(ret - mean, 2), 0) / returns.length;
    return Math.sqrt(variance);
  }

  private calculateCorrelation(portfolio: any, marketFactors: any): number {
    // Simplified correlation calculation
    return 0.65; // Assumed correlation
  }

  private calculateConcentrationRisk(portfolio: any): number {
    // Herfindahl-Hirschman Index for concentration
    const totalValue = portfolio.totalValue;
    let hhi = 0;
    
    portfolio.positions.forEach((position: any) => {
      const weight = position.size * position.currentPrice / totalValue;
      hhi += Math.pow(weight, 2);
    });
    
    return hhi;
  }

  private calculatePortfolioVolatility(portfolio: any): number {
    // Simplified portfolio volatility calculation
    let portfolioVariance = 0;
    const totalValue = portfolio.totalValue;
    
    portfolio.positions.forEach((position: any) => {
      const weight = (position.size * position.currentPrice) / totalValue;
      const assetVolatility = 0.02; // Assumed 2% volatility per asset
      portfolioVariance += Math.pow(weight * assetVolatility, 2);
    });
    
    return Math.sqrt(portfolioVariance);
  }

  private async generateRiskAlerts(
    portfolioId: string,
    metrics: RiskMetricsDto,
    userId: string
  ): Promise<RiskAlertDto[]> {
    const alerts: RiskAlertDto[] = [];
    
    // VaR alerts
    if (metrics.var1d > metrics.concentration * 0.1) {
      alerts.push({
        id: this.generateAlertId(),
        riskType: 'market',
        severity: 'high',
        message: `1-day VaR (${metrics.var1d.toFixed(2)}%) exceeds acceptable threshold`,
        currentScore: metrics.var1d,
        threshold: metrics.concentration * 0.1,
        recommendation: 'Consider reducing portfolio size or increasing hedging',
        timestamp: new Date().toISOString(),
      });
    }
    
    // Volatility alerts
    if (metrics.volatility > 0.03) {
      alerts.push({
        id: this.generateAlertId(),
        riskType: 'market',
        severity: 'medium',
        message: `Portfolio volatility (${(metrics.volatility * 100).toFixed(2)}%) is elevated`,
        currentScore: metrics.volatility,
        threshold: 0.03,
        recommendation: 'Monitor market conditions and consider defensive positions',
        timestamp: new Date().toISOString(),
      });
    }
    
    // Concentration alerts
    if (metrics.concentration > 0.25) {
      alerts.push({
        id: this.generateAlertId(),
        riskType: 'market',
        severity: 'high',
        message: `Portfolio concentration risk (${(metrics.concentration * 100).toFixed(2)}%) is high`,
        currentScore: metrics.concentration,
        threshold: 0.25,
        recommendation: 'Diversify portfolio to reduce concentration risk',
        timestamp: new Date().toISOString(),
      });
    }
    
    return alerts;
  }

  private calculateOverallRiskScore(metrics: RiskMetricsDto, alerts: RiskAlertDto[]): number {
    let score = 0;
    
    // VaR contribution (30%)
    score += (metrics.var1d / 0.05) * 0.3; // Normalized by 5% VaR
    
    // Volatility contribution (25%)
    score += (metrics.volatility / 0.04) * 0.25; // Normalized by 4% volatility
    
    // Concentration contribution (20%)
    score += metrics.concentration * 0.2;
    
    // Beta contribution (15%)
    score += (Math.abs(metrics.beta - 1) / 2) * 0.15; // Deviation from market beta
    
    // Alerts contribution (10%)
    const alertSeverityScore = alerts.reduce((sum, alert) => {
      const severityWeight = {
        low: 0.1,
        medium: 0.3,
        high: 0.6,
        critical: 1.0,
      };
      return sum + severityWeight[alert.severity];
    }, 0) / Math.max(alerts.length, 1);
    
    score += alertSeverityScore * 0.1;
    
    return Math.min(score, 1); // Cap at 1
  }

  private determineRiskLevel(score: number): 'low' | 'medium' | 'high' | 'critical' {
    if (score >= this.RISK_THRESHOLDS.critical) return 'critical';
    if (score >= this.RISK_THRESHOLDS.high) return 'high';
    if (score >= this.RISK_THRESHOLDS.medium) return 'medium';
    return 'low';
  }

  private async storeRiskData(riskData: Partial<RiskData>): Promise<void> {
    const entity = this.riskDataRepository.create(riskData);
    await this.riskDataRepository.save(entity);
  }

  private async cacheRiskAssessment(portfolioId: string, assessment: any): Promise<void> {
    const cacheKey = `risk:assessment:${portfolioId}`;
    await this.redis.setex(cacheKey, 300, JSON.stringify(assessment)); // 5 minutes cache
  }

  private getDefaultMarketFactors(): any {
    return {
      price: 1.0,
      volume: 1.0,
      volatility: 0.015,
      interestRate: 0.05,
      exchangeRate: 1.0,
    };
  }

  private generateAlertId(): string {
    return `alert_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  private normalRandom(): number {
    // Box-Muller transform for normal distribution
    const u1 = Math.random();
    const u2 = Math.random();
    return Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
  }

  async getHistoricalRiskScores(portfolioId: string, days: number = 30): Promise<any[]> {
    const cacheKey = `risk:history:${portfolioId}:${days}`;
    const cached = await this.redis.get(cacheKey);
    
    if (cached) {
      return JSON.parse(cached);
    }
    
    const endDate = new Date();
    const startDate = new Date();
    startDate.setDate(endDate.getDate() - days);
    
    const riskData = await this.riskDataRepository.find({
      where: {
        portfolioId,
        timestamp: {
          $gte: startDate,
          $lte: endDate,
        },
      },
      order: { timestamp: 'ASC' },
    });
    
    const history = riskData.map(data => ({
      timestamp: data.timestamp,
      riskScore: data.riskScore,
      volatility: data.metrics?.volatility || 0,
      var1d: data.metrics?.var1d || 0,
    }));
    
    await this.redis.setex(cacheKey, 3600, JSON.stringify(history)); // 1 hour cache
    
    return history;
  }
}
