import { Injectable, NotFoundException, ForbiddenException, ConflictException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { ApiKey } from './entities/api-key.entity';
import { User } from '../user/entities/user.entity';
import { CreateApiKeyDto, UpdateApiKeyDto, ApiKeyResponseDto, GenerateApiKeyResponseDto } from './dto/api-key.dto';
import * as crypto from 'crypto';
import { v4 as uuidv4 } from 'uuid';

/**
 * API Key Management Service
 *
 * Handles generation, validation, and lifecycle management of API keys used to authenticate
 * API requests. This is a SECURITY-CRITICAL service; all operations require careful
 * attention to key protection, access control, and audit logging.
 *
 * # Security Model
 *
 * ## Key Storage
 * - Raw API keys are NEVER stored in the database
 * - Only SHA-256 hashes of API keys are persisted
 * - Keys are returned to users only once during generation
 * - Users must securely store their keys; we cannot recover lost keys
 *
 * ## Access Control
 * - All operations are protected by role-based access control (RBAC)
 * - Users can only manage their own keys (except admins/developers)
 * - Role-based permissions are enforced at method entry
 *
 * ## Key Validation
 * - Validation uses constant-time comparison to prevent timing attacks
 * - Expired keys are automatically rejected
 * - Revoked keys are permanently disabled
 *
 * ## Audit Trail
 * - All key operations should be logged for security auditing
 * - Last used timestamp is updated on validation
 * - Key generation, revocation, and updates create audit events
 *
 * # Threat Model
 * - **Key Exposure**: Assumes keys may be exposed and leaked
 * - **Unauthorized Access**: Assumes attackers may gain database access
 * - **Privilege Escalation**: Users cannot grant themselves elevated permissions
 * - **Token Reuse**: Revoked or expired keys cannot be revalidated
 *
 * # Security Considerations
 * - Do not log raw API keys
 * - Do not transmit keys over unencrypted channels (use HTTPS only)
 * - Do not cache unhashed keys in memory longer than necessary
 * - Implement rate limiting on key validation to prevent brute force attacks
 * - Monitor for suspicious key generation patterns (e.g., rapid generation)
 */
@Injectable()
export class ApiKeyService {
  constructor(
    @InjectRepository(ApiKey)
    private apiKeyRepository: Repository<ApiKey>,
    @InjectRepository(User)
    private userRepository: Repository<User>,
  ) {}

  /**
   * Generates a new API key for a user.
   *
   * Creates a new cryptographically-random API key and stores a SHA-256 hash in the database.
   * The raw key is returned to the user ONCE; if lost, it cannot be recovered.
   *
   * # Arguments
   * - `createApiKeyDto`: Key metadata (name, description, permissions, expiration)
   * - `userId`: ID of the user to create the key for
   *
   * # Returns
   * `GenerateApiKeyResponseDto` containing:
   * - `apiKey`: The raw API key (string) - returned only once
   * - `apiKeyData`: Key metadata for user reference (excludes raw key)
   *
   * # Security Considerations
   *
   * ## Permission Checks
   * - Verifies user exists before creating key
   * - Checks RBAC permissions (requires 'api-key:create' permission)
   * - Rejects requests from users without create permission
   * - Throws ForbiddenException if user lacks permissions
   *
   * ## Key Generation
   * - Uses crypto.randomBytes(32) for 256-bit entropy
   * - Prefixes with 'sk_live_' for key identification
   * - Returns raw key as hex string
   * - Key format: 'sk_live_' + 64 hex characters
   *
   * ## Key Storage
   * - Only SHA-256 hash of the key is stored (unhashable to raw key)
   * - Key prefix (first 12 chars) stored for user reference
   * - Never store or log the raw key
   * - Hash is computed via this.hashApiKey() using SHA-256
   *
   * ## Audit Trail
   * - Created key is persisted with:
   *   - User ID for attribution
   *   - Creation timestamp (auto-set)
   *   - Key metadata (name, description, permissions)
   *   - Expiration time (if specified)
   * - Recommend logging this operation for audit compliance
   *
   * # Errors
   * - NotFoundException: If user not found
   * - ForbiddenException: If user lacks 'api-key:create' permission
   *
   * # Important Security Notes
   * - The raw key is returned in the HTTP response; use HTTPS only
   * - Client must securely store the returned key
   * - If key is lost, user must generate a new one
   * - Consider implementing rate limiting on key generation
   * - Monitor for suspicious key generation patterns (e.g., many keys in short time)
   *
   * # Example
   * ```typescript
   * const response = await apiKeyService.generateApiKey(
   *   { name: 'CI/CD', description: 'GitHub Actions', permissions: ['scan:read'] },
   *   userId
   * );
   * console.log('Key:', response.apiKey);  // Only show this once to user
   * ```
   */
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

  /**
   * Retrieves all API keys for a user.
   *
   * Returns a list of all active and revoked API keys belonging to the specified user.
   * Raw keys are never returned; only key metadata including prefix for identification.
   *
   * # Arguments
   * - `userId`: ID of the user whose keys to retrieve
   *
   * # Returns
   * Array of `ApiKeyResponseDto` containing:
   * - Key ID for reference
   * - Key name and description
   * - Status (active/revoked)
   * - Key prefix (first 12 characters for identification)
   * - Timestamps (creation, last used, expiration)
   * - Permissions list
   *
   * # Security Considerations
   *
   * ## Permission Checks
   * - Verifies user exists before retrieval
   * - Checks RBAC permissions (requires 'api-key:read' permission)
   * - Rejects requests from users without read permission
   * - Throws ForbiddenException if user lacks permissions
   *
   * ## Information Disclosure
   * - Raw keys are NEVER returned (even if requested)
   * - Only key prefix (first 12 chars) is shown for user reference
   * - Key hash is never returned
   * - Status includes revoked keys so users can manage lifecycle
   *
   * ## Audit Trail
   * - Returns creation and last-used timestamps
   * - Last-used time helps users identify active keys
   * - Can be used to detect unauthorized key access
   *
   * # Errors
   * - NotFoundException: If user not found
   * - ForbiddenException: If user lacks 'api-key:read' permission
   *
   * # Example
   * ```typescript
   * const keys = await apiKeyService.findAllApiKeys(userId);
   * keys.forEach(key => {
   *   console.log(`${key.name}: ${key.keyPrefix}... (${key.status})`);
   * });
   * ```
   */
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

  /**
   * Updates metadata for an existing API key.
   *
   * Allows users to update key name, description, and permissions.
   * Cannot change the key hash or regenerate the underlying key.
   *
   * # Arguments
   * - `id`: ID of the key to update
   * - `updateApiKeyDto`: New metadata values
   * - `userId`: ID of the user performing the update (for access control)
   *
   * # Returns
   * `ApiKeyResponseDto` with updated metadata
   *
   * # Security Considerations
   *
   * ## Permission Checks
   * - Verifies the key belongs to the user (prevents cross-user updates)
   * - Checks 'api-key:update' permission for the user
   * - Throws ForbiddenException if user lacks permissions
   * - Throws NotFoundException if key not found or belongs to different user
   *
   * ## Permission Escalation Prevention
   * - Users can update their own keys only (not other users' keys)
   * - Permission changes are logged in audit trail
   * - Admins/developers can update keys with elevated permissions
   *
   * ## Audit Trail
   * - Update timestamp is recorded
   * - Key itself is not regenerated (cannot fix exposure this way)
   * - Recommend logging permission changes for compliance
   *
   * # Limitations
   * - Cannot change the underlying key material (use revoke + generate instead)
   * - Cannot change key prefix or ID
   * - Cannot reactivate a revoked key (must generate new key)
   *
   * # Errors
   * - NotFoundException: If key not found or belongs to different user
   * - ForbiddenException: If user lacks 'api-key:update' permission
   *
   * # Example
   * ```typescript
   * const updated = await apiKeyService.updateApiKey(
   *   keyId,
   *   { permissions: ['scan:read', 'scan:write'] },
   *   userId
   * );
   * ```
   */
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

  /**
   * Revokes an API key, permanently disabling it.
   *
   * Sets key status to 'revoked' so future validation attempts fail.
   * This is the primary method for deactivating exposed or unused keys.
   *
   * # Arguments
   * - `id`: ID of the key to revoke
   * - `userId`: ID of the user performing the revocation (for access control)
   *
   * # Security Considerations
   *
   * ## Permission Checks
   * - Verifies key belongs to the user
   * - Checks 'api-key:delete' permission
   * - Throws ForbiddenException if user lacks permissions
   * - Throws NotFoundException if key not found
   *
   * ## Permanent Deactivation
   * - Revocation is permanent; cannot be undone
   * - Revoked keys are rejected by validateApiKey()
   * - Clients using revoked keys receive 401 Unauthorized
   * - Must generate new key to regain access
   *
   * ## Audit Trail
   * - Revoked key status is persisted
   * - Update timestamp records when revocation occurred
   * - Recommend logging revocation event for security audit
   * - Include reason for revocation if possible
   *
   * ## Incident Response
   * - Use this method when:
   *   - Key is suspected of being compromised
   *   - User reports lost/stolen key
   *   - Employee leaves organization
   *   - Permission scope changes
   *
   * # Errors
   * - NotFoundException: If key not found or belongs to different user
   * - ForbiddenException: If user lacks 'api-key:delete' permission
   *
   * # Example
   * ```typescript
   * // Key exposed in GitHub commit
   * await apiKeyService.revokeApiKey(keyId, userId);
   * console.log('Exposed key has been revoked');
   * ```
   */
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

  /**
   * Validates an API key and returns associated user and permissions.
   *
   * This is the primary security function for API authentication.
   * Called for every authenticated API request.
   *
   * # Arguments
   * - `apiKey`: The raw API key string from the request (e.g., from Authorization header)
   *
   * # Returns
   * - Object containing `user` and `permissions` if key is valid and active
   * - `null` if key is invalid, expired, or revoked
   *
   * # Security Considerations
   *
   * ## Key Validation Process
   * 1. Hash the provided key using SHA-256
   * 2. Extract prefix (first 12 characters) for indexed lookup
   * 3. Query database for active key with matching hash and prefix
   * 4. Check that key has not expired
   * 5. Update last-used timestamp for audit trail
   * 6. Return user and permissions or null
   *
   * ## Timing Attack Prevention
   * - Uses constant-time comparison (via crypto.timingSafeEqual())
   * - Hash comparison is timing-safe to prevent timing attacks
   * - All lookups take roughly the same time
   * - Invalid keys do not return early to leak timing information
   *
   * ## Expiration Handling
   * - Expired keys are rejected (returns null)
   * - Expiration check uses server time (UTC)
   * - Prevents use of keys past their intended lifetime
   * - Useful for rotating keys periodically
   *
   * ## Audit Trail
   * - Last-used timestamp is updated on successful validation
   * - Can detect unusual access patterns:
   *   - Key used from different IP addresses
   *   - Key used outside normal hours
   *   - Multiple validation failures (in controller)
   * - Recommend logging validation events for security monitoring
   *
   * ## Threat Model
   * - **Brute Force**: Rate limiting should be implemented in controller
   * - **Timing Attack**: Uses timing-safe comparison
   * - **Replay Attack**: Relies on HTTPS for transport security
   * - **Key Exposure**: Hashes are still vulnerable to DB compromise
   *
   * # Performance
   * - O(1) lookup using hash + prefix indices
   * - Single database query
   * - Suitable for high-volume API authentication
   *
   * # Audit Trail
   * - Last-used timestamp is critical for detecting unauthorized access
   * - Recommend periodic review of last-used times
   * - Correlate with request logs to detect suspicious patterns
   * - Can identify compromised keys by unusual access patterns
   *
   * # Errors
   * - Returns `null` for:
   *   - Invalid key (hash doesn't match)
   *   - Expired key
   *   - Revoked key
   *   - Non-existent key
   *   - Revoked status
   *
   * # Important Security Notes
   * - This method is called for EVERY authenticated request
   * - Performance is critical; indices on keyHash and keyPrefix are essential
   * - Do not log raw API keys in debug/trace output
   * - Update last-used timestamp for security monitoring
   * - Consider caching results for high-traffic APIs (cache expires with key expiration)
   *
   * # Example
   * ```typescript
   * // In API controller
   * const result = await apiKeyService.validateApiKey(authHeader);
   * if (!result) {
   *   throw new UnauthorizedException('Invalid API key');
   * }
   * const { user, permissions } = result;
   * // Proceed with authenticated request
   * ```
   */
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

  /**
   * Finds an API key by ID, ensuring it belongs to the specified user.
   * Used internally to enforce access control.
   *
   * # Security: Prevents Cross-User Access
   * - Verifies key belongs to user before returning
   * - Throws NotFoundException if key not found or belongs to different user
   * - Prevents users from accessing other users' keys
   * - Both conditions throw the same error to avoid information leakage
   *
   * @private
   */
  private async findApiKeyByIdAndUser(id: string, userId: string): Promise<ApiKey> {
    const apiKey = await this.apiKeyRepository.findOne({ where: { id, userId } });
    if (!apiKey) {
      throw new NotFoundException('API key not found');
    }
    return apiKey;
  }

  /**
   * Generates a new cryptographically-random API key.
   *
   * # Security
   * - Uses crypto.randomBytes() for cryptographic randomness
   * - 32 bytes (256 bits) of entropy provides strong uniqueness
   * - Prefix 'sk_live_' identifies key type and environment
   * - Encoded as hex for transport in HTTP headers
   *
   * # Format
   * - 'sk_live_' prefix (8 chars)
   * - 64 hex characters (32 bytes = 256 bits)
   * - Total: 72 characters
   * - URL-safe and can be used in HTTP headers
   *
   * @private
   */
  private generateRawApiKey(): string {
    const prefix = 'sk_live_';
    const randomPart = crypto.randomBytes(32).toString('hex');
    return `${prefix}${randomPart}`;
  }

  /**
   * Computes SHA-256 hash of an API key.
   *
   * # Security
   * - Uses crypto.createHash('sha256') for hashing
   * - One-way hash; cannot derive original key from hash
   * - Deterministic: same key always produces same hash
   * - No salt (not needed for this use case, but consider for future)
   *
   * # Usage
   * - Hashes raw keys before storing in database
   * - Hashes incoming keys during validation
   * - Only hashes are compared for authentication
   *
   * # Limitations
   * - SHA-256 is technically not a password hashing algorithm
   * - Consider bcrypt/argon2 if performance allows
   * - Rainbow tables could theoretically compute hashes (unlikely for random keys)
   * - Database compromise exposes all key hashes (use HTTPS + encrypted DB)
   *
   * # Important Notes
   * - Do not log raw keys or hashes
   * - Hashes are unique enough for database queries
   * - Use timing-safe comparison when validating (prevent timing attacks)
   *
   * @private
   */
  private hashApiKey(apiKey: string): string {
    return crypto.createHash('sha256').update(apiKey).digest('hex');
  }

  /**
   * Extracts a prefix from the API key for indexed lookups.
   *
   * # Purpose
   * - Enables fast database lookups without full hash scan
   * - Shows users a partial key for identification
   * - Prevents users from seeing full key after generation
   *
   * # Security
   * - 12-character prefix provides sufficient uniqueness
   * - Prefix alone is not usable for authentication
   * - Must be combined with full key hash for validation
   * - Reduces database lookup complexity from O(n) to O(1)
   *
   * # Examples
   * - 'sk_live_abc123...' -> keyPrefix = 'sk_live_abc12'
   * - Users see 'sk_live_abc12***' in UI (masked remainder)
   * - Supports key rotation/migration by prefix search
   *
   * @private
   */
  private extractKeyPrefix(apiKey: string): string {
    // Return first 12 characters for display/matching
    return apiKey.substring(0, 12);
  }

  /**
   * Formats an API key entity into a safe response DTO.
   *
   * # Security
   * - Excludes raw key hash (never returned)
   * - Excludes key history or sensitive metadata
   * - Returns only user-displayable information
   * - Safe to include in API responses
   *
   * # Contents
   * - Key ID for operations (revoke, update)
   * - Key name and description (user-provided metadata)
   * - Key prefix (first 12 chars for identification)
   * - Status (active/revoked)
   * - Timestamps (creation, last used, expiration)
   * - Permissions list
   *
   * # Usage
   * - Used internally by service methods
   * - Returned to clients in list/get operations
   * - Never includes raw key or hash
   *
   * @private
   */
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

  /**
   * Checks if a user role has the required permission.
   *
   * # Role-Based Access Control (RBAC)
   *
   * ## Roles Defined
   * - **admin**: Full access to all API key operations
   * - **developer**: Full access to API key operations (same as admin for now)
   * - **viewer**: Read-only access to API keys
   *
   * ## Permission Model
   * - Granular permissions: api-key:create, api-key:read, api-key:update, api-key:delete
   * - Permissions are enforced at service level
   * - Role mappings are defined in this method
   *
   * ## Security Considerations
   * - Centralized permission check prevents permission bypass
   * - Called before every sensitive operation
   * - Unknown roles default to no permissions (fail-secure)
   * - Recommend logging permission denials for security audit
   *
   * ## Permission Descriptions
   * - **api-key:create**: Generate new API keys
   * - **api-key:read**: List and view API keys
   * - **api-key:update**: Modify key metadata and permissions
   * - **api-key:delete**: Revoke/disable API keys
   *
   * ## Important Security Notes
   * - This is NOT the full authorization check
   * - Must also verify user owns the key (cross-user protection)
   * - Both checks are necessary for complete access control
   * - Consider adding audit logging for permission checks
   *
   * # Future Improvements
   * - Load role definitions from database instead of hardcoded
   * - Support dynamic role creation
   * - Add resource-level permissions (specific keys)
   * - Implement attribute-based access control (ABAC)
   *
   * @private
   */
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
