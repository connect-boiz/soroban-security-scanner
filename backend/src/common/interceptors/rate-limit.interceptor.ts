import {
  Injectable,
  NestInterceptor,
  ExecutionContext,
  CallHandler,
  ForbiddenException,
} from '@nestjs/common';
import { Observable, throwError } from 'rxjs';
import { catchError, tap } from 'rxjs/operators';
import { Reflector } from '@nestjs/core';
import { RATE_LIMIT_KEY, RateLimitOptions } from '../decorators/rate-limit.decorator';
import { Request } from 'express';

@Injectable()
export class RateLimitInterceptor implements NestInterceptor {
  constructor(private readonly reflector: Reflector) {}

  intercept(context: ExecutionContext, next: CallHandler): Observable<any> {
    const rateLimitOptions = this.reflector.get<RateLimitOptions>(
      RATE_LIMIT_KEY,
      context.getHandler(),
    );

    if (!rateLimitOptions) {
      return next.handle();
    }

    const request = context.switchToHttp().getRequest<Request>();
    const response = context.switchToHttp().getResponse();

    // Add rate limit headers
    response.setHeader('X-RateLimit-Limit', rateLimitOptions.limit || 100);
    response.setHeader('X-RateLimit-TTL', rateLimitOptions.ttl || 60000);

    return next.handle().pipe(
      tap(() => {
        // On successful response, set remaining requests
        response.setHeader('X-RateLimit-Remaining', 'calculating...');
      }),
      catchError((error) => {
        if (error instanceof ForbiddenException && error.message?.includes('Rate limit exceeded')) {
          // Add custom headers for rate limit exceeded
          response.setHeader('Retry-After', error.getResponse?.()['retryAfter'] || 60);
          response.setHeader('X-RateLimit-Remaining', '0');
        }
        return throwError(() => error);
      }),
    );
  }
}
