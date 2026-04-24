import { IsString, IsNumber, IsEnum, IsOptional, IsNotEmpty, Min, Max } from 'class-validator';

export class CreateEscrowDto {
  @IsString()
  @IsNotEmpty()
  beneficiary: string;

  @IsNumber()
  @Min(0.000001) // Minimum amount
  @Max(1000000) // Maximum amount
  amount: number;

  @IsString()
  @IsNotEmpty()
  token: string;

  @IsEnum(['bounty', 'reward', 'emergency'])
  purpose: 'bounty' | 'reward' | 'emergency';

  @IsString()
  @IsOptional()
  lock_until?: string;

  @IsOptional()
  conditions?: any;

  @IsString()
  @IsOptional()
  contract_address?: string;
}
