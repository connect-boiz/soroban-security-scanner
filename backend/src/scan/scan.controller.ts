import { Controller, Post, Get, Param, Query, Body, UseGuards, Request } from '@nestjs/common';
import { ApiTags, ApiOperation, ApiResponse, ApiParam, ApiQuery } from '@nestjs/swagger';
import { ThrottlerGuard } from '@nestjs/throttler';
import { CreateScanDto } from './dto/create-scan.dto';
import { ScanService } from './scan.service';

@ApiTags('scan')
@UseGuards(ThrottlerGuard)
@Controller('scan')
export class ScanController {
  constructor(private readonly scanService: ScanService) {}

  @Post()
  @ApiOperation({ summary: 'Create and start a new security scan' })
  @ApiResponse({ status: 201, description: 'Scan created and started successfully' })
  @ApiResponse({ status: 400, description: 'Invalid input data' })
  async createScan(@Body() createScanDto: CreateScanDto, @Request() req: any) {
    const userId = req.user?.id || 'anonymous';
    
    // Validate code size
    const maxSize = 1024 * 1024; // 1MB
    if (createScanDto.code.length > maxSize) {
      throw new Error('Code size exceeds maximum limit of 1MB');
    }

    const scan = await this.scanService.createScan(createScanDto, userId);
    
    // Start scan asynchronously
    this.scanService.startScan(scan.id).catch(error => {
      console.error(`Failed to start scan ${scan.id}:`, error);
    });

    return {
      scanId: scan.id,
      status: 'started',
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
}
