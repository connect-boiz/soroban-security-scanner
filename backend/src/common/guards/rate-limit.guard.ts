import { Injectable, ExecutionContext, ForbiddenException } from '@nestjs/common';
import { Reflector } from '@nestjs/core';
import { ThrottlerGuard, ThrottlerException } from '@nestjs/throttler';
import { Request } from 'express';

@Injectable()
export class CustomRateLimitGuard extends ThrottlerGuard {
  constructor(
    protected readonly reflector: Reflector,
  ) {
    super(reflector);
  }

  async canActivate(context: ExecutionContext): Promise<boolean> {
    try {
      return await super.canActivate(context) as boolean;
    } catch (error) {
      if (error instanceof ThrottlerException) {
        const request = context.switchToHttp().getRequest<Request>();
        const userId = request.user?.userId || request.ip;
        
        throw new ForbiddenException({
          error: 'Rate limit exceeded',
          message: `Too many requests from ${userId}. Please try again later.`,
          retryAfter: error.getResponse?.()['retryAfter'] || 60,
        });
      }
      throw error;
    }
  }

  protected getTracker(req: Record<string, any>): string {
    // Use user ID if authenticated, otherwise fall back to IP
    return req.user?.userId || req.ip;
  }
}
