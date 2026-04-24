export interface RateLimitConfig {
  default: {
    ttl: number;
    limit: number;
  };
  vulnerabilityReporting: {
    ttl: number;
    limit: number;
  };
  escrowCreation: {
    ttl: number;
    limit: number;
  };
  batchOperations: {
    ttl: number;
    limit: number;
  };
  scanOperations: {
    ttl: number;
    limit: number;
  };
}

export const defaultRateLimitConfig: RateLimitConfig = {
  default: {
    ttl: 60000, // 1 minute
    limit: 100, // 100 requests per minute
  },
  vulnerabilityReporting: {
    ttl: 60000, // 1 minute
    limit: 10, // 10 vulnerability reports per minute
  },
  escrowCreation: {
    ttl: 60000, // 1 minute
    limit: 5, // 5 escrow creations per minute
  },
  batchOperations: {
    ttl: 300000, // 5 minutes
    limit: 3, // 3 batch operations per 5 minutes
  },
  scanOperations: {
    ttl: 60000, // 1 minute
    limit: 20, // 20 scans per minute
  },
};

export const getRateLimitConfig = (): RateLimitConfig => {
  // In production, this would load from environment variables or config service
  return {
    default: {
      ttl: parseInt(process.env.RATE_LIMIT_DEFAULT_TTL || '60000'),
      limit: parseInt(process.env.RATE_LIMIT_DEFAULT_LIMIT || '100'),
    },
    vulnerabilityReporting: {
      ttl: parseInt(process.env.RATE_LIMIT_VULN_REPORTING_TTL || '60000'),
      limit: parseInt(process.env.RATE_LIMIT_VULN_REPORTING_LIMIT || '10'),
    },
    escrowCreation: {
      ttl: parseInt(process.env.RATE_LIMIT_ESCROW_CREATION_TTL || '60000'),
      limit: parseInt(process.env.RATE_LIMIT_ESCROW_CREATION_LIMIT || '5'),
    },
    batchOperations: {
      ttl: parseInt(process.env.RATE_LIMIT_BATCH_TTL || '300000'),
      limit: parseInt(process.env.RATE_LIMIT_BATCH_LIMIT || '3'),
    },
    scanOperations: {
      ttl: parseInt(process.env.RATE_LIMIT_SCAN_TTL || '60000'),
      limit: parseInt(process.env.RATE_LIMIT_SCAN_LIMIT || '20'),
    },
  };
};
