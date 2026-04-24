import { IsEnum, IsNumber, IsString, IsOptional, IsObject } from 'class-validator';

export class CreateFeeDto {
  @IsEnum(['scan', 'api_call', 'storage', 'premium_feature'])
  type: 'scan' | 'api_call' | 'storage' | 'premium_feature';

  @IsNumber()
  amount: number;

  @IsString()
  @IsOptional()
  description?: string;

  @IsObject()
  @IsOptional()
  metadata?: {
    scanComplexity?: number;
    codeSize?: number;
    processingTime?: number;
    resourceUsage?: any;
  };

  @IsString()
  @IsOptional()
  scanId?: string;
}
