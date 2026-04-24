import { Test, TestingModule } from '@nestjs/testing';
import { Reflector } from '@nestjs/core';
import { CustomRateLimitGuard } from './rate-limit.guard';
import { ThrottlerException } from '@nestjs/throttler';
import { ExecutionContext, ForbiddenException } from '@nestjs/common';

describe('CustomRateLimitGuard', () => {
  let guard: CustomRateLimitGuard;
  let reflector: Reflector;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [
        CustomRateLimitGuard,
        {
          provide: Reflector,
          useValue: {
            get: jest.fn(),
          },
        },
      ],
    }).compile();

    guard = module.get<CustomRateLimitGuard>(CustomRateLimitGuard);
    reflector = module.get<Reflector>(Reflector);
  });

  it('should be defined', () => {
    expect(guard).toBeDefined();
  });

  describe('canActivate', () => {
    let mockContext: ExecutionContext;
    let mockRequest: any;

    beforeEach(() => {
      mockRequest = {
        user: { userId: 'test-user' },
        ip: '127.0.0.1',
      };

      mockContext = {
        switchToHttp: jest.fn().mockReturnValue({
          getRequest: jest.fn().mockReturnValue(mockRequest),
        }),
      } as any;
    });

    it('should allow requests within rate limit', async () => {
      // Mock parent canActivate to return true
      jest.spyOn(guard.constructor.prototype.__proto__, 'canActivate')
        .mockResolvedValue(true);

      const result = await guard.canActivate(mockContext);
      expect(result).toBe(true);
    });

    it('should throw ForbiddenException when rate limit exceeded', async () => {
      const throttlerException = new ThrottlerException();
      throttlerException.getResponse = jest.fn().mockReturnValue({
        retryAfter: 60,
      });

      // Mock parent canActivate to throw ThrottlerException
      jest.spyOn(guard.constructor.prototype.__proto__, 'canActivate')
        .mockRejectedValue(throttlerException);

      await expect(guard.canActivate(mockContext)).rejects.toThrow(ForbiddenException);
    });

    it('should include user ID in error message when authenticated', async () => {
      const throttlerException = new ThrottlerException();
      throttlerException.getResponse = jest.fn().mockReturnValue({
        retryAfter: 60,
      });

      jest.spyOn(guard.constructor.prototype.__proto__, 'canActivate')
        .mockRejectedValue(throttlerException);

      try {
        await guard.canActivate(mockContext);
      } catch (error) {
        expect(error).toBeInstanceOf(ForbiddenException);
        expect(error.response.error).toBe('Rate limit exceeded');
        expect(error.response.message).toContain('test-user');
        expect(error.response.retryAfter).toBe(60);
      }
    });

    it('should include IP address in error message when not authenticated', async () => {
      mockRequest.user = null; // No authentication
      mockRequest.ip = '192.168.1.1';

      const throttlerException = new ThrottlerException();
      throttlerException.getResponse = jest.fn().mockReturnValue({
        retryAfter: 120,
      });

      jest.spyOn(guard.constructor.prototype.__proto__, 'canActivate')
        .mockRejectedValue(throttlerException);

      try {
        await guard.canActivate(mockContext);
      } catch (error) {
        expect(error).toBeInstanceOf(ForbiddenException);
        expect(error.response.message).toContain('192.168.1.1');
        expect(error.response.retryAfter).toBe(120);
      }
    });

    it('should pass through other exceptions', async () => {
      const otherError = new Error('Some other error');

      jest.spyOn(guard.constructor.prototype.__proto__, 'canActivate')
        .mockRejectedValue(otherError);

      await expect(guard.canActivate(mockContext)).rejects.toThrow('Some other error');
    });
  });

  describe('getTracker', () => {
    it('should return user ID when authenticated', () => {
      const mockRequest = {
        user: { userId: 'test-user-id' },
        ip: '127.0.0.1',
      };

      const result = (guard as any).getTracker(mockRequest);
      expect(result).toBe('test-user-id');
    });

    it('should return IP address when not authenticated', () => {
      const mockRequest = {
        user: null,
        ip: '192.168.1.100',
      };

      const result = (guard as any).getTracker(mockRequest);
      expect(result).toBe('192.168.1.100');
    });

    it('should return IP address when user ID is missing', () => {
      const mockRequest = {
        user: {},
        ip: '10.0.0.1',
      };

      const result = (guard as any).getTracker(mockRequest);
      expect(result).toBe('10.0.0.1');
    });
  });
});
