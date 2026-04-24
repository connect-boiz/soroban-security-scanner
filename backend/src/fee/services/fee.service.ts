import { Injectable, Logger, BadRequestException, NotFoundException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { Fee } from '../entities/fee.entity';
import { UserBalance } from '../entities/user-balance.entity';
import { CreateFeeDto } from '../dto/create-fee.dto';
import { AddBalanceDto } from '../dto/add-balance.dto';
import { FeeCalculatorService, FeeCalculationParams } from './fee-calculator.service';

@Injectable()
export class FeeService {
  private readonly logger = new Logger(FeeService.name);

  constructor(
    @InjectRepository(Fee)
    private readonly feeRepository: Repository<Fee>,
    @InjectRepository(UserBalance)
    private readonly balanceRepository: Repository<UserBalance>,
    private readonly feeCalculator: FeeCalculatorService,
  ) {}

  /**
   * Create and charge a fee for an operation
   */
  async createAndChargeFee(createFeeDto: CreateFeeDto, userId: string): Promise<Fee> {
    // Get or create user balance
    let balance = await this.getUserBalance(userId);
    if (!balance) {
      balance = await this.createUserBalance(userId);
    }

    // Calculate fee amount if not provided
    let amount = createFeeDto.amount;
    if (!amount) {
      const params: FeeCalculationParams = {
        type: createFeeDto.type,
        codeSize: createFeeDto.metadata?.codeSize,
        complexity: createFeeDto.metadata?.scanComplexity,
        processingTime: createFeeDto.metadata?.processingTime,
        resourceUsage: createFeeDto.metadata?.resourceUsage,
      };
      amount = this.feeCalculator.calculateFee(params);
    }

    // Check if user can afford the operation
    if (balance.balance < amount) {
      throw new BadRequestException(`Insufficient balance. Required: ${amount}, Available: ${balance.balance}`);
    }

    // Deduct from balance
    await this.deductFromBalance(userId, amount);

    // Create fee record
    const fee = this.feeRepository.create({
      ...createFeeDto,
      userId,
      amount,
      status: 'paid',
      paidAt: new Date(),
    });

    const savedFee = await this.feeRepository.save(fee);
    
    this.logger.log(`Fee charged: ${amount} to user ${userId} for ${createFeeDto.type}`);
    
    return savedFee;
  }

  /**
   * Add balance to user account
   */
  async addBalance(userId: string, addBalanceDto: AddBalanceDto): Promise<UserBalance> {
    let balance = await this.getUserBalance(userId);
    if (!balance) {
      balance = await this.createUserBalance(userId);
    }

    balance.balance += addBalanceDto.amount;
    balance.totalDeposited += addBalanceDto.amount;

    await this.balanceRepository.save(balance);

    // Create fee record for the deposit
    await this.feeRepository.save({
      userId,
      type: 'api_call', // Using api_call as deposit type for now
      amount: -addBalanceDto.amount, // Negative amount for deposit
      description: addBalanceDto.description || 'Balance deposit',
      status: 'paid',
      transactionId: addBalanceDto.transactionId,
      paidAt: new Date(),
    });

    this.logger.log(`Balance added: ${addBalanceDto.amount} to user ${userId}`);
    
    return balance;
  }

  /**
   * Get user balance
   */
  async getUserBalance(userId: string): Promise<UserBalance | null> {
    return await this.balanceRepository.findOne({ where: { userId } });
  }

  /**
   * Create user balance record
   */
  private async createUserBalance(userId: string): Promise<UserBalance> {
    const balance = this.balanceRepository.create({
      userId,
      balance: 0,
      totalSpent: 0,
      totalDeposited: 0,
      totalRefunded: 0,
      creditLimit: 1000, // Default credit limit
      usageStats: {
        totalScans: 0,
        totalApiCalls: 0,
        totalStorageUsed: 0,
        lastResetDate: new Date().toISOString(),
      },
    });

    return await this.balanceRepository.save(balance);
  }

  /**
   * Deduct amount from user balance
   */
  private async deductFromBalance(userId: string, amount: number): Promise<void> {
    const balance = await this.getUserBalance(userId);
    if (!balance) {
      throw new NotFoundException('User balance not found');
    }

    if (balance.balance < amount) {
      throw new BadRequestException('Insufficient balance');
    }

    balance.balance -= amount;
    balance.totalSpent += amount;
    balance.lastFeeDeductedAt = new Date();

    await this.balanceRepository.save(balance);
  }

  /**
   * Refund a fee
   */
  async refundFee(feeId: string, reason: string): Promise<Fee> {
    const fee = await this.feeRepository.findOne({ where: { id: feeId } });
    if (!fee) {
      throw new NotFoundException('Fee not found');
    }

    if (fee.status !== 'paid') {
      throw new BadRequestException('Only paid fees can be refunded');
    }

    // Add refund to user balance
    await this.addBalance(fee.userId, {
      amount: fee.amount,
      description: `Refund for ${fee.type}`,
      transactionId: `refund_${feeId}`,
    });

    // Update fee status
    fee.status = 'refunded';
    fee.refundedAt = new Date();
    fee.refundReason = reason;

    const updatedFee = await this.feeRepository.save(fee);

    // Update user balance refund total
    const balance = await this.getUserBalance(fee.userId);
    if (balance) {
      balance.totalRefunded += fee.amount;
      await this.balanceRepository.save(balance);
    }

    this.logger.log(`Fee refunded: ${fee.amount} to user ${fee.userId}`);
    
    return updatedFee;
  }

  /**
   * Get fee history for a user
   */
  async getFeeHistory(userId: string, page: number = 1, limit: number = 10): Promise<{ fees: Fee[], total: number }> {
    const [fees, total] = await this.feeRepository.findAndCount({
      where: { userId },
      order: { createdAt: 'DESC' },
      skip: (page - 1) * limit,
      take: limit,
    });

    return { fees, total };
  }

  /**
   * Check if user can afford an operation
   */
  async canAffordOperation(userId: string, params: FeeCalculationParams): Promise<boolean> {
    const balance = await this.getUserBalance(userId);
    if (!balance) {
      return false;
    }

    return this.feeCalculator.canAffordOperation(balance.balance, params);
  }

  /**
   * Get estimated fee for an operation
   */
  getEstimatedFee(params: FeeCalculationParams) {
    return this.feeCalculator.getEstimatedFee(params);
  }

  /**
   * Get fee statistics
   */
  async getFeeStats(userId?: string): Promise<any> {
    const query = this.feeRepository.createQueryBuilder('fee');
    
    if (userId) {
      query.where('fee.userId = :userId', { userId });
    }

    const stats = await query
      .select('COUNT(*) as totalFees')
      .addSelect('SUM(fee.amount) as totalAmount')
      .addSelect('AVG(fee.amount) as averageAmount')
      .addSelect('COUNT(CASE WHEN fee.status = :paid THEN 1 END) as paidFees', { paid: 'paid' })
      .addSelect('COUNT(CASE WHEN fee.status = :refunded THEN 1 END) as refundedFees', { refunded: 'refunded' })
      .getRawOne();

    return {
      totalFees: parseInt(stats.totalFees) || 0,
      totalAmount: parseInt(stats.totalAmount) || 0,
      averageAmount: parseFloat(stats.averageAmount) || 0,
      paidFees: parseInt(stats.paidFees) || 0,
      refundedFees: parseInt(stats.refundedFees) || 0,
    };
  }
}
