import { IsString, IsUrl, IsEnum, IsOptional, IsObject } from 'class-validator';
import { ApiProperty, ApiPropertyOptional } from '@nestjs/swagger';

export class CreateWebhookDto {
  @ApiProperty()
  @IsString()
  name: string;

  @ApiProperty()
  @IsUrl()
  url: string;

  @ApiProperty({ enum: ['slack', 'discord', 'custom'] })
  @IsEnum(['slack', 'discord', 'custom'])
  type: 'slack' | 'discord' | 'custom';

  @ApiPropertyOptional()
  @IsOptional()
  @IsObject()
  config?: {
    secret?: string;
    channel?: string;
    username?: string;
    icon_url?: string;
  };

  @ApiPropertyOptional({ enum: ['all', 'critical', 'high', 'medium', 'low'] })
  @IsOptional()
  @IsEnum(['all', 'critical', 'high', 'medium', 'low'])
  severityFilter?: 'all' | 'critical' | 'high' | 'medium' | 'low';
}
