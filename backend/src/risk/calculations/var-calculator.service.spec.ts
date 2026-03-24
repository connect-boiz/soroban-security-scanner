import { Test, TestingModule } from '@nestjs/testing';
import { VarCalculatorService } from './var-calculator.service';
import { ConfigService } from '@nestjs/config';
import { Redis } from '@nestjs/redis';

describe('VarCalculatorService', () => {
  let service: VarCalculatorService;
  let configService: jest.Mocked<ConfigService>;
  let redis: jest.Mocked<Redis>;

  const mockReturns = Array.from({ length: 252 }, () => 
    0.0008 + (Math.random() - 0.5) * 0.02 // Generate realistic returns
  );

  beforeEach(async () => {
    const mockConfigService = {
      get: jest.fn(),
    } as any;

    const mockRedis = {
      setex: jest.fn(),
      get: jest.fn(),
    } as any;

    const module: TestingModule = await Test.createTestingModule({
      providers: [
        VarCalculatorService,
        {
          provide: ConfigService,
          useValue: mockConfigService,
        },
        {
          provide: Redis,
          useValue: mockRedis,
        },
      ],
    }).compile();

    service = module.get<VarCalculatorService>(VarCalculatorService);
    configService = module.get(ConfigService);
    redis = module.get(Redis);
  });

  it('should be defined', () => {
    expect(service).toBeDefined();
  });

  describe('calculateVar', () => {
    it('should calculate VaR with valid parameters', async () => {
      redis.setex.mockResolvedValue('OK');

      const result = await service.calculateVar(mockReturns, 0.95, 1);

      expect(result).toHaveProperty('var');
      expect(result).toHaveProperty('expectedShortfall');
      expect(result).toHaveProperty('methodology');
      expect(result).toHaveProperty('assumptions');
      expect(result).toHaveProperty('accuracy');

      expect(result.var).toBeGreaterThanOrEqual(0);
      expect(result.expectedShortfall).toBeGreaterThanOrEqual(0);
      expect(result.accuracy).toBeGreaterThanOrEqual(0);
      expect(result.accuracy).toBeLessThanOrEqual(1);
      expect(['historical', 'parametric', 'monteCarlo']).toContain(result.methodology);
    });

    it('should handle different confidence levels', async () => {
      redis.setex.mockResolvedValue('OK');

      const result95 = await service.calculateVar(mockReturns, 0.95, 1);
      const result99 = await service.calculateVar(mockReturns, 0.99, 1);

      // 99% VaR should be higher than 95% VaR
      expect(result99.var).toBeGreaterThan(result95.var);
    });

    it('should handle different time horizons', async () => {
      redis.setex.mockResolvedValue('OK');

      const result1d = await service.calculateVar(mockReturns, 0.95, 1);
      const result10d = await service.calculateVar(mockReturns, 0.95, 10);

      // 10-day VaR should be higher than 1-day VaR (square root of time)
      expect(result10d.var).toBeGreaterThan(result1d.var);
    });

    it('should validate input parameters', async () => {
      // Test insufficient data
      await expect(service.calculateVar([1, 2, 3], 0.95, 1))
        .rejects.toThrow('Insufficient data');

      // Test invalid confidence level
      await expect(service.calculateVar(mockReturns, 0.5, 1))
        .rejects.toThrow('Invalid confidence level');

      // Test invalid time horizon
      await expect(service.calculateVar(mockReturns, 0.95, 50))
        .rejects.toThrow('Invalid time horizon');
    });

    it('should cache results', async () => {
      redis.setex.mockResolvedValue('OK');

      await service.calculateVar(mockReturns, 0.95, 1);

      expect(redis.setex).toHaveBeenCalledWith(
        expect.stringContaining('var:'),
        expect.any(Number),
        expect.any(String)
      );
    });
  });

  describe('calculateComponentVar', () => {
    const mockPositions = [
      { id: '1', name: 'Stock A', weight: 0.6, beta: 1.2 },
      { id: '2', name: 'Stock B', weight: 0.4, beta: 0.8 },
    ];

    it('should calculate component VaR for positions', async () => {
      redis.setex.mockResolvedValue('OK');

      const result = await service.calculateComponentVar(mockPositions, mockReturns, 0.95);

      expect(result).toHaveProperty('totalVar');
      expect(result).toHaveProperty('componentVars');
      expect(result.componentVars).toHaveLength(2);

      result.componentVars.forEach((component, index) => {
        expect(component).toHaveProperty('positionId', mockPositions[index].id);
        expect(component).toHaveProperty('positionName', mockPositions[index].name);
        expect(component).toHaveProperty('componentVar');
        expect(component).toHaveProperty('marginalVar');
        expect(component).toHaveProperty('percentageContribution');
      });
    });

    it('should ensure component contributions sum to total', async () => {
      redis.setex.mockResolvedValue('OK');

      const result = await service.calculateComponentVar(mockPositions, mockReturns, 0.95);

      const totalContribution = result.componentVars.reduce(
        (sum, comp) => sum + comp.percentageContribution, 0
      );

      expect(totalContribution).toBeCloseTo(100, 1); // Should sum to 100%
    });
  });

  describe('calculateConditionalVar', () => {
    const mockMarketConditions = {
      volatility: 0.025,
      trend: -0.02,
    };

    it('should calculate conditional VaR based on market conditions', async () => {
      redis.setex.mockResolvedValue('OK');

      const result = await service.calculateConditionalVar(mockReturns, mockMarketConditions, 0.95);

      expect(result).toHaveProperty('baseVar');
      expect(result).toHaveProperty('conditionalVar');
      expect(result).toHaveProperty('adjustmentFactor');
      expect(result).toHaveProperty('marketRegime');

      expect(result.baseVar).toBeGreaterThanOrEqual(0);
      expect(result.conditionalVar).toBeGreaterThanOrEqual(0);
      expect(result.adjustmentFactor).toBeGreaterThanOrEqual(0);
      expect(['normal', 'bull_market', 'bear_market', 'high_volatility']).toContain(result.marketRegime);
    });

    it('should adjust VaR based on market volatility', async () => {
      redis.setex.mockResolvedValue('OK');

      const normalMarket = { volatility: 0.015, trend: 0 };
      const volatileMarket = { volatility: 0.04, trend: 0 };

      const normalResult = await service.calculateConditionalVar(mockReturns, normalMarket, 0.95);
      const volatileResult = await service.calculateConditionalVar(mockReturns, volatileMarket, 0.95);

      // High volatility market should have higher conditional VaR
      expect(volatileResult.conditionalVar).toBeGreaterThan(normalResult.conditionalVar);
      expect(volatileResult.adjustmentFactor).toBeGreaterThan(normalResult.adjustmentFactor);
    });
  });

  describe('backtestVar', () => {
    it('should perform VaR backtesting', async () => {
      // Create returns with some violations
      const backtestReturns = Array.from({ length: 300 }, (_, i) => {
        // Add some known violations
        if (i % 50 === 0) return -0.05; // 5% loss (should violate 95% VaR)
        return 0.0008 + (Math.random() - 0.5) * 0.02;
      });

      const result = await service.backtestVar(backtestReturns, 0.95, 252);

      expect(result).toHaveProperty('violations');
      expect(result).toHaveProperty('expectedViolations');
      expect(result).toHaveProperty('violationRate');
      expect(result).toHaveProperty('kupiecPValue');
      expect(result).toHaveProperty('christoffersenPValue');
      expect(result).toHaveProperty('modelAccuracy');

      expect(result.violations).toBeGreaterThanOrEqual(0);
      expect(result.expectedViolations).toBeCloseTo(13, 0); // 5% of 252
      expect(result.violationRate).toBeGreaterThanOrEqual(0);
      expect(result.violationRate).toBeLessThanOrEqual(1);
      expect(['excellent', 'good', 'acceptable', 'poor']).toContain(result.modelAccuracy);
    });

    it('should handle insufficient backtesting data', async () => {
      const shortReturns = Array(100).fill(0.001);

      await expect(service.backtestVar(shortReturns, 0.95, 252))
        .rejects.toThrow('Insufficient data');
    });
  });

  describe('Helper Methods', () => {
    it('should calculate historical VaR correctly', () => {
      const testReturns = [-0.05, -0.03, -0.02, -0.01, 0.01, 0.02, 0.03];
      const result = (service as any).calculateHistoricalVar(testReturns, 0.95);

      expect(result).toBeCloseTo(0.05, 2); // 95% VaR should be 5%
    });

    it('should calculate parametric VaR correctly', () => {
      const testReturns = Array.from({ length: 100 }, () => (Math.random() - 0.5) * 0.02);
      const result = (service as any).calculateParametricVar(testReturns, 0.95);

      expect(result).toBeGreaterThanOrEqual(0);
    });

    it('should generate normal random numbers', () => {
      const randomNumbers = Array.from({ length: 1000 }, () => 
        (service as any).generateNormalRandom(0, 1)
      );

      const mean = randomNumbers.reduce((sum, x) => sum + x, 0) / randomNumbers.length;
      const variance = randomNumbers.reduce((sum, x) => sum + Math.pow(x - mean, 2), 0) / randomNumbers.length;

      // Should be approximately normal (mean ≈ 0, variance ≈ 1)
      expect(Math.abs(mean)).toBeLessThan(0.1);
      expect(Math.abs(variance - 1)).toBeLessThan(0.2);
    });

    it('should check normality correctly', () => {
      // Normal distribution
      const normalReturns = Array.from({ length: 1000 }, () => 
        (service as any).generateNormalRandom(0, 0.02)
      );
      expect((service as any).checkNormality(normalReturns)).toBe(true);

      // Non-normal distribution (extreme values)
      const nonNormalReturns = Array.from({ length: 100 }, (_, i) => 
        i < 5 ? -0.1 : 0.001 // Few extreme losses
      );
      expect((service as any).checkNormality(nonNormalReturns)).toBe(false);
    });

    it('should get correct z-scores', () => {
      expect((service as any).getZScore(0.90)).toBeCloseTo(1.28, 2);
      expect((service as any).getZScore(0.95)).toBeCloseTo(1.645, 3);
      expect((service as any).getZScore(0.99)).toBeCloseTo(2.33, 2);
    });

    it('should calculate expected shortfall correctly', () => {
      const testReturns = [-0.08, -0.06, -0.05, -0.04, -0.03, -0.02, -0.01, 0.01];
      const result = (service as any).calculateExpectedShortfall(testReturns, 0.95);

      // Expected shortfall should be average of worst 5% (0.4 returns ≈ 2 returns)
      // Worst returns are -0.08 and -0.06, average is -0.07
      expect(result).toBeCloseTo(0.07, 2);
    });
  });

  describe('Error Handling', () => {
    it('should handle calculation errors gracefully', async () => {
      // Test with invalid returns (NaN values)
      const invalidReturns = [NaN, Infinity, -Infinity];

      await expect(service.calculateVar(invalidReturns, 0.95, 1))
        .rejects.toThrow('VaR calculation failed');
    });

    it('should handle edge cases', async () => {
      // Test with all positive returns (no risk)
      const positiveReturns = Array.from({ length: 100 }, () => 0.01);
      
      redis.setex.mockResolvedValue('OK');
      
      const result = await service.calculateVar(positiveReturns, 0.95, 1);
      
      expect(result.var).toBeGreaterThanOrEqual(0);
      expect(result.expectedShortfall).toBeGreaterThanOrEqual(0);
    });
  });
});
