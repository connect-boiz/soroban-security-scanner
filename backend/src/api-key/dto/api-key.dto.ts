import { IsString, IsOptional, IsEnum, IsArray, IsNotEmpty, MinLength, MaxLength } from 'class-validator';

export class CreateApiKeyDto {
  @IsString()
  @IsNotEmpty()
  @MinLength(3)
  @MaxLength(100)
  name: string;

  @IsString()
  @IsOptional()
  @MaxLength(500)
  description?: string;

  @IsArray()
  @IsOptional()
  permissions?: string[] = ['scan:read', 'scan:write'];

  @IsOptional()
  expiresAt?: Date;
}

export class UpdateApiKeyDto {
  @IsString()
  @IsOptional()
  @MinLength(3)
  @MaxLength(100)
  name?: string;

  @IsString()
  @IsOptional()
  @MaxLength(500)
  description?: string;

  @IsArray()
  @IsOptional()
  permissions?: string[];

  @IsOptional()
  expiresAt?: Date;
}

export class ApiKeyResponseDto {
  id: string;
  name: string;
  description?: string;
  status: 'active' | 'revoked';
  keyPrefix: string;
  lastUsedAt?: Date;
  expiresAt?: Date;
  permissions: string[];
  createdAt: Date;
  updatedAt: Date;
}

export class GenerateApiKeyResponseDto {
  apiKey: string;
  apiKeyData: ApiKeyResponseDto;
}
