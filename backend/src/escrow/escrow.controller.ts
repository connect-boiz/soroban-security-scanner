import {
  Controller,
  Post,
  Get,
  Body,
  Param,
  Query,
  HttpCode,
  HttpStatus,
  Logger,
  UseGuards,
  Request,
  UseInterceptors,
} from '@nestjs/common';
import { ApiTags, ApiOperation, ApiResponse, ApiBearerAuth } from '@nestjs/swagger';
import { Throttle } from '@nestjs/throttler';
import { EscrowService } from './escrow.service';
import { JwtAuthGuard } from '../auth/jwt-auth.guard';
import { CreateEscrowDto } from './dto/create-escrow.dto';
import { ReleaseEscrowDto } from './dto/release-escrow.dto';
import { CustomRateLimitGuard } from '../common/guards/rate-limit.guard';
import { EscrowCreationRateLimit } from '../common/decorators/rate-limit.decorator';
import { EnhancedRolesGuard, RequirePermissions, Permission } from '../auth/enhanced-roles.guard';
import { RequireMultiSignature } from '../auth/multi-signature.decorator';

@ApiTags('escrow')
@Controller('escrow')
@UseGuards(JwtAuthGuard, EnhancedRolesGuard, CustomRateLimitGuard)
@ApiBearerAuth()
export class EscrowController {
  private readonly logger = new Logger(EscrowController.name);

  constructor(private readonly escrowService: EscrowService) {}

  @Post()
  @HttpCode(HttpStatus.CREATED)
  @RequirePermissions(Permission.CREATE_ESCROW)
  @ApiOperation({ summary: 'Create a new escrow entry' })
  @ApiResponse({ status: 201, description: 'Escrow created successfully' })
  @ApiResponse({ status: 400, description: 'Invalid request' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  @Throttle(5, 60) // 5 requests per minute
  @EscrowCreationRateLimit()
  async createEscrow(
    @Body() createEscrowDto: CreateEscrowDto,
    @Request() req,
  ): Promise<{
    success: boolean;
    data?: any;
    error?: string;
  }> {
    try {
      this.logger.log(
        `User ${req.user.userId} creating escrow for ${createEscrowDto.beneficiary}`,
      );

      const escrow = await this.escrowService.createEscrow(
        createEscrowDto,
        req.user.userId,
      );

      this.logger.log(`Escrow created successfully with ID: ${escrow.id}`);

      return {
        success: true,
        data: escrow,
      };
    } catch (error) {
      this.logger.error(`Failed to create escrow: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }

  @Post(':escrowId/release')
  @HttpCode(HttpStatus.OK)
  @RequirePermissions(Permission.RELEASE_ESCROW)
  @RequireMultiSignature({
    requiredSignatures: 2,
    timeoutMinutes: 30,
    allowedRoles: ['admin', 'developer'],
    operationType: 'release_escrow'
  })
  @ApiOperation({ summary: 'Release funds from escrow' })
  @ApiResponse({ status: 200, description: 'Escrow released successfully' })
  @ApiResponse({ status: 400, description: 'Invalid request' })
  @ApiResponse({ status: 404, description: 'Escrow not found' })
  @Throttle(10, 60) // 10 requests per minute
  async releaseEscrow(
    @Param('escrowId') escrowId: string,
    @Body() releaseEscrowDto: ReleaseEscrowDto,
    @Request() req,
  ): Promise<{
    success: boolean;
    data?: any;
    error?: string;
  }> {
    try {
      this.logger.log(
        `User ${req.user.userId} releasing escrow ${escrowId}`,
      );

      const result = await this.escrowService.releaseEscrow(
        escrowId,
        releaseEscrowDto,
        req.user.userId,
      );

      if (result.success) {
        this.logger.log(`Escrow ${escrowId} released successfully`);
      }

      return result;
    } catch (error) {
      this.logger.error(`Failed to release escrow ${escrowId}: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }

  @Get(':escrowId')
  @ApiOperation({ summary: 'Get escrow details by ID' })
  @ApiResponse({ status: 200, description: 'Escrow details retrieved successfully' })
  @ApiResponse({ status: 404, description: 'Escrow not found' })
  async getEscrow(@Param('escrowId') escrowId: string, @Request() req): Promise<{
    success: boolean;
    data?: any;
    error?: string;
  }> {
    try {
      const escrow = await this.escrowService.getEscrow(escrowId, req.user.userId);
      
      return {
        success: true,
        data: escrow,
      };
    } catch (error) {
      this.logger.error(`Failed to get escrow ${escrowId}: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }

  @Get('user/:userId')
  @ApiOperation({ summary: 'Get escrow history for a user' })
  @ApiResponse({ status: 200, description: 'Escrow history retrieved successfully' })
  async getEscrowHistory(
    @Param('userId') userId: string,
    @Query('page') page?: number,
    @Query('limit') limit?: number,
    @Request() req,
  ): Promise<{
    success: boolean;
    data?: any;
    error?: string;
  }> {
    try {
      // Users can only view their own escrow history
      if (userId !== req.user.userId) {
        return {
          success: false,
          error: 'Unauthorized to view other users escrow history',
        };
      }

      const pageNum = page ? parseInt(page.toString()) : 1;
      const limitNum = limit ? parseInt(limit.toString()) : 10;

      const history = await this.escrowService.getEscrowHistory(userId, pageNum, limitNum);
      
      return {
        success: true,
        data: history,
      };
    } catch (error) {
      this.logger.error(`Failed to get escrow history for ${userId}: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }

  @Get('stats/summary')
  @ApiOperation({ summary: 'Get escrow statistics summary' })
  @ApiResponse({ status: 200, description: 'Statistics retrieved successfully' })
  async getEscrowStats(): Promise<{
    success: boolean;
    data?: any;
    error?: string;
  }> {
    try {
      const stats = await this.escrowService.getEscrowStats();
      
      return {
        success: true,
        data: stats,
      };
    } catch (error) {
      this.logger.error(`Failed to get escrow stats: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }
}
