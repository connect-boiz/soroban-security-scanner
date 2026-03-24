import { Injectable, Logger } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { Redis } from '@nestjs/redis';

@Injectable()
export class VarCalculatorService {
  private readonly logger = new Logger(VarCalculatorService.name);
  private readonly CONFIDENCE_LEVELS = [0.90, 0.95, 0.99];
  private readonly TIME_HORIZONS = [1, 10, 30]; // days

  constructor(
    private readonly configService: ConfigService,
    private readonly redis: Redis,
  ) {}

  async calculateVar(
    portfolioReturns: number[],
    confidenceLevel: number = 0.95,
    timeHorizon: number = 1
  ): Promise<{
    var: number;
    expectedShortfall: number;
    methodology: string;
    assumptions: string[];
    accuracy: number;
  }> {
    const startTime = Date.now();
    
    try {
      // Validate inputs
      this.validateInputs(portfolioReturns, confidenceLevel, timeHorizon);
      
      // Calculate VaR using multiple methods
      const historicalVar = this.calculateHistoricalVar(portfolioReturns, confidenceLevel);
      const parametricVar = this.calculateParametricVar(portfolioReturns, confidenceLevel);
      const monteCarloVar = this.calculateMonteCarloVar(portfolioReturns, confidenceLevel);
      
      // Select best method based on data characteristics
      const bestMethod = this.selectBestMethod(portfolioReturns, {
        historical: historicalVar,
        parametric: parametricVar,
        monteCarlo: monteCarloVar,
      });
      
      // Calculate Expected Shortfall
      const expectedShortfall = this.calculateExpectedShortfall(portfolioReturns, confidenceLevel);
      
      // Scale by time horizon
      const scaledVar = bestMethod.value * Math.sqrt(timeHorizon);
      const scaledES = expectedShortfall * Math.sqrt(timeHorizon);
      
      // Calculate accuracy estimate
      const accuracy = this.calculateAccuracyEstimate(portfolioReturns, bestMethod.method);
      
      const result = {
        var: scaledVar,
        expectedShortfall: scaledES,
        methodology: bestMethod.method,
        assumptions: this.getMethodAssumptions(bestMethod.method),
        accuracy,
      };
      
      // Cache result
      await this.cacheVarResult(portfolioReturns.length, confidenceLevel, timeHorizon, result);
      
      const processingTime = Date.now() - startTime;
      this.logger.log(`VaR calculated in ${processingTime}ms using ${bestMethod.method} method`);
      
      return result;
    } catch (error) {
      this.logger.error('VaR calculation failed:', error);
      throw new Error('VaR calculation failed');
    }
  }

  async calculateComponentVar(
    portfolioPositions: any[],
    portfolioReturns: number[],
    confidenceLevel: number = 0.95
  ): Promise<{
    totalVar: number;
    componentVars: Array<{
      positionId: string;
      positionName: string;
      componentVar: number;
      marginalVar: number;
      percentageContribution: number;
    }>;
  }> {
    try {
      // Calculate total portfolio VaR
      const totalVarResult = await this.calculateVar(portfolioReturns, confidenceLevel, 1);
      const totalVar = totalVarResult.var;
      
      // Calculate component VaR for each position
      const componentVars = [];
      
      for (const position of portfolioPositions) {
        const positionReturns = this.calculatePositionReturns(position, portfolioReturns);
        const positionVar = await this.calculateVar(positionReturns, confidenceLevel, 1);
        
        // Calculate marginal VaR (approximate)
        const marginalVar = this.calculateMarginalVar(position, portfolioReturns, confidenceLevel);
        
        // Calculate percentage contribution
        const percentageContribution = (positionVar.var / totalVar) * 100;
        
        componentVars.push({
          positionId: position.id,
          positionName: position.name || position.id,
          componentVar: positionVar.var,
          marginalVar,
          percentageContribution,
        });
      }
      
      // Sort by contribution
      componentVars.sort((a, b) => b.percentageContribution - a.percentageContribution);
      
      return {
        totalVar,
        componentVars,
      };
    } catch (error) {
      this.logger.error('Component VaR calculation failed:', error);
      throw new Error('Component VaR calculation failed');
    }
  }

  async calculateConditionalVar(
    portfolioReturns: number[],
    marketConditions: any,
    confidenceLevel: number = 0.95
  ): Promise<{
    baseVar: number;
    conditionalVar: number;
    adjustmentFactor: number;
    marketRegime: string;
  }> {
    try {
      // Calculate base VaR
      const baseVarResult = await this.calculateVar(portfolioReturns, confidenceLevel, 1);
      const baseVar = baseVarResult.var;
      
      // Identify market regime
      const marketRegime = this.identifyMarketRegime(marketConditions);
      
      // Calculate conditional VaR based on market regime
      const conditionalVar = this.calculateConditionalVarByRegime(
        portfolioReturns,
        marketRegime,
        confidenceLevel
      );
      
      // Calculate adjustment factor
      const adjustmentFactor = conditionalVar / baseVar;
      
      return {
        baseVar,
        conditionalVar,
        adjustmentFactor,
        marketRegime,
      };
    } catch (error) {
      this.logger.error('Conditional VaR calculation failed:', error);
      throw new Error('Conditional VaR calculation failed');
    }
  }

  private validateInputs(returns: number[], confidenceLevel: number, timeHorizon: number): void {
    if (returns.length < 30) {
      throw new Error('Insufficient data: need at least 30 return observations');
    }
    
    if (!this.CONFIDENCE_LEVELS.includes(confidenceLevel)) {
      throw new Error(`Invalid confidence level: must be one of ${this.CONFIDENCE_LEVELS}`);
    }
    
    if (!this.TIME_HORIZONS.includes(timeHorizon)) {
      throw new Error(`Invalid time horizon: must be one of ${this.TIME_HORIZONS}`);
    }
  }

  private calculateHistoricalVar(returns: number[], confidenceLevel: number): number {
    const sortedReturns = returns.sort((a, b) => a - b);
    const index = Math.floor((1 - confidenceLevel) * sortedReturns.length);
    return Math.abs(sortedReturns[index] || 0);
  }

  private calculateParametricVar(returns: number[], confidenceLevel: number): number {
    const mean = returns.reduce((sum, ret) => sum + ret, 0) / returns.length;
    const variance = returns.reduce((sum, ret) => sum + Math.pow(ret - mean, 2), 0) / returns.length;
    const stdDev = Math.sqrt(variance);
    
    // Get z-score for confidence level
    const zScore = this.getZScore(confidenceLevel);
    
    return Math.abs(mean - zScore * stdDev);
  }

  private calculateMonteCarloVar(returns: number[], confidenceLevel: number): number {
    const numSimulations = 10000;
    const mean = returns.reduce((sum, ret) => sum + ret, 0) / returns.length;
    const variance = returns.reduce((sum, ret) => sum + Math.pow(ret - mean, 2), 0) / returns.length;
    const stdDev = Math.sqrt(variance);
    
    // Generate Monte Carlo simulations
    const simulatedReturns = [];
    for (let i = 0; i < numSimulations; i++) {
      const randomReturn = this.generateNormalRandom(mean, stdDev);
      simulatedReturns.push(randomReturn);
    }
    
    // Calculate VaR from simulated returns
    const sortedReturns = simulatedReturns.sort((a, b) => a - b);
    const index = Math.floor((1 - confidenceLevel) * sortedReturns.length);
    return Math.abs(sortedReturns[index] || 0);
  }

  private calculateExpectedShortfall(returns: number[], confidenceLevel: number): number {
    const sortedReturns = returns.sort((a, b) => a - b);
    const cutoffIndex = Math.floor((1 - confidenceLevel) * sortedReturns.length);
    const tailReturns = sortedReturns.slice(0, cutoffIndex);
    
    if (tailReturns.length === 0) return 0;
    
    const averageLoss = tailReturns.reduce((sum, ret) => sum + ret, 0) / tailReturns.length;
    return Math.abs(averageLoss);
  }

  private selectBestMethod(returns: number[], methods: any): any {
    // Check for normality
    const isNormal = this.checkNormality(returns);
    
    // Check data size
    const hasEnoughData = returns.length >= 250;
    
    // Select method based on data characteristics
    if (isNormal && hasEnoughData) {
      return { method: 'parametric', value: methods.parametric };
    } else if (hasEnoughData) {
      return { method: 'historical', value: methods.historical };
    } else {
      return { method: 'monteCarlo', value: methods.monteCarlo };
    }
  }

  private checkNormality(returns: number[]): boolean {
    // Simple normality test using skewness and kurtosis
    const mean = returns.reduce((sum, ret) => sum + ret, 0) / returns.length;
    const variance = returns.reduce((sum, ret) => sum + Math.pow(ret - mean, 2), 0) / returns.length;
    const stdDev = Math.sqrt(variance);
    
    // Calculate skewness
    const skewness = returns.reduce((sum, ret) => {
      return sum + Math.pow((ret - mean) / stdDev, 3);
    }, 0) / returns.length;
    
    // Calculate kurtosis
    const kurtosis = returns.reduce((sum, ret) => {
      return sum + Math.pow((ret - mean) / stdDev, 4);
    }, 0) / returns.length - 3;
    
    // Consider normal if skewness and kurtosis are within reasonable bounds
    return Math.abs(skewness) < 0.5 && Math.abs(kurtosis) < 1.0;
  }

  private calculatePositionReturns(position: any, portfolioReturns: number[]): number[] {
    // Simplified position returns calculation
    const positionWeight = position.weight || 0.1;
    const positionBeta = position.beta || 1.0;
    
    return portfolioReturns.map(ret => ret * positionWeight * positionBeta);
  }

  private calculateMarginalVar(position: any, portfolioReturns: number[], confidenceLevel: number): number {
    // Simplified marginal VaR calculation
    const positionWeight = position.weight || 0.1;
    const portfolioVar = this.calculateHistoricalVar(portfolioReturns, confidenceLevel);
    
    return portfolioVar * positionWeight * 1.1; // Assume 10% marginal impact
  }

  private identifyMarketRegime(marketConditions: any): string {
    const volatility = marketConditions.volatility || 0.02;
    const trend = marketConditions.trend || 0;
    
    if (volatility > 0.04) {
      return 'high_volatility';
    } else if (trend < -0.05) {
      return 'bear_market';
    } else if (trend > 0.05) {
      return 'bull_market';
    } else {
      return 'normal';
    }
  }

  private calculateConditionalVarByRegime(
    returns: number[],
    regime: string,
    confidenceLevel: number
  ): number {
    const baseVar = this.calculateHistoricalVar(returns, confidenceLevel);
    
    // Apply regime-specific adjustments
    const regimeAdjustments = {
      normal: 1.0,
      bull_market: 0.8,
      bear_market: 1.5,
      high_volatility: 1.8,
    };
    
    const adjustment = regimeAdjustments[regime] || 1.0;
    return baseVar * adjustment;
  }

  private getZScore(confidenceLevel: number): number {
    const zScores = {
      0.90: 1.28,
      0.95: 1.645,
      0.99: 2.33,
    };
    
    return zScores[confidenceLevel] || 1.645;
  }

  private generateNormalRandom(mean: number, stdDev: number): number {
    // Box-Muller transform
    const u1 = Math.random();
    const u2 = Math.random();
    const normalRandom = Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
    
    return mean + normalRandom * stdDev;
  }

  private calculateAccuracyEstimate(returns: number[], method: string): number {
    const sampleSize = returns.length;
    
    // Base accuracy depends on sample size and method
    const baseAccuracy = {
      historical: Math.min(sampleSize / 1000, 0.95),
      parametric: Math.min(sampleSize / 500, 0.90),
      monteCarlo: 0.85, // Monte Carlo has fixed accuracy due to simulation
    };
    
    return baseAccuracy[method] || 0.85;
  }

  private getMethodAssumptions(method: string): string[] {
    const assumptions = {
      historical: [
        'Historical patterns will repeat',
        'Sufficient historical data available',
        'Market conditions are representative',
      ],
      parametric: [
        'Returns are normally distributed',
        'Volatility is constant over time',
        'Linear relationships between assets',
      ],
      monteCarlo: [
        'Statistical properties are stable',
        'Sufficient random sampling',
        'Model captures key risk factors',
      ],
    };
    
    return assumptions[method] || ['Standard risk assumptions apply'];
  }

  private async cacheVarResult(
    dataSize: number,
    confidenceLevel: number,
    timeHorizon: number,
    result: any
  ): Promise<void> {
    const cacheKey = `var:${dataSize}:${confidenceLevel}:${timeHorizon}`;
    await this.redis.setex(cacheKey, 3600, JSON.stringify(result)); // 1 hour cache
  }

  async backtestVar(
    portfolioReturns: number[],
    confidenceLevel: number = 0.95,
    windowSize: number = 252
  ): Promise<{
    violations: number;
    expectedViolations: number;
    violationRate: number;
    kupiecPValue: number;
    christoffersenPValue: number;
    modelAccuracy: string;
  }> {
    try {
      const violations = [];
      const totalObservations = portfolioReturns.length - windowSize;
      
      for (let i = windowSize; i < portfolioReturns.length; i++) {
        const windowReturns = portfolioReturns.slice(i - windowSize, i);
        const varResult = await this.calculateVar(windowReturns, confidenceLevel, 1);
        const actualReturn = portfolioReturns[i];
        
        // Check if violation occurred
        if (actualReturn < -varResult.var) {
          violations.push({
            date: i,
            actualReturn,
            predictedVar: varResult.var,
            violation: actualReturn + varResult.var,
          });
        }
      }
    
      const violationCount = violations.length;
      const expectedViolations = totalObservations * (1 - confidenceLevel);
      const violationRate = violationCount / totalObservations;
      
      // Kupiec test for unconditional coverage
      const kupiecPValue = this.calculateKupiecPValue(violationCount, totalObservations, confidenceLevel);
      
      // Christoffersen test for independence
      const christoffersenPValue = this.calculateChristoffersenPValue(violations, totalObservations);
      
      // Determine model accuracy
      let modelAccuracy = 'poor';
      if (violationRate > 0.03 && violationRate < 0.07 && kupiecPValue > 0.05) {
        modelAccuracy = 'excellent';
      } else if (violationRate > 0.02 && violationRate < 0.08 && kupiecPValue > 0.01) {
        modelAccuracy = 'good';
      } else if (violationRate > 0.01 && violationRate < 0.09) {
        modelAccuracy = 'acceptable';
      }
      
      return {
        violations: violationCount,
        expectedViolations: Math.round(expectedViolations),
        violationRate,
        kupiecPValue,
        christoffersenPValue,
        modelAccuracy,
      };
    } catch (error) {
      this.logger.error('VaR backtest failed:', error);
      throw new Error('VaR backtest failed');
    }
  }

  private calculateKupiecPValue(violations: number, observations: number, confidenceLevel: number): number {
    const expectedViolations = observations * (1 - confidenceLevel);
    
    if (violations === 0) return 1.0;
    
    // Likelihood ratio test
    const lr = 2 * (
      violations * Math.log(violations / expectedViolations) +
      (observations - violations) * Math.log((observations - violations) / (observations - expectedViolations))
    );
    
    // Approximate p-value (simplified)
    return Math.exp(-lr / 2);
  }

  private calculateChristoffersenPValue(violations: any[], observations: number): number {
    // Simplified independence test
    if (violations.length < 2) return 1.0;
    
    // Check for clustering of violations
    let clusteredViolations = 0;
    for (let i = 1; i < violations.length; i++) {
      if (violations[i].date - violations[i - 1].date < 5) { // Within 5 days
        clusteredViolations++;
      }
    }
    
    const clusteringRate = clusteredViolations / (violations.length - 1);
    
    // Return high p-value if no clustering, low if clustering exists
    return clusteringRate < 0.1 ? 0.8 : 0.2;
  }

  async getVarCalculationHistory(portfolioId: string): Promise<any[]> {
    const cacheKey = `var:history:${portfolioId}`;
    const cached = await this.redis.get(cacheKey);
    
    if (cached) {
      return JSON.parse(cached);
    }
    
    return [];
  }
}
