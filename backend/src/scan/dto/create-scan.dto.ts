import { IsString, IsNotEmpty, IsOptional, IsBoolean, ValidateNested } from 'class-validator';
import { Type } from 'class-transformer';

export class ScanOptionsDto {
  @IsOptional()
  @IsBoolean()
  deepAnalysis?: boolean;

  @IsOptional()
  @IsBoolean()
  checkInvariants?: boolean;

  @IsOptional()
  @IsBoolean()
  enableExperimental?: boolean;

  @IsOptional()
  @IsString()
  customRules?: string;
}

export class CreateScanDto {
  @IsString()
  @IsNotEmpty()
  code: string;

  @IsOptional()
  @ValidateNested()
  @Type(() => ScanOptionsDto)
  options?: ScanOptionsDto;
}
