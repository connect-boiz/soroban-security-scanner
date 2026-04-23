import { IsString, IsOptional, IsObject, ValidateNested } from 'class-validator';
import { Type } from 'class-transformer';

export class VulnerabilityDto {
  @IsString()
  id: string;

  @IsString()
  file_path: string;

  @IsString()
  vulnerability_type: string;

  @IsString()
  severity: string;

  @IsString()
  title: string;

  @IsString()
  description: string;

  @IsString()
  code_snippet: string;

  @IsString()
  line_number: number;

  @IsOptional()
  @IsObject()
  sarif_report?: any;
}

export class GeneratePatchDto {
  @ValidateNested()
  @Type(() => VulnerabilityDto)
  vulnerability: VulnerabilityDto;

  @IsString()
  original_code: string;

  @IsOptional()
  @IsString()
  context?: string;
}
