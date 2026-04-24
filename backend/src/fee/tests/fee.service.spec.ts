import { Test, TestingModule } from '@nestjs/testing';
import { getRepositoryToken } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { FeeService } from '../services/fee.service';
import { FeeCalculatorService } from '../services/fee-calculator.service';
import { Fee } from '../entities/fee.entity';
import { UserBalance } from '../entities/user-balance.entity';
import { CreateFeeDto } from '../dto/create-fee.dto';
import { AddBalanceDto } from '../dto/add-balance.dto';

describe('FeeService', () => {
  let service: FeeService;
  let feeRepository: Repository<Fee>;
  let balanceRepository: Repository<UserBalance>;
  let feeCalculator: FeeCalculatorService;

  const mockFeeRepository = {
    create: jest.fn(),
    save: jest.fn(),
    findOne: jest.fn(),
    findAndCount: jest.fn(),
    createQueryBuilder: jest.fn(),
  };

  const mockBalanceRepository = {
    findOne: jest.fn(),
    save: jest.fn(),
    create: jest.fn(),
  };

  const mockFeeCalculator = {
    calculateFee: jest.fn(),
    canAffordOperation: jest.fn(),
    getEstimatedFee: jest.fn(),
  };

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [
        FeeService,
        {
          provide: getRepositoryToken(Fee),
          useValue: mockFeeRepository,
        },
        {
          provide: getRepositoryToken(UserBalance),
          useValue: mockBalanceRepository,
        },
        {
          provide: FeeCalculatorService,
          useValue: mockFeeCalculator,
        },
      ],
    }).compile();

    service = module.get<FeeService>(FeeService);
    feeRepository = module.get<Repository<Fee>>(getRepositoryToken(Fee));
    balanceRepository = module.get<Repository<UserBalance>>(getRepositoryToken(UserBalance));
    feeCalculator = module.get<FeeCalculatorService>(FeeCalculatorService);
  });

  afterEach(() => {
    jest.clearAllMocks();
  });

  describe('createAndChargeFee', () => {
    const userId = 'user-123';
    const createFeeDto: CreateFeeDto = {
      type: 'scan',
      amount: 100,
      description: 'Test scan fee',
    };

    it('should successfully create and charge a fee when user has sufficient balance', async () => {
      const mockBalance = {
        balance: 200,
        totalSpent: 0,
        totalDeposited: 200,
        totalRefunded: 0,
      };

      const mockFee = {
        id: 'fee-123',
        userId,
        type: 'scan',
        amount: 100,
        status: 'paid',
        paidAt: new Date(),
      };

      mockBalanceRepository.findOne.mockResolvedValue(mockBalance);
      mockFeeCalculator.calculateFee.mockReturnValue(100);
      mockFeeRepository.create.mockReturnValue(mockFee);
      mockFeeRepository.save.mockResolvedValue(mockFee);
      mockBalanceRepository.save.mockResolvedValue({ ...mockBalance, balance: 100, totalSpent: 100 });

      const result = await service.createAndChargeFee(createFeeDto, userId);

      expect(result).toEqual(mockFee);
      expect(feeCalculator.calculateFee).toHaveBeenCalled();
      expect(feeRepository.create).toHaveBeenCalledWith({
        ...createFeeDto,
        userId,
        amount: 100,
        status: 'paid',
        paidAt: expect.any(Date),
      });
    });

    it('should create new balance if none exists', async () => {
      const mockBalance = {
        balance: 100,
        totalSpent: 0,
        totalDeposited: 0,
        totalRefunded: 0,
        creditLimit: 1000,
        usageStats: {
          totalScans: 0,
          totalApiCalls: 0,
          totalStorageUsed: 0,
          lastResetDate: new Date().toISOString(),
        },
      };

      mockBalanceRepository.findOne.mockResolvedValue(null);
      mockBalanceRepository.create.mockReturnValue(mockBalance);
      mockBalanceRepository.save.mockResolvedValue(mockBalance);
      mockFeeCalculator.calculateFee.mockReturnValue(100);

      const mockFee = {
        id: 'fee-123',
        userId,
        type: 'scan',
        amount: 100,
        status: 'paid',
        paidAt: new Date(),
      };

      mockFeeRepository.create.mockReturnValue(mockFee);
      mockFeeRepository.save.mockResolvedValue(mockFee);

      await service.createAndChargeFee(createFeeDto, userId);

      expect(mockBalanceRepository.create).toHaveBeenCalledWith({
        userId,
        balance: 0,
        totalSpent: 0,
        totalDeposited: 0,
        totalRefunded: 0,
        creditLimit: 1000,
        usageStats: {
          totalScans: 0,
          totalApiCalls: 0,
          totalStorageUsed: 0,
          lastResetDate: expect.any(String),
        },
      });
    });

    it('should throw error when user has insufficient balance', async () => {
      const mockBalance = {
        balance: 50,
        totalSpent: 0,
        totalDeposited: 50,
      };

      mockBalanceRepository.findOne.mockResolvedValue(mockBalance);
      mockFeeCalculator.calculateFee.mockReturnValue(100);

      await expect(service.createAndChargeFee(createFeeDto, userId)).rejects.toThrow(
        'Insufficient balance. Required: 100, Available: 50'
      );
    });
  });

  describe('addBalance', () => {
    const userId = 'user-123';
    const addBalanceDto: AddBalanceDto = {
      amount: 500,
      description: 'Test deposit',
    };

    it('should successfully add balance to existing user', async () => {
      const mockBalance = {
        balance: 100,
        totalSpent: 0,
        totalDeposited: 100,
        totalRefunded: 0,
      };

      const updatedBalance = {
        ...mockBalance,
        balance: 600,
        totalDeposited: 600,
      };

      mockBalanceRepository.findOne.mockResolvedValue(mockBalance);
      mockBalanceRepository.save.mockResolvedValue(updatedBalance);
      mockFeeRepository.save.mockResolvedValue({});

      const result = await service.addBalance(userId, addBalanceDto);

      expect(result).toEqual(updatedBalance);
      expect(mockBalanceRepository.save).toHaveBeenCalledWith(updatedBalance);
      expect(mockFeeRepository.save).toHaveBeenCalledWith({
        userId,
        type: 'api_call',
        amount: -500,
        description: 'Test deposit',
        status: 'paid',
        transactionId: undefined,
        paidAt: expect.any(Date),
      });
    });

    it('should create new balance if none exists', async () => {
      const mockBalance = {
        balance: 500,
        totalSpent: 0,
        totalDeposited: 500,
        totalRefunded: 0,
        creditLimit: 1000,
        usageStats: {
          totalScans: 0,
          totalApiCalls: 0,
          totalStorageUsed: 0,
          lastResetDate: new Date().toISOString(),
        },
      };

      mockBalanceRepository.findOne.mockResolvedValue(null);
      mockBalanceRepository.create.mockReturnValue(mockBalance);
      mockBalanceRepository.save.mockResolvedValue(mockBalance);
      mockFeeRepository.save.mockResolvedValue({});

      const result = await service.addBalance(userId, addBalanceDto);

      expect(result).toEqual(mockBalance);
      expect(mockBalanceRepository.create).toHaveBeenCalled();
    });
  });

  describe('refundFee', () => {
    const feeId = 'fee-123';
    const refundReason = 'Customer request';

    it('should successfully refund a paid fee', async () => {
      const mockFee = {
        id: feeId,
        userId: 'user-123',
        amount: 100,
        status: 'paid',
      };

      const mockBalance = {
        balance: 0,
        totalSpent: 100,
        totalDeposited: 100,
        totalRefunded: 0,
      };

      const refundedFee = {
        ...mockFee,
        status: 'refunded',
        refundedAt: new Date(),
        refundReason,
      };

      mockFeeRepository.findOne.mockResolvedValue(mockFee);
      mockBalanceRepository.findOne.mockResolvedValue(mockBalance);
      mockBalanceRepository.save.mockResolvedValue({ ...mockBalance, totalRefunded: 100 });
      mockFeeRepository.save.mockResolvedValue(refundedFee);
      jest.spyOn(service, 'addBalance').mockResolvedValue(mockBalance);

      const result = await service.refundFee(feeId, refundReason);

      expect(result).toEqual(refundedFee);
      expect(service.addBalance).toHaveBeenCalledWith('user-123', {
        amount: 100,
        description: 'Refund for scan',
        transactionId: `refund_${feeId}`,
      });
    });

    it('should throw error if fee not found', async () => {
      mockFeeRepository.findOne.mockResolvedValue(null);

      await expect(service.refundFee(feeId, refundReason)).rejects.toThrow('Fee not found');
    });

    it('should throw error if fee is not paid', async () => {
      const mockFee = {
        id: feeId,
        status: 'pending',
      };

      mockFeeRepository.findOne.mockResolvedValue(mockFee);

      await expect(service.refundFee(feeId, refundReason)).rejects.toThrow('Only paid fees can be refunded');
    });
  });

  describe('canAffordOperation', () => {
    const userId = 'user-123';

    it('should return true when user can afford operation', async () => {
      const mockBalance = {
        balance: 200,
      };

      const params = {
        type: 'scan' as const,
        codeSize: 1000,
      };

      mockBalanceRepository.findOne.mockResolvedValue(mockBalance);
      mockFeeCalculator.canAffordOperation.mockReturnValue(true);

      const result = await service.canAffordOperation(userId, params);

      expect(result).toBe(true);
      expect(feeCalculator.canAffordOperation).toHaveBeenCalledWith(200, params);
    });

    it('should return false when user cannot afford operation', async () => {
      const mockBalance = {
        balance: 50,
      };

      const params = {
        type: 'scan' as const,
        codeSize: 1000,
      };

      mockBalanceRepository.findOne.mockResolvedValue(mockBalance);
      mockFeeCalculator.canAffordOperation.mockReturnValue(false);

      const result = await service.canAffordOperation(userId, params);

      expect(result).toBe(false);
    });

    it('should return false when user has no balance', async () => {
      const params = {
        type: 'scan' as const,
        codeSize: 1000,
      };

      mockBalanceRepository.findOne.mockResolvedValue(null);

      const result = await service.canAffordOperation(userId, params);

      expect(result).toBe(false);
    });
  });

  describe('getFeeHistory', () => {
    const userId = 'user-123';

    it('should return paginated fee history', async () => {
      const mockFees = [
        { id: 'fee-1', userId, amount: 100 },
        { id: 'fee-2', userId, amount: 50 },
      ];

      mockFeeRepository.findAndCount.mockResolvedValue([mockFees, 2]);

      const result = await service.getFeeHistory(userId, 1, 10);

      expect(result).toEqual({
        fees: mockFees,
        total: 2,
      });
      expect(mockFeeRepository.findAndCount).toHaveBeenCalledWith({
        where: { userId },
        order: { createdAt: 'DESC' },
        skip: 0,
        take: 10,
      });
    });
  });

  describe('getFeeStats', () => {
    it('should return fee statistics for user', async () => {
      const mockStats = {
        totalFees: 10,
        totalAmount: 1000,
        averageAmount: 100,
        paidFees: 8,
        refundedFees: 2,
      };

      const mockQueryBuilder = {
        where: jest.fn().mockReturnThis(),
        select: jest.fn().mockReturnThis(),
        addSelect: jest.fn().mockReturnThis(),
        getRawOne: jest.fn().mockResolvedValue(mockStats),
      };

      mockFeeRepository.createQueryBuilder.mockReturnValue(mockQueryBuilder);

      const result = await service.getFeeStats('user-123');

      expect(result).toEqual({
        totalFees: 10,
        totalAmount: 1000,
        averageAmount: 100,
        paidFees: 8,
        refundedFees: 2,
      });
      expect(mockQueryBuilder.where).toHaveBeenCalledWith('fee.userId = :userId', { userId: 'user-123' });
    });

    it('should return global fee statistics when no userId provided', async () => {
      const mockStats = {
        totalFees: 100,
        totalAmount: 10000,
        averageAmount: 100,
        paidFees: 80,
        refundedFees: 20,
      };

      const mockQueryBuilder = {
        where: jest.fn().mockReturnThis(),
        select: jest.fn().mockReturnThis(),
        addSelect: jest.fn().mockReturnThis(),
        getRawOne: jest.fn().mockResolvedValue(mockStats),
      };

      mockFeeRepository.createQueryBuilder.mockReturnValue(mockQueryBuilder);

      const result = await service.getFeeStats();

      expect(result).toEqual({
        totalFees: 100,
        totalAmount: 10000,
        averageAmount: 100,
        paidFees: 80,
        refundedFees: 20,
      });
      expect(mockQueryBuilder.where).not.toHaveBeenCalled();
    });
  });
});
