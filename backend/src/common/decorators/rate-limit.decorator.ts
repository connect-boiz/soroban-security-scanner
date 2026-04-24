import { SetMetadata } from '@nestjs/common';

export interface RateLimitOptions {
  ttl?: number; // Time to live in milliseconds
  limit?: number; // Number of requests allowed
  ignoreUserAgents?: string[]; // User agents to ignore
  skipSuccessful?: boolean; // Skip counting successful requests
  skipFailed?: boolean; // Skip counting failed requests
}

export const RATE_LIMIT_KEY = 'rate_limit';

export const RateLimit = (options: RateLimitOptions = {}) => 
  SetMetadata(RATE_LIMIT_KEY, options);

// Predefined rate limit decorators for common use cases
export const VulnerabilityReportRateLimit = () => 
  RateLimit({
    ttl: 60000, // 1 minute
    limit: 10, // 10 vulnerability reports per minute
    skipFailed: true, // Don't count failed reports
  });

export const EscrowCreationRateLimit = () => 
  RateLimit({
    ttl: 60000, // 1 minute  
    limit: 5, // 5 escrow creations per minute
    skipFailed: true, // Don't count failed creations
  });

export const BatchOperationRateLimit = () => 
  RateLimit({
    ttl: 300000, // 5 minutes
    limit: 3, // 3 batch operations per 5 minutes
    skipFailed: true,
  });

export const ScanRateLimit = () => 
  RateLimit({
    ttl: 60000, // 1 minute
    limit: 20, // 20 scans per minute
    skipFailed: true,
  });
