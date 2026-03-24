import { IsString, IsNumber, IsEnum, IsOptional, IsObject, ValidateNested, IsArray } from 'class-validator';
import { Type } from 'class-transformer';
import { ApiProperty, ApiPropertyOptional } from '@nestjs/swagger';

export class MarketFactorDto {
  @ApiProperty({ description: 'Price factor' })
  @IsNumber()
  price: number;

  @ApiProperty({ description: 'Volume factor' })
  @IsNumber()
  volume: number;

  @ApiProperty({ description: 'Volatility factor' })
  @IsNumber()
  volatility: number;

  @ApiProperty({ description: 'Interest rate factor' })
  @IsNumber()
  interestRate: number;

  @ApiProperty({ description: 'Exchange rate factor' })
  @IsNumber()
  exchangeRate: number;
}

export class PositionDto {
  @ApiProperty({ description: 'Position identifier' })
  @IsString()
  id: string;

  @ApiProperty({ description: 'Position type' })
  @IsString()
  type: string;

  @ApiProperty({ description: 'Position size' })
  @IsNumber()
  size: number;

  @ApiProperty({ description: 'Current price' })
  @IsNumber()
  currentPrice: number;

  @ApiPropertyOptional({ description: 'Position maturity date' })
  @IsOptional()
  @IsString()
  maturityDate?: string;

  @ApiPropertyOptional({ description: 'Underlying asset' })
  @IsOptional()
  @IsString()
  underlying?: string;
}

export class PortfolioDto {
  @ApiProperty({ description: 'Portfolio identifier' })
  @IsString()
  id: string;

  @ApiProperty({ description: 'Portfolio positions', type: [PositionDto] })
  @IsArray()
  @ValidateNested({ each: true })
  @Type(() => PositionDto)
  positions: PositionDto[];

  @ApiProperty({ description: 'Total portfolio value' })
  @IsNumber()
  totalValue: number;

  @ApiPropertyOptional({ description: 'Portfolio currency' })
  @IsOptional()
  @IsString()
  currency?: string;
}

export class RiskAssessmentDto {
  @ApiProperty({ description: 'User identifier' })
  @IsString()
  userId: string;

  @ApiProperty({ description: 'Portfolio data' })
  @ValidateNested()
  @Type(() => PortfolioDto)
  portfolio: PortfolioDto;

  @ApiPropertyOptional({ description: 'Market factors' })
  @IsOptional()
  @ValidateNested()
  @Type(() => MarketFactorDto)
  marketFactors?: MarketFactorDto;

  @ApiPropertyOptional({ description: 'Assessment confidence level', default: 0.95 })
  @IsOptional()
  @IsNumber()
  confidenceLevel?: number;

  @ApiPropertyOptional({ description: 'Time horizon in days', default: 1 })
  @IsOptional()
  @IsNumber()
  timeHorizon?: number;

  @ApiPropertyOptional({ description: 'Include stress testing', default: true })
  @IsOptional()
  includeStressTest?: boolean;
}

export class RiskMetricsDto {
  @ApiProperty({ description: 'Value at Risk (1-day)' })
  @IsNumber()
  var1d: number;

  @ApiProperty({ description: 'Value at Risk (10-day)' })
  @IsNumber()
  var10d: number;

  @ApiProperty({ description: 'Value at Risk (30-day)' })
  @IsNumber()
  var30d: number;

  @ApiProperty({ description: 'Expected Shortfall' })
  @IsNumber()
  expectedShortfall: number;

  @ApiProperty({ description: 'Portfolio beta' })
  @IsNumber()
  beta: number;

  @ApiProperty({ description: 'Portfolio volatility' })
  @IsNumber()
  volatility: number;

  @ApiProperty({ description: 'Correlation coefficient' })
  @IsNumber()
  correlation: number;

  @ApiProperty({ description: 'Concentration risk' })
  @IsNumber()
  concentration: number;
}

export class RiskAlertDto {
  @ApiProperty({ description: 'Alert identifier' })
  @IsString()
  id: string;

  @ApiProperty({ description: 'Risk type' })
  @IsEnum(['market', 'credit', 'operational', 'liquidity', 'counterparty'])
  riskType: 'market' | 'credit' | 'operational' | 'liquidity' | 'counterparty';

  @ApiProperty({ description: 'Alert severity' })
  @IsEnum(['low', 'medium', 'high', 'critical'])
  severity: 'low' | 'medium' | 'high' | 'critical';

  @ApiProperty({ description: 'Alert message' })
  @IsString()
  message: string;

  @ApiProperty({ description: 'Current risk score' })
  @IsNumber()
  currentScore: number;

  @ApiProperty({ description: 'Risk threshold' })
  @IsNumber()
  threshold: number;

  @ApiProperty({ description: 'Recommended action' })
  @IsString()
  recommendation: string;

  @ApiProperty({ description: 'Alert timestamp' })
  @IsString()
  timestamp: string;
}

export class RiskReportDto {
  @ApiProperty({ description: 'Report identifier' })
  @IsString()
  id: string;

  @ApiProperty({ description: 'Portfolio identifier' })
  @IsString()
  portfolioId: string;

  @ApiProperty({ description: 'Risk metrics' })
  @ValidateNested()
  @Type(() => RiskMetricsDto)
  metrics: RiskMetricsDto;

  @ApiProperty({ description: 'Risk alerts', type: [RiskAlertDto] })
  @IsArray()
  @ValidateNested({ each: true })
  @Type(() => RiskAlertDto)
  alerts: RiskAlertDto[];

  @ApiProperty({ description: 'Overall risk score' })
  @IsNumber()
  overallRiskScore: number;

  @ApiProperty({ description: 'Risk level' })
  @IsEnum(['low', 'medium', 'high', 'critical'])
  riskLevel: 'low' | 'medium' | 'high' | 'critical';

  @ApiProperty({ description: 'Hedging recommendations' })
  @IsArray()
  @IsString({ each: true })
  hedgingRecommendations: string[];

  @ApiProperty({ description: 'Report generation timestamp' })
  @IsString()
  generatedAt: string;

  @ApiProperty({ description: 'Report period' })
  @IsString()
  period: string;
}
