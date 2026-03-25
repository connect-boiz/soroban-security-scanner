import { Injectable, NotFoundException, ForbiddenException, ConflictException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { ApiKey } from './entities/api-key.entity';
import { User } from '../user/entities/user.entity';
import { CreateApiKeyDto, UpdateApiKeyDto, ApiKeyResponseDto, GenerateApiKeyResponseDto } from './dto/api-key.dto';
import * as crypto from 'crypto';
import { v4 as uuidv4 } from 'uuid';

@Injectable()
export class ApiKeyService {
  constructor(
    @InjectRepository(ApiKey)
    private apiKeyRepository: Repository<ApiKey>,
    @InjectRepository(User)
    private userRepository: Repository<User>,
  ) {}

  async generateApiKey(createApiKeyDto: CreateApiKeyDto, userId: string): Promise<GenerateApiKeyResponseDto> {
    const user = await this.userRepository.findOne({ where: { id: userId } });
    if (!user) {
      throw new NotFoundException('User not found');
    }

    // Check if user has permission to create API keys
    if (!this.hasPermission(user.role, 'api-key:create')) {
      throw new ForbiddenException('Insufficient permissions to create API keys');
    }

    // Generate the actual API key
    const apiKey = this.generateRawApiKey();
    const keyHash = this.hashApiKey(apiKey);
    const keyPrefix = this.extractKeyPrefix(apiKey);

    // Create API key record
    const apiKeyEntity = this.apiKeyRepository.create({
      userId,
      keyHash,
      keyPrefix,
      name: createApiKeyDto.name,
      description: createApiKeyDto.description,
      permissions: createApiKeyDto.permissions || ['scan:read', 'scan:write'],
      expiresAt: createApiKeyDto.expiresAt,
      status: 'active',
    });

    const savedApiKey = await this.apiKeyRepository.save(apiKeyEntity);

    return {
      apiKey, // Only return the raw key once during generation
      apiKeyData: this.formatApiKeyResponse(savedApiKey),
    };
  }

  async findAllApiKeys(userId: string): Promise<ApiKeyResponseDto[]> {
    const user = await this.userRepository.findOne({ where: { id: userId } });
    if (!user) {
      throw new NotFoundException('User not found');
    }

    // Check if user has permission to view API keys
    if (!this.hasPermission(user.role, 'api-key:read')) {
      throw new ForbiddenException('Insufficient permissions to view API keys');
    }

    const apiKeys = await this.apiKeyRepository.find({
      where: { userId },
      order: { createdAt: 'DESC' },
    });

    return apiKeys.map(key => this.formatApiKeyResponse(key));
  }

  async updateApiKey(id: string, updateApiKeyDto: UpdateApiKeyDto, userId: string): Promise<ApiKeyResponseDto> {
    const apiKey = await this.findApiKeyByIdAndUser(id, userId);
    
    // Check if user has permission to update API keys
    const user = await this.userRepository.findOne({ where: { id: userId } });
    if (!this.hasPermission(user.role, 'api-key:update')) {
      throw new ForbiddenException('Insufficient permissions to update API keys');
    }

    Object.assign(apiKey, updateApiKeyDto);
    const updatedApiKey = await this.apiKeyRepository.save(apiKey);

    return this.formatApiKeyResponse(updatedApiKey);
  }

  async revokeApiKey(id: string, userId: string): Promise<void> {
    const apiKey = await this.findApiKeyByIdAndUser(id, userId);
    
    // Check if user has permission to revoke API keys
    const user = await this.userRepository.findOne({ where: { id: userId } });
    if (!this.hasPermission(user.role, 'api-key:delete')) {
      throw new ForbiddenException('Insufficient permissions to revoke API keys');
    }

    apiKey.status = 'revoked';
    await this.apiKeyRepository.save(apiKey);
  }

  async validateApiKey(apiKey: string): Promise<{ user: User; permissions: string[] } | null> {
    const keyHash = this.hashApiKey(apiKey);
    const keyPrefix = this.extractKeyPrefix(apiKey);

    const apiKeyRecord = await this.apiKeyRepository.findOne({
      where: { 
        keyHash, 
        keyPrefix,
        status: 'active',
      },
      relations: ['user'],
    });

    if (!apiKeyRecord) {
      return null;
    }

    // Check if key has expired
    if (apiKeyRecord.expiresAt && apiKeyRecord.expiresAt < new Date()) {
      return null;
    }

    // Update last used timestamp
    apiKeyRecord.lastUsedAt = new Date();
    await this.apiKeyRepository.save(apiKeyRecord);

    return {
      user: apiKeyRecord.user,
      permissions: apiKeyRecord.permissions,
    };
  }

  private async findApiKeyByIdAndUser(id: string, userId: string): Promise<ApiKey> {
    const apiKey = await this.apiKeyRepository.findOne({ where: { id, userId } });
    if (!apiKey) {
      throw new NotFoundException('API key not found');
    }
    return apiKey;
  }

  private generateRawApiKey(): string {
    const prefix = 'sk_live_';
    const randomPart = crypto.randomBytes(32).toString('hex');
    return `${prefix}${randomPart}`;
  }

  private hashApiKey(apiKey: string): string {
    return crypto.createHash('sha256').update(apiKey).digest('hex');
  }

  private extractKeyPrefix(apiKey: string): string {
    // Return first 12 characters for display/matching
    return apiKey.substring(0, 12);
  }

  private formatApiKeyResponse(apiKey: ApiKey): ApiKeyResponseDto {
    return {
      id: apiKey.id,
      name: apiKey.name,
      description: apiKey.description,
      status: apiKey.status,
      keyPrefix: apiKey.keyPrefix,
      lastUsedAt: apiKey.lastUsedAt,
      expiresAt: apiKey.expiresAt,
      permissions: apiKey.permissions,
      createdAt: apiKey.createdAt,
      updatedAt: apiKey.updatedAt,
    };
  }

  private hasPermission(userRole: string, permission: string): boolean {
    const rolePermissions = {
      admin: [
        'api-key:create',
        'api-key:read',
        'api-key:update',
        'api-key:delete',
      ],
      developer: [
        'api-key:create',
        'api-key:read',
        'api-key:update',
        'api-key:delete',
      ],
      viewer: [
        'api-key:read',
      ],
    };

    return rolePermissions[userRole]?.includes(permission) || false;
  }
}
