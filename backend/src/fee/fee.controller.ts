import { Controller, Get, Post, Param, Query, Body, UseGuards, Request } from '@nestjs/common';
import { ApiTags, ApiOperation, ApiResponse, ApiParam, ApiQuery } from '@nestjs/swagger';
import { ThrottlerGuard } from '@nestjs/throttler';
import { FeeService } from './services/fee.service';
import { FeeCalculatorService, FeeCalculationParams } from './services/fee-calculator.service';
import { AddBalanceDto } from './dto/add-balance.dto';

@ApiTags('fee')
@UseGuards(ThrottlerGuard)
@Controller('fee')
export class FeeController {
  constructor(
    private readonly feeService: FeeService,
    private readonly feeCalculator: FeeCalculatorService,
  ) {}

  @Post('estimate')
  @ApiOperation({ summary: 'Get estimated fee for an operation' })
  @ApiResponse({ status: 200, description: 'Fee estimation calculated successfully' })
  async estimateFee(@Body() params: FeeCalculationParams) {
    return this.feeCalculator.getEstimatedFee(params);
  }

  @Get('balance')
  @ApiOperation({ summary: 'Get user balance' })
  @ApiResponse({ status: 200, description: 'User balance retrieved successfully' })
  @ApiResponse({ status: 404, description: 'User balance not found' })
  async getBalance(@Request() req: any) {
    const userId = req.user?.id || 'anonymous';
    const balance = await this.feeService.getUserBalance(userId);
    
    if (!balance) {
      return {
        userId,
        balance: 0,
        totalSpent: 0,
        totalDeposited: 0,
        totalRefunded: 0,
        creditLimit: 1000,
      };
    }

    return balance;
  }

  @Post('balance/add')
  @ApiOperation({ summary: 'Add balance to user account' })
  @ApiResponse({ status: 200, description: 'Balance added successfully' })
  @ApiResponse({ status: 400, description: 'Invalid input data' })
  async addBalance(@Body() addBalanceDto: AddBalanceDto, @Request() req: any) {
    const userId = req.user?.id || 'anonymous';
    return await this.feeService.addBalance(userId, addBalanceDto);
  }

  @Get('history')
  @ApiOperation({ summary: 'Get fee history for user' })
  @ApiQuery({ name: 'page', required: false, description: 'Page number (default: 1)' })
  @ApiQuery({ name: 'limit', required: false, description: 'Items per page (default: 10)' })
  @ApiResponse({ status: 200, description: 'Fee history retrieved successfully' })
  async getFeeHistory(
    @Request() req: any,
    @Query('page') page?: number,
    @Query('limit') limit?: number,
  ) {
    const userId = req.user?.id || 'anonymous';
    const pageNum = page ? parseInt(page.toString()) : 1;
    const limitNum = limit ? parseInt(limit.toString()) : 10;

    return await this.feeService.getFeeHistory(userId, pageNum, limitNum);
  }

  @Get('stats')
  @ApiOperation({ summary: 'Get fee statistics' })
  @ApiResponse({ status: 200, description: 'Fee statistics retrieved successfully' })
  async getFeeStats(@Request() req: any) {
    const userId = req.user?.id;
    return await this.feeService.getFeeStats(userId);
  }

  @Post('refund/:feeId')
  @ApiOperation({ summary: 'Refund a fee' })
  @ApiParam({ name: 'feeId', description: 'Fee ID to refund' })
  @ApiResponse({ status: 200, description: 'Fee refunded successfully' })
  @ApiResponse({ status: 404, description: 'Fee not found' })
  @ApiResponse({ status: 400, description: 'Fee cannot be refunded' })
  async refundFee(@Param('feeId') feeId: string, @Body() body: { reason: string }) {
    return await this.feeService.refundFee(feeId, body.reason);
  }

  @Get('can-afford')
  @ApiOperation({ summary: 'Check if user can afford an operation' })
  @ApiResponse({ status: 200, description: 'Affordability check completed' })
  async canAffordOperation(@Body() params: FeeCalculationParams, @Request() req: any) {
    const userId = req.user?.id || 'anonymous';
    const canAfford = await this.feeService.canAffordOperation(userId, params);
    
    if (!canAfford) {
      const balance = await this.feeService.getUserBalance(userId);
      const estimatedFee = this.feeCalculator.getEstimatedFee(params);
      
      return {
        canAfford: false,
        currentBalance: balance?.balance || 0,
        requiredFee: estimatedFee.estimatedFee,
        shortfall: estimatedFee.estimatedFee - (balance?.balance || 0),
      };
    }

    return { canAfford: true };
  }
}
