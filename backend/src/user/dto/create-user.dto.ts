import { IsEmail, IsString, IsOptional, IsEnum, MinLength, MaxLength } from 'class-validator';

export class CreateUserDto {
  @IsEmail()
  email: string;

  @IsString()
  @MinLength(6)
  password: string;

  @IsString()
  @IsOptional()
  @MaxLength(50)
  firstName?: string;

  @IsString()
  @IsOptional()
  @MaxLength(50)
  lastName?: string;

  @IsEnum(['admin', 'developer', 'viewer'])
  @IsOptional()
  role?: 'admin' | 'developer' | 'viewer' = 'developer';
}

export class UpdateUserDto {
  @IsString()
  @IsOptional()
  @MaxLength(50)
  firstName?: string;

  @IsString()
  @IsOptional()
  @MaxLength(50)
  lastName?: string;

  @IsEnum(['admin', 'developer', 'viewer'])
  @IsOptional()
  role?: 'admin' | 'developer' | 'viewer';

  @IsString()
  @IsOptional()
  @MinLength(6)
  password?: string;

  @IsOptional()
  isActive?: boolean;
}
