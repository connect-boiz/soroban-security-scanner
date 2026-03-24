import { Controller, Get, Post, Body, Param, Query } from '@nestjs/common';
import { ApiTags, ApiOperation, ApiResponse } from '@nestjs/swagger';

import { RiskManagementService } from './risk-management.service';
import { RiskAssessmentDto, RiskReportDto, RiskAlertDto } from './dto/risk-assessment.dto';

@ApiTags('Risk Management')
@Controller('risk')
export class RiskController {
  constructor(private readonly riskManagementService: RiskManagementService) {}

  @Post('assess')
  @ApiOperation({ summary: 'Assess portfolio risk' })
  @ApiResponse({ status: 200, description: 'Risk assessment completed', type: RiskReportDto })
  async assessRisk(@Body() assessmentDto: RiskAssessmentDto): Promise<RiskReportDto> {
    return this.riskManagementService.assessRisk(assessmentDto);
  }

  @Get('portfolio/:portfolioId/alerts')
  @ApiOperation({ summary: 'Get risk alerts for portfolio' })
  @ApiResponse({ status: 200, description: 'Risk alerts retrieved', type: [RiskAlertDto] })
  async getRiskAlerts(@Param('portfolioId') portfolioId: string): Promise<RiskAlertDto[]> {
    return this.riskManagementService.getRiskAlerts(portfolioId);
  }

  @Get('portfolio/:portfolioId/metrics')
  @ApiOperation({ summary: 'Get real-time risk metrics' })
  @ApiResponse({ status: 200, description: 'Risk metrics retrieved' })
  async getRiskMetrics(@Param('portfolioId') portfolioId: string): Promise<any> {
    return this.riskManagementService.getRiskMetrics(portfolioId);
  }

  @Post('portfolio/:portfolioId/stress-test')
  @ApiOperation({ summary: 'Run stress tests on portfolio' })
  @ApiResponse({ status: 200, description: 'Stress tests completed' })
  async runStressTests(
    @Param('portfolioId') portfolioId: string,
    @Body() portfolioData: any
  ): Promise<any> {
    return this.riskManagementService.runStressTests(portfolioData);
  }

  @Get('portfolio/:portfolioId/var')
  @ApiOperation({ summary: 'Calculate Value at Risk' })
  @ApiResponse({ status: 200, description: 'VaR calculation completed' })
  async calculateVar(
    @Param('portfolioId') portfolioId: string,
    @Query('confidence') confidence: number = 0.95,
    @Query('horizon') horizon: number = 1
  ): Promise<any> {
    return this.riskManagementService.calculateVar(portfolioId, confidence, horizon);
  }

  @Post('portfolio/:portfolioId/hedge')
  @ApiOperation({ summary: 'Generate hedging strategies' })
  @ApiResponse({ status: 200, description: 'Hedging strategies generated' })
  async generateHedgingStrategies(
    @Param('portfolioId') portfolioId: string,
    @Body() portfolioData: any
  ): Promise<any> {
    return this.riskManagementService.generateHedgingStrategies(portfolioData);
  }
}
