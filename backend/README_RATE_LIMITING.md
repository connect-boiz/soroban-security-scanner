# Rate Limiting Implementation

This document describes the rate limiting implementation for the Soroban Security Scanner backend, specifically addressing issue #118 for vulnerability reporting and escrow creation.

## Overview

The rate limiting system uses `@nestjs/throttler` with Redis storage to provide configurable rate limits across different API endpoints. The implementation includes:

- **Vulnerability Reporting**: 10 requests per minute
- **Escrow Creation**: 5 requests per minute  
- **Batch Operations**: 3 requests per 5 minutes
- **Scan Operations**: 20 requests per minute
- **Default**: 100 requests per minute

## Architecture

### Components

1. **CustomRateLimitGuard** (`src/common/guards/rate-limit.guard.ts`)
   - Extends NestJS ThrottlerGuard
   - Provides user-friendly error messages
   - Uses user ID for authenticated users, IP address for anonymous users

2. **Rate Limit Decorators** (`src/common/decorators/rate-limit.decorator.ts`)
   - Predefined decorators for common use cases
   - Configurable TTL and limits
   - Options to skip failed/successful requests

3. **Rate Limit Interceptor** (`src/common/interceptors/rate-limit.interceptor.ts`)
   - Adds rate limit headers to responses
   - Provides visibility into remaining requests

4. **Configuration** (`src/common/config/rate-limit.config.ts`)
   - Environment-based configuration
   - Centralized rate limit settings

### Implementation Details

#### Vulnerability Reporting Rate Limits

```typescript
@Throttle(10, 60) // 10 requests per minute
@VulnerabilityReportRateLimit()
async generatePatch(...) {
  // Implementation
}
```

#### Escrow Creation Rate Limits

```typescript
@Throttle(5, 60) // 5 requests per minute
@EscrowCreationRateLimit()
async createEscrow(...) {
  // Implementation
}
```

## Configuration

### Environment Variables

Add these to your `.env` file:

```bash
# Rate Limiting Configuration
RATE_LIMIT_DEFAULT_TTL=60000
RATE_LIMIT_DEFAULT_LIMIT=100

# Vulnerability reporting rate limits
RATE_LIMIT_VULN_REPORTING_TTL=60000
RATE_LIMIT_VULN_REPORTING_LIMIT=10

# Escrow creation rate limits
RATE_LIMIT_ESCROW_CREATION_TTL=60000
RATE_LIMIT_ESCROW_CREATION_LIMIT=5

# Batch operations rate limits
RATE_LIMIT_BATCH_TTL=300000
RATE_LIMIT_BATCH_LIMIT=3

# Scan operations rate limits
RATE_LIMIT_SCAN_TTL=60000
RATE_LIMIT_SCAN_LIMIT=20
```

### Redis Configuration

Rate limiting requires Redis for distributed storage:

```bash
REDIS_URL=redis://localhost:6379
REDIS_KEY_PREFIX=soroban_scanner:
```

## API Endpoints with Rate Limiting

### Vulnerability Reporting
- `POST /llm-patch/generate` - 10 requests/minute
- `POST /llm-patch/batch-generate` - 3 requests/5 minutes

### Escrow Management
- `POST /escrow` - 5 requests/minute
- `POST /escrow/:id/release` - 10 requests/minute

### Scan Operations
- `POST /scan` - 20 requests/minute (existing)

## Error Responses

When rate limits are exceeded, the API returns:

```json
{
  "error": "Rate limit exceeded",
  "message": "Too many requests from user_123. Please try again later.",
  "retryAfter": 60
}
```

With HTTP headers:
- `Retry-After`: 60
- `X-RateLimit-Remaining`: 0

## Testing

### Unit Tests
- Custom rate limit guard tests
- Decorator functionality tests
- Configuration loading tests

### E2E Tests
- Rate limit enforcement tests
- Header validation tests
- Error response format tests

Run tests:
```bash
npm run test rate-limit.e2e-spec.ts
npm run test src/common/guards/rate-limit.guard.spec.ts
```

## Monitoring

Rate limiting can be monitored through:

1. **Redis Keys**: Monitor rate limit keys in Redis
2. **Application Logs**: Rate limit violations are logged
3. **Response Headers**: Client applications can monitor rate limit headers

## Security Considerations

1. **User Identification**: Uses authenticated user ID when available, falls back to IP
2. **Distributed Storage**: Redis ensures rate limits work across multiple instances
3. **Configurable Limits**: Easy to adjust limits based on load
4. **Graceful Degradation**: Failed requests don't count against limits (configurable)

## Future Enhancements

1. **Dynamic Rate Limiting**: Adjust limits based on user reputation
2. **Burst Capacity**: Allow short bursts within limits
3. **Rate Limit Analytics**: Dashboard for monitoring rate limit usage
4. **Sliding Window**: More sophisticated rate limiting algorithms

## Troubleshooting

### Common Issues

1. **Redis Connection**: Ensure Redis is running and accessible
2. **Environment Variables**: Check all rate limit environment variables are set
3. **Time Synchronization**: Ensure all servers have synchronized time
4. **Memory Usage**: Monitor Redis memory usage for high-traffic scenarios

### Debug Mode

Enable debug logging:
```bash
LOG_LEVEL=debug
```

This will show rate limit decisions and Redis operations in the logs.
