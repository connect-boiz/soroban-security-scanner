import { Test, TestingModule } from '@nestjs/testing';
import { RiskAssessorService } from './risk-assessor.service';
import { Repository } from 'typeorm';
import { ConfigService } from '@nestjs/config';
import { Redis } from '@nestjs/redis';
import { RiskData } from '../entities/risk-data.entity';

describe('RiskAssessorService', () => {
  let service: RiskAssessorService;
  let riskDataRepository: jest.Mocked<Repository<RiskData>>;
  let configService: jest.Mocked<ConfigService>;
  let redis: jest.Mocked<Redis>;

  const mockRiskData = {
    id: 'test-id',
    userId: 'test-user',
    portfolioId: 'test-portfolio',
    riskType: 'market',
    riskScore: 0.75,
    exposure: 100000,
    metrics: {
      var1d: 0.02,
      var10d: 0.06,
      var30d: 0.10,
      expectedShortfall: 0.025,
      beta: 1.2,
      volatility: 0.015,
      correlation: 0.65,
      concentration: 0.3,
    },
    severity: 'high',
    timestamp: new Date(),
  };

  const mockAssessmentDto = {
    userId: 'test-user',
    portfolio: {
      id: 'test-portfolio',
      positions: [
        {
          id: 'position-1',
          type: 'stock',
          size: 100,
          currentPrice: 50,
        },
      ],
      totalValue: 5000,
    },
    confidenceLevel: 0.95,
    timeHorizon: 1,
  };

  beforeEach(async () => {
    const mockRepository = {
      create: jest.fn(),
      save: jest.fn(),
      find: jest.fn(),
      findOne: jest.fn(),
    } as any;

    const mockConfigService = {
      get: jest.fn(),
    } as any;

    const mockRedis = {
      setex: jest.fn(),
      get: jest.fn(),
      set: jest.fn(),
      lpush: jest.fn(),
      lrange: jest.fn(),
      ltrim: jest.fn(),
      expire: jest.fn(),
      del: jest.fn(),
      keys: jest.fn(),
    } as any;

    const module: TestingModule = await Test.createTestingModule({
      providers: [
        RiskAssessorService,
        {
          provide: 'RiskDataRepository',
          useValue: mockRepository,
        },
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

    service = module.get<RiskAssessorService>(RiskAssessorService);
    riskDataRepository = module.get('RiskDataRepository');
    configService = module.get(ConfigService);
    redis = module.get(Redis);
  });

  it('should be defined', () => {
    expect(service).toBeDefined();
  });

  describe('assessRisk', () => {
    it('should perform risk assessment successfully', async () => {
      // Mock repository save
      riskDataRepository.create.mockReturnValue(mockRiskData);
      riskDataRepository.save.mockResolvedValue(mockRiskData);

      // Mock Redis operations
      redis.setex.mockResolvedValue('OK');

      const result = await service.assessRisk(mockAssessmentDto);

      expect(result).toHaveProperty('metrics');
      expect(result).toHaveProperty('alerts');
      expect(result).toHaveProperty('overallScore');
      expect(result).toHaveProperty('riskLevel');
      expect(result.overallScore).toBeGreaterThanOrEqual(0);
      expect(result.overallScore).toBeLessThanOrEqual(1);
      expect(['low', 'medium', 'high', 'critical']).toContain(result.riskLevel);
    });

    it('should generate alerts for high risk scenarios', async () => {
      // Create high-risk portfolio
      const highRiskDto = {
        ...mockAssessmentDto,
        portfolio: {
          ...mockAssessmentDto.portfolio,
          totalValue: 1000000, // Large portfolio
          positions: [
            {
              id: 'position-1',
              type: 'stock',
              size: 10000,
              currentPrice: 100,
            },
          ],
        },
      };

      riskDataRepository.create.mockReturnValue(mockRiskData);
      riskDataRepository.save.mockResolvedValue(mockRiskData);
      redis.setex.mockResolvedValue('OK');

      const result = await service.assessRisk(highRiskDto);

      expect(result.alerts).toBeDefined();
      expect(result.alerts.length).toBeGreaterThan(0);
    });

    it('should handle assessment errors gracefully', async () => {
      // Mock repository error
      riskDataRepository.save.mockRejectedValue(new Error('Database error'));

      await expect(service.assessRisk(mockAssessmentDto)).rejects.toThrow('Risk assessment failed');
    });

    it('should cache assessment results', async () => {
      riskDataRepository.create.mockReturnValue(mockRiskData);
      riskDataRepository.save.mockResolvedValue(mockRiskData);
      redis.setex.mockResolvedValue('OK');

      await service.assessRisk(mockAssessmentDto);

      expect(redis.setex).toHaveBeenCalledWith(
        expect.stringContaining('risk:assessment:'),
        expect.any(Number),
        expect.any(String)
      );
    });
  });

  describe('calculateRiskMetrics', () => {
    it('should calculate valid risk metrics', async () => {
      riskDataRepository.create.mockReturnValue(mockRiskData);
      riskDataRepository.save.mockResolvedValue(mockRiskData);
      redis.setex.mockResolvedValue('OK');

      const result = await service.assessRisk(mockAssessmentDto);

      expect(result.metrics).toHaveProperty('var1d');
      expect(result.metrics).toHaveProperty('var10d');
      expect(result.metrics).toHaveProperty('var30d');
      expect(result.metrics).toHaveProperty('expectedShortfall');
      expect(result.metrics).toHaveProperty('beta');
      expect(result.metrics).toHaveProperty('volatility');
      expect(result.metrics).toHaveProperty('correlation');
      expect(result.metrics).toHaveProperty('concentration');

      // Validate metric ranges
      expect(result.metrics.var1d).toBeGreaterThanOrEqual(0);
      expect(result.metrics.volatility).toBeGreaterThanOrEqual(0);
      expect(result.metrics.concentration).toBeGreaterThanOrEqual(0);
      expect(result.metrics.concentration).toBeLessThanOrEqual(1);
    });
  });

  describe('getHistoricalRiskScores', () => {
    it('should return historical risk scores', async () => {
      const mockHistory = [
        { timestamp: new Date('2026-03-20'), riskScore: 0.5, volatility: 0.01, var1d: 0.015 },
        { timestamp: new Date('2026-03-21'), riskScore: 0.6, volatility: 0.012, var1d: 0.018 },
        { timestamp: new Date('2026-03-22'), riskScore: 0.7, volatility: 0.015, var1d: 0.022 },
      ];

      redis.get.mockResolvedValue(JSON.stringify(mockHistory));

      const result = await service.getHistoricalRiskScores('test-portfolio', 3);

      expect(result).toEqual(mockHistory);
      expect(redis.get).toHaveBeenCalledWith('risk:history:test-portfolio:3');
    });

    it('should return empty array when no history exists', async () => {
      redis.get.mockResolvedValue(null);

      const result = await service.getHistoricalRiskScores('test-portfolio', 3);

      expect(result).toEqual([]);
    });
  });

  describe('Helper Methods', () => {
    it('should calculate portfolio returns correctly', () => {
      const portfolio = {
        positions: [
          { id: '1', type: 'stock', size: 100, currentPrice: 50 },
          { id: '2', type: 'bond', size: 200, currentPrice: 1000 },
        ],
        totalValue: 205000,
      };

      // Access private method through reflection for testing
      const returns = (service as any).calculatePortfolioReturns(portfolio);

      expect(returns).toHaveLength(252); // 252 trading days
      expect(returns.every((r: number) => typeof r === 'number')).toBe(true);
    });

    it('should determine risk level correctly', () => {
      const testCases = [
        { score: 0.2, expected: 'low' },
        { score: 0.4, expected: 'medium' },
        { score: 0.7, expected: 'high' },
        { score: 0.95, expected: 'critical' },
      ];

      testCases.forEach(({ score, expected }) => {
        const level = (service as any).determineRiskLevel(score);
        expect(level).toBe(expected);
      });
    });

    it('should generate valid alert IDs', () => {
      const alertId1 = (service as any).generateAlertId();
      const alertId2 = (service as any).generateAlertId();

      expect(alertId1).toMatch(/^rt_alert_\d+_[a-z0-9]+$/);
      expect(alertId2).toMatch(/^rt_alert_\d+_[a-z0-9]+$/);
      expect(alertId1).not.toBe(alertId2);
    });
  });
});
