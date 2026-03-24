import { Injectable, Logger, OnModuleInit } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { Redis } from '@nestjs/redis';
import { Cron, CronExpression } from '@nestjs/schedule';

import { RiskAssessorService } from '../assessment/risk-assessor.service';
import { RiskData } from '../entities/risk-data.entity';
import { RiskAlertDto } from '../dto/risk-alert.dto';

@Injectable()
export class RealTimeMonitorService implements OnModuleInit {
  private readonly logger = new Logger(RealTimeMonitorService.name);
  private readonly MONITORING_INTERVAL = 10000; // 10 seconds
  private activePortfolios = new Map<string, any>();
  private alertThresholds = new Map<string, any>();

  constructor(
    private readonly configService: ConfigService,
    private readonly redis: Redis,
    private readonly riskAssessorService: RiskAssessorService,
  ) {}

  async onModuleInit() {
    this.logger.log('Initializing real-time risk monitoring service');
    await this.loadActivePortfolios();
    await this.initializeAlertThresholds();
  }

  @Cron(CronExpression.EVERY_10_SECONDS)
  async monitorPortfolioRisks(): Promise<void> {
    const startTime = Date.now();
    
    try {
      const portfolios = Array.from(this.activePortfolios.keys());
      
      for (const portfolioId of portfolios) {
        await this.monitorPortfolio(portfolioId);
      }
      
      const monitoringTime = Date.now() - startTime;
      if (monitoringTime > this.MONITORING_INTERVAL) {
        this.logger.warn(`Monitoring took ${monitoringTime}ms, exceeding interval of ${this.MONITORING_INTERVAL}ms`);
      }
    } catch (error) {
      this.logger.error('Real-time monitoring failed:', error);
    }
  }

  async startMonitoringPortfolio(portfolioId: string, portfolioData: any): Promise<void> {
    this.activePortfolios.set(portfolioId, {
      ...portfolioData,
      lastUpdated: new Date(),
      monitoringActive: true,
    });
    
    await this.redis.set(`portfolio:active:${portfolioId}`, JSON.stringify(portfolioData));
    this.logger.log(`Started monitoring portfolio: ${portfolioId}`);
  }

  async stopMonitoringPortfolio(portfolioId: string): Promise<void> {
    this.activePortfolios.delete(portfolioId);
    await this.redis.del(`portfolio:active:${portfolioId}`);
    this.logger.log(`Stopped monitoring portfolio: ${portfolioId}`);
  }

  private async monitorPortfolio(portfolioId: string): Promise<void> {
    const portfolio = this.activePortfolios.get(portfolioId);
    if (!portfolio || !portfolio.monitoringActive) {
      return;
    }

    try {
      // Get current market data
      const marketData = await this.getCurrentMarketData(portfolio);
      
      // Perform quick risk assessment
      const riskAssessment = await this.performQuickRiskAssessment(portfolio, marketData);
      
      // Check for alert conditions
      const alerts = await this.checkAlertConditions(portfolioId, riskAssessment);
      
      // Process alerts
      if (alerts.length > 0) {
        await this.processAlerts(portfolioId, alerts);
      }
      
      // Update portfolio metrics
      await this.updatePortfolioMetrics(portfolioId, riskAssessment);
      
      // Check for automated mitigation
      await this.checkAutomatedMitigation(portfolioId, riskAssessment, alerts);
      
    } catch (error) {
      this.logger.error(`Failed to monitor portfolio ${portfolioId}:`, error);
    }
  }

  private async getCurrentMarketData(portfolio: any): Promise<any> {
    // Simulate real-time market data
    const baseFactors = {
      price: 1.0,
      volume: 1.0,
      volatility: 0.015,
      interestRate: 0.05,
      exchangeRate: 1.0,
    };
    
    // Add random market movements
    return {
      ...baseFactors,
      price: baseFactors.price * (1 + (Math.random() - 0.5) * 0.02),
      volatility: baseFactors.volatility * (1 + (Math.random() - 0.5) * 0.3),
      volume: baseFactors.volume * (1 + (Math.random() - 0.5) * 0.2),
    };
  }

  private async performQuickRiskAssessment(portfolio: any, marketData: any): Promise<any> {
    // Simplified quick assessment for real-time monitoring
    const portfolioValue = portfolio.totalValue;
    const volatility = marketData.volatility;
    
    // Quick VaR calculation
    const quickVar = portfolioValue * volatility * 2.33; // 99% confidence
    
    // Quick beta estimation
    const quickBeta = 1.0 + (Math.random() - 0.5) * 0.4;
    
    // Quick concentration check
    const concentration = this.calculateQuickConcentration(portfolio);
    
    return {
      portfolioValue,
      volatility,
      quickVar,
      quickBeta,
      concentration,
      riskScore: this.calculateQuickRiskScore(quickVar, volatility, concentration),
      timestamp: new Date(),
    };
  }

  private calculateQuickConcentration(portfolio: any): number {
    const totalValue = portfolio.totalValue;
    let maxConcentration = 0;
    
    portfolio.positions?.forEach((position: any) => {
      const weight = (position.size * position.currentPrice) / totalValue;
      maxConcentration = Math.max(maxConcentration, weight);
    });
    
    return maxConcentration;
  }

  private calculateQuickRiskScore(varValue: number, volatility: number, concentration: number): number {
    // Normalize and combine risk factors
    const varScore = Math.min(varValue / 1000000, 1); // Normalize by $1M
    const volatilityScore = Math.min(volatility / 0.05, 1); // Normalize by 5%
    const concentrationScore = concentration;
    
    return (varScore * 0.4 + volatilityScore * 0.3 + concentrationScore * 0.3);
  }

  private async checkAlertConditions(portfolioId: string, assessment: any): Promise<RiskAlertDto[]> {
    const alerts: RiskAlertDto[] = [];
    const thresholds = this.alertThresholds.get(portfolioId) || this.getDefaultThresholds();
    
    // VaR threshold check
    if (assessment.quickVar > thresholds.varThreshold) {
      alerts.push({
        id: this.generateAlertId(),
        riskType: 'market',
        severity: assessment.quickVar > thresholds.varThreshold * 1.5 ? 'critical' : 'high',
        message: `Real-time VaR ($${assessment.quickVar.toFixed(2)}) exceeds threshold`,
        currentScore: assessment.quickVar,
        threshold: thresholds.varThreshold,
        recommendation: 'Consider immediate position reduction',
        timestamp: new Date().toISOString(),
      });
    }
    
    // Volatility threshold check
    if (assessment.volatility > thresholds.volatilityThreshold) {
      alerts.push({
        id: this.generateAlertId(),
        riskType: 'market',
        severity: assessment.volatility > thresholds.volatilityThreshold * 1.5 ? 'high' : 'medium',
        message: `Market volatility (${(assessment.volatility * 100).toFixed(2)}%) is elevated`,
        currentScore: assessment.volatility,
        threshold: thresholds.volatilityThreshold,
        recommendation: 'Monitor market conditions closely',
        timestamp: new Date().toISOString(),
      });
    }
    
    // Concentration threshold check
    if (assessment.concentration > thresholds.concentrationThreshold) {
      alerts.push({
        id: this.generateAlertId(),
        riskType: 'market',
        severity: 'medium',
        message: `Position concentration (${(assessment.concentration * 100).toFixed(2)}%) is high`,
        currentScore: assessment.concentration,
        threshold: thresholds.concentrationThreshold,
        recommendation: 'Consider diversifying positions',
        timestamp: new Date().toISOString(),
      });
    }
    
    return alerts;
  }

  private async processAlerts(portfolioId: string, alerts: RiskAlertDto[]): Promise<void> {
    for (const alert of alerts) {
      // Store alert in Redis for real-time dashboard
      await this.redis.lpush(`alerts:${portfolioId}`, JSON.stringify(alert));
      await this.redis.expire(`alerts:${portfolioId}`, 86400); // 24 hours
      
      // Send to notification channels
      await this.sendNotifications(portfolioId, alert);
      
      // Log critical alerts
      if (alert.severity === 'critical') {
        this.logger.warn(`CRITICAL ALERT for portfolio ${portfolioId}: ${alert.message}`);
      }
    }
  }

  private async sendNotifications(portfolioId: string, alert: RiskAlertDto): Promise<void> {
    const notification = {
      portfolioId,
      alert,
      timestamp: new Date().toISOString(),
    };
    
    // Send to WebSocket clients (if implemented)
    await this.redis.publish('risk-alerts', JSON.stringify(notification));
    
    // Store for email/SMS notifications
    await this.redis.lpush('notifications:pending', JSON.stringify(notification));
  }

  private async updatePortfolioMetrics(portfolioId: string, assessment: any): Promise<void> {
    const metrics = {
      portfolioId,
      riskScore: assessment.riskScore,
      volatility: assessment.volatility,
      var: assessment.quickVar,
      beta: assessment.quickBeta,
      concentration: assessment.concentration,
      timestamp: assessment.timestamp,
    };
    
    // Store latest metrics
    await this.redis.set(`metrics:${portfolioId}`, JSON.stringify(metrics));
    
    // Store in time series for charts
    await this.redis.lpush(`metrics:history:${portfolioId}`, JSON.stringify(metrics));
    await this.redis.ltrim(`metrics:history:${portfolioId}`, 0, 1000); // Keep last 1000 points
  }

  private async checkAutomatedMitigation(portfolioId: string, assessment: any, alerts: RiskAlertDto[]): Promise<void> {
    const criticalAlerts = alerts.filter(alert => alert.severity === 'critical');
    
    if (criticalAlerts.length > 0) {
      // Trigger automated mitigation
      await this.triggerAutomatedMitigation(portfolioId, assessment, criticalAlerts);
    }
  }

  private async triggerAutomatedMitigation(portfolioId: string, assessment: any, alerts: RiskAlertDto[]): Promise<void> {
    const mitigationAction = {
      portfolioId,
      action: 'automated_mitigation',
      triggers: alerts.map(alert => alert.id),
      assessment,
      timestamp: new Date().toISOString(),
    };
    
    // Queue mitigation action
    await this.redis.lpush('mitigation:queue', JSON.stringify(mitigationAction));
    
    this.logger.log(`Automated mitigation triggered for portfolio ${portfolioId}`);
  }

  private async loadActivePortfolios(): Promise<void> {
    try {
      const keys = await this.redis.keys('portfolio:active:*');
      
      for (const key of keys) {
        const portfolioData = await this.redis.get(key);
        if (portfolioData) {
          const portfolioId = key.split(':')[2];
          this.activePortfolios.set(portfolioId, JSON.parse(portfolioData));
        }
      }
      
      this.logger.log(`Loaded ${this.activePortfolios.size} active portfolios for monitoring`);
    } catch (error) {
      this.logger.error('Failed to load active portfolios:', error);
    }
  }

  private async initializeAlertThresholds(): Promise<void> {
    // Set default thresholds for all portfolios
    const defaultThresholds = this.getDefaultThresholds();
    
    for (const portfolioId of this.activePortfolios.keys()) {
      this.alertThresholds.set(portfolioId, defaultThresholds);
    }
  }

  private getDefaultThresholds(): any {
    return {
      varThreshold: 100000, // $100K
      volatilityThreshold: 0.04, // 4%
      concentrationThreshold: 0.3, // 30%
      riskScoreThreshold: 0.8,
    };
  }

  async updateAlertThresholds(portfolioId: string, thresholds: any): Promise<void> {
    this.alertThresholds.set(portfolioId, thresholds);
    await this.redis.set(`thresholds:${portfolioId}`, JSON.stringify(thresholds));
  }

  async getRealTimeMetrics(portfolioId: string): Promise<any> {
    const metrics = await this.redis.get(`metrics:${portfolioId}`);
    return metrics ? JSON.parse(metrics) : null;
  }

  async getRecentAlerts(portfolioId: string, limit: number = 10): Promise<RiskAlertDto[]> {
    const alerts = await this.redis.lrange(`alerts:${portfolioId}`, 0, limit - 1);
    return alerts.map(alert => JSON.parse(alert));
  }

  private generateAlertId(): string {
    return `rt_alert_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  async getMonitoringStatus(): Promise<any> {
    return {
      activePortfolios: this.activePortfolios.size,
      monitoringInterval: this.MONITORING_INTERVAL,
      lastUpdate: new Date().toISOString(),
      portfolios: Array.from(this.activePortfolios.keys()).map(id => ({
        id,
        lastUpdated: this.activePortfolios.get(id)?.lastUpdated,
        monitoringActive: this.activePortfolios.get(id)?.monitoringActive,
      })),
    };
  }
}
