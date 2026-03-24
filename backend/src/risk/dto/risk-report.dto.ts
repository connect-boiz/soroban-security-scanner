import { IsString, IsNumber, IsEnum, IsArray, IsOptional, IsObject, ValidateNested } from 'class-validator';
import { Type } from 'class-transformer';
import { ApiProperty, ApiPropertyOptional } from '@nestjs/swagger';

export class RiskTrendDto {
  @ApiProperty({ description: 'Timestamp' })
  @IsString()
  timestamp: string;

  @ApiProperty({ description: 'Risk score' })
  @IsNumber()
  riskScore: number;

  @ApiProperty({ description: 'Portfolio value' })
  @IsNumber()
  portfolioValue: number;

  @ApiProperty({ description: 'Volatility' })
  @IsNumber()
  volatility: number;
}

export class StressTestResultDto {
  @ApiProperty({ description: 'Scenario name' })
  @IsString()
  scenario: string;

  @ApiProperty({ description: 'Portfolio loss' })
  @IsNumber()
  portfolioLoss: number;

  @ApiProperty({ description: 'Loss percentage' })
  @IsNumber()
  lossPercentage: number;

  @ApiProperty({ description: 'Recovery time (days)' })
  @IsNumber()
  recoveryTime: number;

  @ApiProperty({ description: 'Risk factors impacted' })
  @IsArray()
  @IsString({ each: true })
  riskFactors: string[];
}

export class HedgingStrategyDto {
  @ApiProperty({ description: 'Strategy name' })
  @IsString()
  name: string;

  @ApiProperty({ description: 'Strategy type' })
  @IsString()
  type: string;

  @ApiProperty({ description: 'Effectiveness percentage' })
  @IsNumber()
  effectiveness: number;

  @ApiProperty({ description: 'Implementation cost' })
  @IsNumber()
  cost: number;

  @ApiProperty({ description: 'Hedge ratio' })
  @IsNumber()
  hedgeRatio: number;

  @ApiProperty({ description: 'Recommended instruments' })
  @IsArray()
  @IsString({ each: true })
  instruments: string[];
}

export class ComplianceCheckDto {
  @ApiProperty({ description: 'Regulation name' })
  @IsString()
  regulation: string;

  @ApiProperty({ description: 'Compliance status' })
  @IsEnum(['compliant', 'non-compliant', 'partial'])
  status: 'compliant' | 'non-compliant' | 'partial';

  @ApiProperty({ description: 'Risk score impact' })
  @IsNumber()
  riskScoreImpact: number;

  @ApiProperty({ description: 'Required actions' })
  @IsArray()
  @IsString({ each: true })
  requiredActions: string[];

  @ApiPropertyOptional({ description: 'Deadline for compliance' })
  @IsOptional()
  @IsString()
  deadline?: string;
}

export class DetailedRiskReportDto {
  @ApiProperty({ description: 'Report identifier' })
  @IsString()
  id: string;

  @ApiProperty({ description: 'Portfolio identifier' })
  @IsString()
  portfolioId: string;

  @ApiProperty({ description: 'Report period start' })
  @IsString()
  periodStart: string;

  @ApiProperty({ description: 'Report period end' })
  @IsString()
  periodEnd: string;

  @ApiProperty({ description: 'Executive summary' })
  @IsString()
  executiveSummary: string;

  @ApiProperty({ description: 'Risk trends', type: [RiskTrendDto] })
  @IsArray()
  @ValidateNested({ each: true })
  @Type(() => RiskTrendDto)
  riskTrends: RiskTrendDto[];

  @ApiProperty({ description: 'Stress test results', type: [StressTestResultDto] })
  @IsArray()
  @ValidateNested({ each: true })
  @Type(() => StressTestResultDto)
  stressTestResults: StressTestResultDto[];

  @ApiProperty({ description: 'Recommended hedging strategies', type: [HedgingStrategyDto] })
  @IsArray()
  @ValidateNested({ each: true })
  @Type(() => HedgingStrategyDto)
  hedgingStrategies: HedgingStrategyDto[];

  @ApiProperty({ description: 'Compliance checks', type: [ComplianceCheckDto] })
  @IsArray()
  @ValidateNested({ each: true })
  @Type(() => ComplianceCheckDto)
  complianceChecks: ComplianceCheckDto[];

  @ApiProperty({ description: 'Key risk indicators' })
  @IsObject()
  keyRiskIndicators: {
    totalExposure: number;
    concentrationRisk: number;
    liquidityRisk: number;
    marketRisk: number;
    creditRisk: number;
    operationalRisk: number;
  };

  @ApiProperty({ description: 'Risk mitigation actions taken' })
  @IsArray()
  @IsString({ each: true })
  mitigationActions: string[];

  @ApiProperty({ description: 'Report generation timestamp' })
  @IsString()
  generatedAt: string;

  @ApiProperty({ description: 'Next review date' })
  @IsString()
  nextReviewDate: string;
}
