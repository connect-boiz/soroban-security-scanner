import { IsString, IsBoolean, IsOptional } from 'class-validator';

export class ReleaseEscrowDto {
  @IsString()
  @IsOptional()
  release_signature?: string;

  @IsBoolean()
  @IsOptional()
  conditions_met?: boolean;

  @IsString()
  @IsOptional()
  release_reason?: string;
}
