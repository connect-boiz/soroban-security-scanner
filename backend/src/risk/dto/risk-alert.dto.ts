import { IsString, IsNumber, IsEnum, IsOptional, IsArray, IsObject } from 'class-validator';
import { ApiProperty, ApiPropertyOptional } from '@nestjs/swagger';

export class RiskThresholdDto {
  @ApiProperty({ description: 'Threshold name' })
  @IsString()
  name: string;

  @ApiProperty({ description: 'Threshold value' })
  @IsNumber()
  value: number;

  @ApiProperty({ description: 'Threshold type' })
  @IsEnum(['absolute', 'percentage', 'score'])
  type: 'absolute' | 'percentage' | 'score';

  @ApiPropertyOptional({ description: 'Threshold description' })
  @IsOptional()
  @IsString()
  description?: string;
}

export class RiskAlertConfigDto {
  @ApiProperty({ description: 'Alert configuration name' })
  @IsString()
  name: string;

  @ApiProperty({ description: 'Risk type' })
  @IsEnum(['market', 'credit', 'operational', 'liquidity', 'counterparty'])
  riskType: 'market' | 'credit' | 'operational' | 'liquidity' | 'counterparty';

  @ApiProperty({ description: 'Alert thresholds', type: [RiskThresholdDto] })
  @IsArray()
  thresholds: RiskThresholdDto[];

  @ApiProperty({ description: 'Notification channels' })
  @IsArray()
  @IsString({ each: true })
  notificationChannels: string[];

  @ApiProperty({ description: 'Escalation rules' })
  @IsObject()
  escalationRules: {
    critical: { time: number; action: string };
    high: { time: number; action: string };
    medium: { time: number; action: string };
    low: { time: number; action: string };
  };

  @ApiProperty({ description: 'Auto-mitigation enabled' })
  autoMitigation: boolean;

  @ApiPropertyOptional({ description: 'Mitigation strategies' })
  @IsOptional()
  @IsArray()
  @IsString({ each: true })
  mitigationStrategies?: string[];
}

export class RiskAlertHistoryDto {
  @ApiProperty({ description: 'Alert identifier' })
  @IsString()
  alertId: string;

  @ApiProperty({ description: 'Alert status' })
  @IsEnum(['triggered', 'acknowledged', 'mitigated', 'resolved'])
  status: 'triggered' | 'acknowledged' | 'mitigated' | 'resolved';

  @ApiProperty({ description: 'Timestamp' })
  @IsString()
  timestamp: string;

  @ApiProperty({ description: 'Action taken' })
  @IsString()
  action: string;

  @ApiPropertyOptional({ description: 'Action result' })
  @IsOptional()
  @IsString()
  result?: string;

  @ApiPropertyOptional({ description: 'User who took action' })
  @IsOptional()
  @IsString()
  userId?: string;
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
