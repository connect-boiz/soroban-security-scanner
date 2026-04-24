import { Injectable, CanActivate, ExecutionContext, ForbiddenException } from '@nestjs/common';
import { Reflector } from '@nestjs/core';
import { ROLES_KEY } from './roles.decorator';
import { MULTI_SIG_KEY } from './multi-signature.decorator';
import { MultiSignatureService } from './multi-signature.service';

export enum Permission {
  // User management
  CREATE_USER = 'create_user',
  READ_USER = 'read_user',
  UPDATE_USER = 'update_user',
  DELETE_USER = 'delete_user',
  
  // Vulnerability management
  ACKNOWLEDGE_VULNERABILITY = 'acknowledge_vulnerability',
  MARK_FALSE_POSITIVE = 'mark_false_positive',
  VERIFY_VULNERABILITY = 'verify_vulnerability',
  
  // Escrow management
  CREATE_ESCROW = 'create_escrow',
  RELEASE_ESCROW = 'release_escrow',
  REFUND_ESCROW = 'refund_escrow',
  ADMIN_ESCROW = 'admin_escrow',
  
  // Scan management
  START_SCAN = 'start_scan',
  CANCEL_SCAN = 'cancel_scan',
  ADMIN_SCAN = 'admin_scan',
  
  // LLM Patch management
  GENERATE_PATCH = 'generate_patch',
  APPLY_PATCH = 'apply_patch',
  ADMIN_PATCH = 'admin_patch',
  
  // System administration
  SYSTEM_CONFIG = 'system_config',
  VIEW_LOGS = 'view_logs',
  MANAGE_API_KEYS = 'manage_api_keys',
}

export const ROLE_PERMISSIONS = {
  admin: Object.values(Permission),
  developer: [
    Permission.READ_USER,
    Permission.CREATE_ESCROW,
    Permission.RELEASE_ESCROW,
    Permission.REFUND_ESCROW,
    Permission.START_SCAN,
    Permission.GENERATE_PATCH,
    Permission.APPLY_PATCH,
  ],
  viewer: [
    Permission.READ_USER,
    Permission.CREATE_ESCROW,
  ],
};

@Injectable()
export class EnhancedRolesGuard implements CanActivate {
  constructor(
    private reflector: Reflector,
    private multiSignatureService: MultiSignatureService,
  ) {}

  async canActivate(context: ExecutionContext): Promise<boolean> {
    const request = context.switchToHttp().getRequest();
    const user = request.user;

    if (!user) {
      throw new ForbiddenException('User not authenticated');
    }

    // Check role-based permissions
    const requiredRoles = this.reflector.getAllAndOverride<string[]>(ROLES_KEY, [
      context.getHandler(),
      context.getClass(),
    ]);

    if (requiredRoles && requiredRoles.length > 0) {
      const hasRole = requiredRoles.some((role) => user.role === role);
      if (!hasRole) {
        throw new ForbiddenException(`Insufficient role. Required: ${requiredRoles.join(', ')}`);
      }
    }

    // Check permission-based access
    const requiredPermissions = this.reflector.getAllAndOverride<Permission[]>(PERMISSIONS_KEY, [
      context.getHandler(),
      context.getClass(),
    ]);

    if (requiredPermissions && requiredPermissions.length > 0) {
      const userPermissions = ROLE_PERMISSIONS[user.role] || [];
      const hasPermission = requiredPermissions.every(permission => 
        userPermissions.includes(permission)
      );

      if (!hasPermission) {
        throw new ForbiddenException(`Insufficient permissions. Required: ${requiredPermissions.join(', ')}`);
      }
    }

    // Check multi-signature requirements
    const multiSigConfig = this.reflector.getAllAndOverride(MULTI_SIG_KEY, [
      context.getHandler(),
      context.getClass(),
    ]);

    if (multiSigConfig) {
      const isValidMultiSig = await this.multiSignatureService.validateMultiSignature(
        request,
        multiSigConfig,
        user
      );

      if (!isValidMultiSig) {
        throw new ForbiddenException('Multi-signature validation failed');
      }
    }

    return true;
  }
}

export const PERMISSIONS_KEY = 'permissions';
export const RequirePermissions = (...permissions: Permission[]) => 
  SetMetadata(PERMISSIONS_KEY, permissions);
