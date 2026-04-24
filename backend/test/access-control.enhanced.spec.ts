import { Test, TestingModule } from '@nestjs/testing';
import { EnhancedRolesGuard, Permission, ROLE_PERMISSIONS } from '../src/auth/enhanced-roles.guard';
import { MultiSignatureService } from '../src/auth/multi-signature.service';
import { Reflector } from '@nestjs/core';
import { ExecutionContext, ForbiddenException } from '@nestjs/common';

describe('Enhanced Access Control System', () => {
  let guard: EnhancedRolesGuard;
  let multiSignatureService: MultiSignatureService;
  let reflector: Reflector;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [
        EnhancedRolesGuard,
        {
          provide: MultiSignatureService,
          useValue: {
            validateMultiSignature: jest.fn(),
          },
        },
        {
          provide: Reflector,
          useValue: {
            getAllAndOverride: jest.fn(),
          },
        },
      ],
    }).compile();

    guard = module.get<EnhancedRolesGuard>(EnhancedRolesGuard);
    multiSignatureService = module.get<MultiSignatureService>(MultiSignatureService);
    reflector = module.get<Reflector>(Reflector);
  });

  describe('Role-Based Access Control', () => {
    const mockContext = (user: any, metadata: any = {}) => {
      const context = {
        switchToHttp: jest.fn().mockReturnValue({
          getRequest: jest.fn().mockReturnValue({ user }),
        }),
      } as any;
      
      reflector.getAllAndOverride = jest.fn().mockReturnValue(metadata);
      
      return context;
    };

    it('should allow access with correct role', async () => {
      const user = { userId: 'test-user', role: 'admin' };
      const context = mockContext(user, ['admin']);
      
      const result = await guard.canActivate(context);
      expect(result).toBe(true);
    });

    it('should deny access with incorrect role', async () => {
      const user = { userId: 'test-user', role: 'viewer' };
      const context = mockContext(user, ['admin']);
      
      await expect(guard.canActivate(context)).rejects.toThrow(ForbiddenException);
    });

    it('should allow access with correct permissions', async () => {
      const user = { userId: 'test-user', role: 'developer' };
      const context = mockContext(user, null, [Permission.GENERATE_PATCH]);
      
      reflector.getAllAndOverride
        .mockReturnValueOnce(null) // No roles required
        .mockReturnValueOnce([Permission.GENERATE_PATCH]); // Permissions required
      
      const result = await guard.canActivate(context);
      expect(result).toBe(true);
    });

    it('should deny access with insufficient permissions', async () => {
      const user = { userId: 'test-user', role: 'viewer' };
      const context = mockContext(user, null, [Permission.ADMIN_ESCROW]);
      
      reflector.getAllAndOverride
        .mockReturnValueOnce(null) // No roles required
        .mockReturnValueOnce([Permission.ADMIN_ESCROW]); // Admin permission required
      
      await expect(guard.canActivate(context)).rejects.toThrow(ForbiddenException);
    });

    it('should deny access if user is not authenticated', async () => {
      const context = mockContext(null, ['admin']);
      
      await expect(guard.canActivate(context)).rejects.toThrow(ForbiddenException);
    });
  });

  describe('Permission System', () => {
    it('should have correct permissions for admin role', () => {
      const adminPermissions = ROLE_PERMISSIONS.admin;
      expect(adminPermissions).toContain(Permission.ADMIN_ESCROW);
      expect(adminPermissions).toContain(Permission.ADMIN_PATCH);
      expect(adminPermissions).toContain(Permission.SYSTEM_CONFIG);
      expect(adminPermissions.length).toBe(Object.values(Permission).length);
    });

    it('should have limited permissions for developer role', () => {
      const devPermissions = ROLE_PERMISSIONS.developer;
      expect(devPermissions).toContain(Permission.GENERATE_PATCH);
      expect(devPermissions).toContain(Permission.APPLY_PATCH);
      expect(devPermissions).not.toContain(Permission.SYSTEM_CONFIG);
      expect(devPermissions).not.toContain(Permission.DELETE_USER);
    });

    it('should have minimal permissions for viewer role', () => {
      const viewerPermissions = ROLE_PERMISSIONS.viewer;
      expect(viewerPermissions).toContain(Permission.READ_USER);
      expect(viewerPermissions).toContain(Permission.CREATE_ESCROW);
      expect(viewerPermissions).not.toContain(Permission.GENERATE_PATCH);
      expect(viewerPermissions).not.toContain(Permission.ADMIN_ESCROW);
    });
  });

  describe('Multi-Signature Integration', () => {
    it('should validate multi-signature when required', async () => {
      const user = { userId: 'test-user', role: 'admin' };
      const context = mockContext(user, ['admin']);
      
      const multiSigConfig = {
        requiredSignatures: 2,
        timeoutMinutes: 60,
        operationType: 'test_operation',
      };

      reflector.getAllAndOverride
        .mockReturnValueOnce(['admin']) // Role check
        .mockReturnValueOnce(null) // No permissions required
        .mockReturnValueOnce(multiSigConfig); // Multi-sig required

      (multiSignatureService.validateMultiSignature as jest.Mock).mockResolvedValue(true);

      const result = await guard.canActivate(context);
      expect(result).toBe(true);
      expect(multiSignatureService.validateMultiSignature).toHaveBeenCalledWith(
        context.switchToHttp().getRequest(),
        multiSigConfig,
        user
      );
    });

    it('should deny access when multi-signature validation fails', async () => {
      const user = { userId: 'test-user', role: 'admin' };
      const context = mockContext(user, ['admin']);
      
      const multiSigConfig = {
        requiredSignatures: 2,
        timeoutMinutes: 60,
        operationType: 'test_operation',
      };

      reflector.getAllAndOverride
        .mockReturnValueOnce(['admin']) // Role check
        .mockReturnValueOnce(null) // No permissions required
        .mockReturnValueOnce(multiSigConfig); // Multi-sig required

      (multiSignatureService.validateMultiSignature as jest.Mock).mockResolvedValue(false);

      await expect(guard.canActivate(context)).rejects.toThrow(ForbiddenException);
    });
  });
});

describe('MultiSignatureService', () => {
  let service: MultiSignatureService;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [
        MultiSignatureService,
        {
          provide: 'ConfigService',
          useValue: {
            get: jest.fn(),
          },
        },
      ],
    }).compile();

    service = module.get<MultiSignatureService>(MultiSignatureService);
  });

  describe('Multi-Signature Request Management', () => {
    it('should create a new multi-signature request', async () => {
      const config = {
        requiredSignatures: 2,
        timeoutMinutes: 60,
        operationType: 'test_operation',
      };

      const request = await service.createMultiSignatureRequest(
        'test_operation',
        'user1',
        config,
        { metadata: 'test' }
      );

      expect(request).toBeDefined();
      expect(request.operationType).toBe('test_operation');
      expect(request.requestedBy).toBe('user1');
      expect(request.requiredSignatures).toBe(2);
      expect(request.status).toBe('pending');
      expect(request.currentSignatures).toHaveLength(0);
    });

    it('should add signature to request', async () => {
      const config = {
        requiredSignatures: 2,
        operationType: 'test_operation',
      };

      const request = await service.createMultiSignatureRequest(
        'test_operation',
        'user1',
        config
      );

      const result = await service.addSignature(
        request.id,
        'user2',
        'admin',
        'signature123'
      );

      expect(result.success).toBe(true);
      expect(result.request?.currentSignatures).toHaveLength(1);
      expect(result.request?.currentSignatures[0].userId).toBe('user2');
    });

    it('should approve request when required signatures reached', async () => {
      const config = {
        requiredSignatures: 2,
        operationType: 'test_operation',
      };

      const request = await service.createMultiSignatureRequest(
        'test_operation',
        'user1',
        config
      );

      // Add first signature
      await service.addSignature(request.id, 'user2', 'admin', 'sig1');
      
      // Add second signature (should approve)
      const result = await service.addSignature(request.id, 'user3', 'developer', 'sig2');

      expect(result.success).toBe(true);
      expect(result.request?.status).toBe('approved');
      expect(result.request?.currentSignatures).toHaveLength(2);
    });

    it('should prevent duplicate signatures from same user', async () => {
      const config = {
        requiredSignatures: 2,
        operationType: 'test_operation',
      };

      const request = await service.createMultiSignatureRequest(
        'test_operation',
        'user1',
        config
      );

      // First signature
      await service.addSignature(request.id, 'user2', 'admin', 'sig1');
      
      // Try to add second signature from same user
      const result = await service.addSignature(request.id, 'user2', 'admin', 'sig2');

      expect(result.success).toBe(false);
      expect(result.message).toContain('already signed');
    });

    it('should handle expired requests', async () => {
      const config = {
        requiredSignatures: 2,
        timeoutMinutes: 0.001, // Very short timeout
        operationType: 'test_operation',
      };

      const request = await service.createMultiSignatureRequest(
        'test_operation',
        'user1',
        config
      );

      // Wait for expiration
      await new Promise(resolve => setTimeout(resolve, 100));

      const result = await service.addSignature(request.id, 'user2', 'admin', 'sig1');

      expect(result.success).toBe(false);
      expect(result.message).toContain('expired');
    });

    it('should allow request cancellation by creator', async () => {
      const config = {
        requiredSignatures: 2,
        operationType: 'test_operation',
      };

      const request = await service.createMultiSignatureRequest(
        'test_operation',
        'user1',
        config
      );

      const result = await service.cancelRequest(request.id, 'user1');

      expect(result.success).toBe(true);
      expect(result.message).toContain('cancelled');

      const cancelledRequest = await service.getRequest(request.id);
      expect(cancelledRequest?.status).toBe('rejected');
    });

    it('should prevent cancellation by non-creator', async () => {
      const config = {
        requiredSignatures: 2,
        operationType: 'test_operation',
      };

      const request = await service.createMultiSignatureRequest(
        'test_operation',
        'user1',
        config
      );

      const result = await service.cancelRequest(request.id, 'user2');

      expect(result.success).toBe(false);
      expect(result.message).toContain('Only request creator can cancel');
    });
  });
});
