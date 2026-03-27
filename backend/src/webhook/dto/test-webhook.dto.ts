import { IsString, IsOptional } from 'class-validator';
import { ApiPropertyOptional } from '@nestjs/swagger';

export class TestWebhookDto {
  @ApiPropertyOptional()
  @IsOptional()
  @IsString()
  message?: string;
}
