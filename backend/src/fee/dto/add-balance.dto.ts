import { IsNumber, IsString, IsOptional } from 'class-validator';

export class AddBalanceDto {
  @IsNumber()
  amount: number;

  @IsString()
  @IsOptional()
  paymentMethod?: string;

  @IsString()
  @IsOptional()
  transactionId?: string;

  @IsString()
  @IsOptional()
  description?: string;
}
