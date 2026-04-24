import { Test, TestingModule } from '@nestjs/testing';
import { ConfigService } from '@nestjs/config';
import { FeeCalculatorService, FeeCalculationParams } from '../services/fee-calculator.service';

describe('FeeCalculatorService', () => {
  let service: FeeCalculatorService;
  let configService: ConfigService;

  const mockConfigService = {
    get: jest.fn(),
  };

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [
        FeeCalculatorService,
        {
          provide: ConfigService,
          useValue: mockConfigService,
        },
      ],
    }).compile();

    service = module.get<FeeCalculatorService>(FeeCalculatorService);
    configService = module.get<ConfigService>(ConfigService);

    // Setup default config values
    mockConfigService.get.mockImplementation((key: string, defaultValue?: any) => {
      const config = {
        'FEES_BASE_SCAN_FEE': 100,
        'FEES_BASE_API_CALL_FEE': 1,
        'FEES_BASE_STORAGE_FEE': 10,
        'FEES_BASE_PREMIUM_FEE': 500,
      };
      return config[key] || defaultValue;
    });
  });

  afterEach(() => {
    jest.clearAllMocks();
  });

  describe('calculateFee', () => {
    it('should return base fee for simple scan', () => {
      const params: FeeCalculationParams = {
        type: 'scan',
      };

      const result = service.calculateFee(params);

      expect(result).toBe(100);
    });

    it('should calculate increased fee for large code scans', () => {
      const params: FeeCalculationParams = {
        type: 'scan',
        codeSize: 150000, // > 100KB
      };

      const result = service.calculateFee(params);

      expect(result).toBe(200); // 100 * 2
    });

    it('should calculate increased fee for complex scans', () => {
      const params: FeeCalculationParams = {
        type: 'scan',
        complexity: 9, // > 8
      };

      const result = service.calculateFee(params);

      expect(result).toBe(180); // 100 * 1.8
    });

    it('should calculate increased fee for long processing time', () => {
      const params: FeeCalculationParams = {
        type: 'scan',
        processingTime: 400, // > 5 minutes
      };

      const result = service.calculateFee(params);

      expect(result).toBe(150); // 100 * 1.5
    });

    it('should combine multiple multipliers for scan', () => {
      const params: FeeCalculationParams = {
        type: 'scan',
        codeSize: 150000, // 2x multiplier
        complexity: 9, // 1.8x multiplier
        processingTime: 400, // 1.5x multiplier
      };

      const result = service.calculateFee(params);

      expect(result).toBe(540); // 100 * 2 * 1.8 * 1.5 = 540
    });

    it('should calculate API call fees based on resource usage', () => {
      const params: FeeCalculationParams = {
        type: 'api_call',
        resourceUsage: {
          cpu: 90, // > 80%
          memory: 600, // > 512MB
        },
      };

      const result = service.calculateFee(params);

      expect(result).toBe(2); // 1 * 1.5 * 1.3 = 1.95, rounded to 2
    });

    it('should calculate storage fees based on size', () => {
      const params: FeeCalculationParams = {
        type: 'storage',
        resourceUsage: {
          storageSize: 150 * 1024 * 1024, // 150MB
        },
      };

      const result = service.calculateFee(params);

      expect(result).toBe(20); // 10 * 2
    });

    it('should calculate premium feature fees based on complexity', () => {
      const params: FeeCalculationParams = {
        type: 'premium_feature',
        complexity: 10,
      };

      const result = service.calculateFee(params);

      expect(result).toBe(1000); // 500 * (10/5) = 1000
    });

    it('should return 0 for unknown operation types', () => {
      const params: FeeCalculationParams = {
        type: 'unknown' as any,
      };

      const result = service.calculateFee(params);

      expect(result).toBe(0);
    });
  });

  describe('canAffordOperation', () => {
    it('should return true when user has sufficient balance', () => {
      const params: FeeCalculationParams = {
        type: 'scan',
      };

      const result = service.canAffordOperation(200, params);

      expect(result).toBe(true);
    });

    it('should return false when user has insufficient balance', () => {
      const params: FeeCalculationParams = {
        type: 'scan',
        codeSize: 150000, // Fee = 200
      };

      const result = service.canAffordOperation(100, params);

      expect(result).toBe(false);
    });
  });

  describe('getEstimatedFee', () => {
    it('should return detailed fee breakdown', () => {
      const params: FeeCalculationParams = {
        type: 'scan',
        codeSize: 150000, // 2x multiplier
      };

      const result = service.getEstimatedFee(params);

      expect(result).toEqual({
        estimatedFee: 200,
        breakdown: {
          baseFee: 100,
          multiplier: 2,
          totalFee: 200,
        },
      });
    });

    it('should handle simple scan fee estimation', () => {
      const params: FeeCalculationParams = {
        type: 'scan',
      };

      const result = service.getEstimatedFee(params);

      expect(result).toEqual({
        estimatedFee: 100,
        breakdown: {
          baseFee: 100,
          multiplier: 1,
          totalFee: 100,
        },
      });
    });

    it('should handle API call fee estimation', () => {
      const params: FeeCalculationParams = {
        type: 'api_call',
      };

      const result = service.getEstimatedFee(params);

      expect(result).toEqual({
        estimatedFee: 1,
        breakdown: {
          baseFee: 1,
          multiplier: 1,
          totalFee: 1,
        },
      });
    });

    it('should handle storage fee estimation', () => {
      const params: FeeCalculationParams = {
        type: 'storage',
      };

      const result = service.getEstimatedFee(params);

      expect(result).toEqual({
        estimatedFee: 10,
        breakdown: {
          baseFee: 10,
          multiplier: 1,
          totalFee: 10,
        },
      });
    });

    it('should handle premium feature fee estimation', () => {
      const params: FeeCalculationParams = {
        type: 'premium_feature',
      };

      const result = service.getEstimatedFee(params);

      expect(result).toEqual({
        estimatedFee: 500,
        breakdown: {
          baseFee: 500,
          multiplier: 1,
          totalFee: 500,
        },
      });
    });
  });

  describe('custom configuration', () => {
    it('should use custom base fees from config', () => {
      mockConfigService.get.mockImplementation((key: string) => {
        const customConfig = {
          'FEES_BASE_SCAN_FEE': 200,
          'FEES_BASE_API_CALL_FEE': 2,
          'FEES_BASE_STORAGE_FEE': 20,
          'FEES_BASE_PREMIUM_FEE': 1000,
        };
        return customConfig[key];
      });

      const scanParams: FeeCalculationParams = { type: 'scan' };
      const apiParams: FeeCalculationParams = { type: 'api_call' };
      const storageParams: FeeCalculationParams = { type: 'storage' };
      const premiumParams: FeeCalculationParams = { type: 'premium_feature' };

      expect(service.calculateFee(scanParams)).toBe(200);
      expect(service.calculateFee(apiParams)).toBe(2);
      expect(service.calculateFee(storageParams)).toBe(20);
      expect(service.calculateFee(premiumParams)).toBe(1000);
    });

    it('should use default values when config is missing', () => {
      mockConfigService.get.mockReturnValue(undefined);

      const params: FeeCalculationParams = {
        type: 'scan',
      };

      const result = service.calculateFee(params);

      expect(result).toBe(0); // Default base fee is 0 when not configured
    });
  });
});
