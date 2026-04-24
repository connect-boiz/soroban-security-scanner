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
import { LlmPatchService, PatchRequest, PatchResponse } from './llm-patch.service';
import { JwtAuthGuard } from '../auth/jwt-auth.guard';
import { ApplyPatchRequest } from './dto/apply-patch.dto';
import { GeneratePatchDto } from './dto/generate-patch.dto';
import { CustomRateLimitGuard } from '../common/guards/rate-limit.guard';
import { VulnerabilityReportRateLimit, BatchOperationRateLimit } from '../common/decorators/rate-limit.decorator';

@ApiTags('llm-patch')
@Controller('llm-patch')
@UseGuards(JwtAuthGuard, CustomRateLimitGuard)
@ApiBearerAuth()
export class LlmPatchController {
  private readonly logger = new Logger(LlmPatchController.name);

  constructor(private readonly llmPatchService: LlmPatchService) {}

  @Post('generate')
  @HttpCode(HttpStatus.OK)
  @ApiOperation({ summary: 'Generate AI-powered security patch' })
  @ApiResponse({ status: 200, description: 'Patch generated successfully', type: Object })
  @ApiResponse({ status: 400, description: 'Invalid request' })
  @ApiResponse({ status: 500, description: 'Internal server error' })
  @Throttle(10, 60) // 10 requests per minute
  @VulnerabilityReportRateLimit()
  async generatePatch(
    @Body() generatePatchDto: GeneratePatchDto,
    @Request() req,
  ): Promise<{
    success: boolean;
    data?: PatchResponse;
    error?: string;
    confidence_level?: string;
    shouldApply?: boolean;
  }> {
    try {
      this.logger.log(
        `User ${req.user.userId} requesting patch for vulnerability: ${generatePatchDto.vulnerability.id}`,
      );

      const patchRequest: PatchRequest = {
        vulnerability: generatePatchDto.vulnerability,
        original_code: generatePatchDto.original_code,
        context: generatePatchDto.context,
      };

      const patchResponse = await this.llmPatchService.generatePatch(patchRequest);

      const confidenceLevel = this.llmPatchService.getConfidenceLevel(
        patchResponse.confidence_score,
      );
      const shouldApply = this.llmPatchService.shouldApplyPatch(patchResponse.confidence_score);

      this.logger.log(
        `Patch generated with confidence ${patchResponse.confidence_score} (${confidenceLevel})`,
      );

      return {
        success: true,
        data: patchResponse,
        confidence_level: confidenceLevel,
        should_apply,
      };
    } catch (error) {
      this.logger.error(`Failed to generate patch: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }

  @Post('batch-generate')
  @HttpCode(HttpStatus.OK)
  @ApiOperation({ summary: 'Generate multiple AI-powered security patches' })
  @ApiResponse({ status: 200, description: 'Batch patches generated successfully' })
  @Throttle(3, 300) // 3 requests per 5 minutes
  @BatchOperationRateLimit()
  async batchGeneratePatches(
    @Body() batchDto: { requests: GeneratePatchDto[] },
    @Request() req,
  ): Promise<{
    success: boolean;
    results?: PatchResponse[];
    errors?: string[];
    summary?: any;
  }> {
    try {
      this.logger.log(
        `User ${req.user.userId} requesting batch patches for ${batchDto.requests.length} vulnerabilities`,
      );

      const patchRequests: PatchRequest[] = batchDto.requests.map(dto => ({
        vulnerability: dto.vulnerability,
        original_code: dto.original_code,
        context: dto.context,
      }));

      const { results, errors } = await this.llmPatchService.batchGeneratePatches(patchRequests);
      const summary = this.llmPatchService.generatePatchSummary(results);

      this.logger.log(
        `Batch patch generation completed: ${results.length} successful, ${errors.length} failed`,
      );

      return {
        success: true,
        results,
        errors,
        summary,
      };
    } catch (error) {
      this.logger.error(`Failed to generate batch patches: ${error.message}`);
      return {
        success: false,
        errors: [error.message],
      };
    }
  }

  @Post(':remediationId/apply')
  @HttpCode(HttpStatus.OK)
  @ApiOperation({ summary: 'Apply a generated patch to the target directory' })
  @ApiResponse({ status: 200, description: 'Patch applied successfully' })
  @ApiResponse({ status: 400, description: 'Invalid request' })
  @ApiResponse({ status: 404, description: 'Remediation not found' })
  async applyPatch(
    @Param('remediationId') remediationId: string,
    @Body() applyPatchDto: ApplyPatchRequest,
    @Request() req,
  ): Promise<{
    success: boolean;
    message?: string;
    error?: string;
  }> {
    try {
      this.logger.log(
        `User ${req.user.userId} applying patch ${remediationId} to ${applyPatchDto.target_dir}`,
      );

      const result = await this.llmPatchService.applyPatch(
        remediationId,
        applyPatchDto.target_dir,
      );

      if (result.success) {
        this.logger.log(`Patch ${remediationId} applied successfully`);
        return {
          success: true,
          message: result.message,
        };
      } else {
        this.logger.warn(`Patch ${remediationId} application failed: ${result.error}`);
        return {
          success: false,
          error: result.error,
        };
      }
    } catch (error) {
      this.logger.error(`Failed to apply patch ${remediationId}: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }

  @Get('history/:vulnerabilityId')
  @ApiOperation({ summary: 'Get remediation history for a vulnerability' })
  @ApiResponse({ status: 200, description: 'History retrieved successfully' })
  async getRemediationHistory(
    @Param('vulnerabilityId') vulnerabilityId: string,
  ): Promise<{
    success: boolean;
    data?: any;
    error?: string;
  }> {
    try {
      const history = await this.llmPatchService.getRemediationHistory(vulnerabilityId);
      
      return {
        success: true,
        data: history,
      };
    } catch (error) {
      this.logger.error(`Failed to get remediation history: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }

  @Get('stats')
  @ApiOperation({ summary: 'Get LLM patch service statistics' })
  @ApiResponse({ status: 200, description: 'Statistics retrieved successfully' })
  async getServiceStats(): Promise<{
    success: boolean;
    data?: any;
    error?: string;
  }> {
    try {
      const stats = await this.llmPatchService.getServiceStats();
      
      return {
        success: true,
        data: stats,
      };
    } catch (error) {
      this.logger.error(`Failed to get service stats: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }

  @Get('health')
  @ApiOperation({ summary: 'Check LLM patch service health' })
  @ApiResponse({ status: 200, description: 'Service is healthy' })
  @ApiResponse({ status: 503, description: 'Service is unavailable' })
  async healthCheck(): Promise<{
    success: boolean;
    status?: string;
    error?: string;
  }> {
    try {
      const isHealthy = await this.llmPatchService.healthCheck();
      
      return {
        success: true,
        status: isHealthy ? 'healthy' : 'unhealthy',
      };
    } catch (error) {
      this.logger.error(`Health check failed: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }

  @Get('confidence/:score')
  @ApiOperation({ summary: 'Get confidence level description for a score' })
  @ApiResponse({ status: 200, description: 'Confidence level retrieved' })
  async getConfidenceLevel(@Param('score') score: string): Promise<{
    success: boolean;
    level?: string;
    shouldApply?: boolean;
    error?: string;
  }> {
    try {
      const confidenceScore = parseFloat(score);
      if (isNaN(confidenceScore) || confidenceScore < 0 || confidenceScore > 1) {
        return {
          success: false,
          error: 'Invalid confidence score. Must be between 0 and 1.',
        };
      }

      const level = this.llmPatchService.getConfidenceLevel(confidenceScore);
      const shouldApply = this.llmPatchService.shouldApplyPatch(confidenceScore);

      return {
        success: true,
        level,
        should_apply,
      };
    } catch (error) {
      this.logger.error(`Failed to get confidence level: ${error.message}`);
      return {
        success: false,
        error: error.message,
      };
    }
  }
}
