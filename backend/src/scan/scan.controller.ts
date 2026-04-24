import { Controller, Post, Get, Param, Query, Body, UseGuards, Request } from '@nestjs/common';
import { ApiTags, ApiOperation, ApiResponse, ApiParam, ApiQuery } from '@nestjs/swagger';
import { ThrottlerGuard } from '@nestjs/throttler';
import { CreateScanDto } from './dto/create-scan.dto';
import { ScanService } from './scan.service';
import { FeeGuard, SetFeeType, SetFeeParams } from '../fee/guards/fee.guard';
import { JwtAuthGuard } from '../auth/jwt-auth.guard';
import { EnhancedRolesGuard, RequirePermissions, Permission } from '../auth/enhanced-roles.guard';
import { RequireMultiSignature } from '../auth/multi-signature.decorator';

@ApiTags('scan')
@UseGuards(JwtAuthGuard, EnhancedRolesGuard, ThrottlerGuard)
@Controller('scan')
export class ScanController {
  constructor(private readonly scanService: ScanService) {}

  @Post()
  @UseGuards(FeeGuard)
  @SetFeeType('scan')
  @SetFeeParams((req) => ({
    codeSize: req.body.code?.length || 0,
    complexity: req.body.options?.complexity || 1,
  }))
  @ApiOperation({ summary: 'Create and start a new security scan' })
  @ApiResponse({ status: 201, description: 'Scan created and started successfully' })
  @ApiResponse({ status: 400, description: 'Invalid input data' })
  @ApiResponse({ status: 402, description: 'Insufficient balance' })
  async createScan(@Body() createScanDto: CreateScanDto, @Request() req: any) {
    const userId = req.user?.id || 'anonymous';
    
    // Validate code size
    const maxSize = 1024 * 1024; // 1MB
    if (createScanDto.code.length > maxSize) {
      throw new Error('Code size exceeds maximum limit of 1MB');
    }

    const scan = await this.scanService.createScan(createScanDto, userId);
    
    // Charge fee for the scan
    if (req.feeInfo) {
      await this.scanService.chargeScanFee(scan.id, userId, req.feeInfo);
    }
    
    // Start scan asynchronously
    this.scanService.startScan(scan.id).catch(error => {
      console.error(`Failed to start scan ${scan.id}:`, error);
    });

    return {
      scanId: scan.id,
      status: 'started',
      feeCharged: req.feeInfo?.estimatedFee?.totalFee || 0,
      message: 'Scan initiated successfully',
    };
  }

  @Get(':scanId')
  @ApiOperation({ summary: 'Get scan results by ID' })
  @ApiParam({ name: 'scanId', description: 'Scan ID' })
  @ApiResponse({ status: 200, description: 'Scan results retrieved successfully' })
  @ApiResponse({ status: 404, description: 'Scan not found' })
  async getScan(@Param('scanId') scanId: string) {
    return await this.scanService.getScan(scanId);
  }

  @Get('user/:userId')
  @ApiOperation({ summary: 'Get scan history for a user' })
  @ApiParam({ name: 'userId', description: 'User ID' })
  @ApiQuery({ name: 'page', required: false, description: 'Page number (default: 1)' })
  @ApiQuery({ name: 'limit', required: false, description: 'Items per page (default: 10)' })
  @ApiResponse({ status: 200, description: 'Scan history retrieved successfully' })
  async getScanHistory(
    @Param('userId') userId: string,
    @Query('page') page?: number,
    @Query('limit') limit?: number,
  ) {
    const pageNum = page ? parseInt(page.toString()) : 1;
    const limitNum = limit ? parseInt(limit.toString()) : 10;

    return await this.scanService.getScanHistory(userId, pageNum, limitNum);
  }

  @Get('stats/summary')
  @ApiOperation({ summary: 'Get scan statistics summary' })
  @ApiResponse({ status: 200, description: 'Statistics retrieved successfully' })
  async getScanStats() {
    return await this.scanService.getScanStats();
  }

  @Get(':scanId/sarif')
  @ApiOperation({ summary: 'Get SARIF report for a scan' })
  @ApiParam({ name: 'scanId', description: 'Scan ID' })
  @ApiResponse({ status: 200, description: 'SARIF report retrieved successfully' })
  @ApiResponse({ status: 404, description: 'Scan not found' })
  async getSarifReport(@Param('scanId') scanId: string) {
    const scan = await this.scanService.getScan(scanId);
    
    // Generate SARIF report if not already stored
    if (!scan.sarifReport) {
      // This would be generated during scan analysis in a real implementation
      throw new Error('SARIF report not available for this scan');
    }

    return {
      scanId: scan.id,
      sarifReport: scan.sarifReport,
      generatedAt: scan.updatedAt,
    };
  }

  @Post(':scanId/vulnerabilities/:vulnerabilityId/acknowledge')
  @RequirePermissions(Permission.ACKNOWLEDGE_VULNERABILITY)
  @RequireMultiSignature({
    requiredSignatures: 2,
    timeoutMinutes: 60,
    allowedRoles: ['admin', 'developer'],
    operationType: 'acknowledge_vulnerability'
  })
  @ApiOperation({ summary: 'Acknowledge a vulnerability' })
  @ApiParam({ name: 'scanId', description: 'Scan ID' })
  @ApiParam({ name: 'vulnerabilityId', description: 'Vulnerability ID' })
  @ApiResponse({ status: 200, description: 'Vulnerability acknowledged successfully' })
  async acknowledgeVulnerability(
    @Param('scanId') scanId: string,
    @Param('vulnerabilityId') vulnerabilityId: string,
    @Body() body: { notes?: string },
    @Request() req: any
  ) {
    // In a real implementation, this would update the vulnerability status in the database
    return {
      scanId,
      vulnerabilityId,
      status: 'acknowledged',
      notes: body.notes,
      acknowledgedBy: req.user.userId,
      acknowledgedAt: new Date().toISOString(),
    };
  }

  @Post(':scanId/vulnerabilities/:vulnerabilityId/false-positive')
  @RequirePermissions(Permission.MARK_FALSE_POSITIVE)
  @RequireMultiSignature({
    requiredSignatures: 2,
    timeoutMinutes: 60,
    allowedRoles: ['admin', 'developer'],
    operationType: 'mark_false_positive'
  })
  @ApiOperation({ summary: 'Mark vulnerability as false positive' })
  @ApiParam({ name: 'scanId', description: 'Scan ID' })
  @ApiParam({ name: 'vulnerabilityId', description: 'Vulnerability ID' })
  @ApiResponse({ status: 200, description: 'Vulnerability marked as false positive successfully' })
  async markAsFalsePositive(
    @Param('scanId') scanId: string,
    @Param('vulnerabilityId') vulnerabilityId: string,
    @Body() body: { reason?: string },
    @Request() req: any
  ) {
    // In a real implementation, this would update the vulnerability status in the database
    return {
      scanId,
      vulnerabilityId,
      status: 'false_positive',
      reason: body.reason,
      markedBy: req.user.userId,
      markedAt: new Date().toISOString(),
    };
  }
}
