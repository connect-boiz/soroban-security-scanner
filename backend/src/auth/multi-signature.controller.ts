import {
  Controller,
  Post,
  Get,
  Body,
  Param,
  HttpCode,
  HttpStatus,
  Logger,
  UseGuards,
  Request,
} from '@nestjs/common';
import { ApiTags, ApiOperation, ApiResponse, ApiBearerAuth } from '@nestjs/swagger';
import { JwtAuthGuard } from './jwt-auth.guard';
import { EnhancedRolesGuard, RequirePermissions, Permission } from './enhanced-roles.guard';
import { MultiSignatureService, MultiSignatureRequest } from './multi-signature.service';
import { RequireMultiSignature } from './multi-signature.decorator';

export interface CreateMultiSignatureRequestDto {
  operationType: string;
  requiredSignatures: number;
  timeoutMinutes?: number;
  allowedRoles?: string[];
  metadata?: any;
}

export interface SignRequestDto {
  signature: string;
}

@ApiTags('multi-signature')
@Controller('multi-signature')
@UseGuards(JwtAuthGuard, EnhancedRolesGuard)
@ApiBearerAuth()
export class MultiSignatureController {
  private readonly logger = new Logger(MultiSignatureController.name);

  constructor(private readonly multiSignatureService: MultiSignatureService) {}

  @Post('request')
  @HttpCode(HttpStatus.CREATED)
  @RequirePermissions(Permission.ADMIN_ESCROW, Permission.ADMIN_PATCH, Permission.ADMIN_SCAN)
  @ApiOperation({ summary: 'Create a new multi-signature request' })
  @ApiResponse({ status: 201, description: 'Multi-signature request created successfully' })
  @ApiResponse({ status: 400, description: 'Invalid request' })
  @ApiResponse({ status: 403, description: 'Insufficient permissions' })
  async createRequest(
    @Body() createDto: CreateMultiSignatureRequestDto,
    @Request() req: any,
  ): Promise<{
    success: boolean;
    data?: MultiSignatureRequest;
    error?: string;
  }> {
    try {
      this.logger.log(
        `User ${req.user.userId} creating multi-signature request for ${createDto.operationType}`,
      );

      const config = {
        requiredSignatures: createDto.requiredSignatures,
        timeoutMinutes: createDto.timeoutMinutes,
        allowedRoles: createDto.allowedRoles,
        operationType: createDto.operationType,
      };

      const request = await this.multiSignatureService.createMultiSignatureRequest(
        createDto.operationType,
        req.user.userId,
        config,
        createDto.metadata,
      );

      return {
        success: true,
        data: request,
      };
    } catch (error) {
      this.logger.error(`Failed to create multi-signature request: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }

  @Post(':requestId/sign')
  @HttpCode(HttpStatus.OK)
  @ApiOperation({ summary: 'Sign a multi-signature request' })
  @ApiResponse({ status: 200, description: 'Request signed successfully' })
  @ApiResponse({ status: 400, description: 'Invalid request' })
  @ApiResponse({ status: 404, description: 'Request not found' })
  async signRequest(
    @Param('requestId') requestId: string,
    @Body() signDto: SignRequestDto,
    @Request() req: any,
  ): Promise<{
    success: boolean;
    message: string;
    request?: MultiSignatureRequest;
    error?: string;
  }> {
    try {
      this.logger.log(
        `User ${req.user.userId} signing multi-signature request ${requestId}`,
      );

      const result = await this.multiSignatureService.addSignature(
        requestId,
        req.user.userId,
        req.user.role,
        signDto.signature,
      );

      return result;
    } catch (error) {
      this.logger.error(`Failed to sign request ${requestId}: ${error.message}`);
      return {
        success: false,
        message: error.message,
        error: error.message,
      };
    }
  }

  @Post(':requestId/cancel')
  @HttpCode(HttpStatus.OK)
  @ApiOperation({ summary: 'Cancel a multi-signature request' })
  @ApiResponse({ status: 200, description: 'Request cancelled successfully' })
  @ApiResponse({ status: 404, description: 'Request not found' })
  @ApiResponse({ status: 403, description: 'Only request creator can cancel' })
  async cancelRequest(
    @Param('requestId') requestId: string,
    @Request() req: any,
  ): Promise<{
    success: boolean;
    message: string;
    error?: string;
  }> {
    try {
      this.logger.log(
        `User ${req.user.userId} cancelling multi-signature request ${requestId}`,
      );

      const result = await this.multiSignatureService.cancelRequest(requestId, req.user.userId);

      return result;
    } catch (error) {
      this.logger.error(`Failed to cancel request ${requestId}: ${error.message}`);
      return {
        success: false,
        message: error.message,
        error: error.message,
      };
    }
  }

  @Get('pending')
  @RequirePermissions(Permission.ADMIN_ESCROW, Permission.ADMIN_PATCH, Permission.ADMIN_SCAN)
  @ApiOperation({ summary: 'Get all pending multi-signature requests' })
  @ApiResponse({ status: 200, description: 'Pending requests retrieved successfully' })
  async getPendingRequests(): Promise<{
    success: boolean;
    data?: MultiSignatureRequest[];
    error?: string;
  }> {
    try {
      const requests = await this.multiSignatureService.getPendingRequests();

      return {
        success: true,
        data: requests,
      };
    } catch (error) {
      this.logger.error(`Failed to get pending requests: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }

  @Get(':requestId')
  @ApiOperation({ summary: 'Get multi-signature request details' })
  @ApiResponse({ status: 200, description: 'Request details retrieved successfully' })
  @ApiResponse({ status: 404, description: 'Request not found' })
  async getRequest(
    @Param('requestId') requestId: string,
  ): Promise<{
    success: boolean;
    data?: MultiSignatureRequest;
    error?: string;
  }> {
    try {
      const request = await this.multiSignatureService.getRequest(requestId);

      if (!request) {
        return {
          success: false,
          error: 'Request not found',
        };
      }

      return {
        success: true,
        data: request,
      };
    } catch (error) {
      this.logger.error(`Failed to get request ${requestId}: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }

  @Post('cleanup')
  @RequirePermissions(Permission.SYSTEM_CONFIG)
  @ApiOperation({ summary: 'Clean up expired multi-signature requests' })
  @ApiResponse({ status: 200, description: 'Expired requests cleaned up successfully' })
  async cleanupExpired(): Promise<{
    success: boolean;
    message: string;
    error?: string;
  }> {
    try {
      await this.multiSignatureService.cleanupExpiredRequests();

      return {
        success: true,
        message: 'Expired requests cleaned up successfully',
      };
    } catch (error) {
      this.logger.error(`Failed to cleanup expired requests: ${error.message}`);
      return {
        success: false,
        message: error.message,
        error: error.message,
      };
    }
  }
}
