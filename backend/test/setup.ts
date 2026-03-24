import { ConfigModule } from '@nestjs/config';
import { Test } from '@nestjs/testing';

// Global test setup
beforeAll(async () => {
  // Set test environment variables
  process.env.NODE_ENV = 'test';
  process.env.DATABASE_URL = 'postgresql://scanner_user:scanner_pass@localhost:5432/soroban_scanner_test';
  process.env.REDIS_URL = 'redis://localhost:6379';
  process.env.JWT_SECRET = 'test_secret_key';
  process.env.DATABASE_SYNCHRONIZE = 'true';
});

// Mock console methods to reduce test noise
global.console = {
  ...console,
  log: jest.fn(),
  debug: jest.fn(),
  info: jest.fn(),
  warn: jest.fn(),
  error: jest.fn(),
};

// Mock Redis for tests
jest.mock('@nestjs/redis', () => ({
  Redis: jest.fn().mockImplementation(() => ({
    get: jest.fn(),
    set: jest.fn(),
    setex: jest.fn(),
    del: jest.fn(),
    lpush: jest.fn(),
    lrange: jest.fn(),
    ltrim: jest.fn(),
    expire: jest.fn(),
    keys: jest.fn(),
    publish: jest.fn(),
  })),
}));

// Mock TypeORM repository
jest.mock('@nestjs/typeorm', () => ({
  TypeOrmModule: {
    forFeature: jest.fn(),
    forRootAsync: jest.fn(),
  },
  InjectRepository: jest.fn(),
}));

// Mock ConfigService
jest.mock('@nestjs/config', () => ({
  ConfigModule: {
   .forRoot: jest.fn(),
  },
  ConfigService: jest.fn().mockImplementation(() => ({
    get: jest.fn((key: string) => {
      const defaults = {
        DATABASE_URL: 'postgresql://scanner_user:scanner_pass@localhost:5432/soroban_scanner_test',
        REDIS_URL: 'redis://localhost:6379',
        JWT_SECRET: 'test_secret_key',
        NODE_ENV: 'test',
        RISK_MONITORING_INTERVAL: '10000',
        RISK_ALERT_THRESHOLDS_VAR: '100000',
        RISK_VAR_CONFIDENCE_LEVEL: '0.95',
      };
      return defaults[key] || null;
    }),
  })),
}));

// Add custom matchers
expect.extend({
  toBeWithinRange(received: number, floor: number, ceiling: number) {
    const pass = received >= floor && received <= ceiling;
    if (pass) {
      return {
        message: () =>
          `expected ${received} not to be within range ${floor} - ${ceiling}`,
        pass: true,
      };
    } else {
      return {
        message: () =>
          `expected ${received} to be within range ${floor} - ${ceiling}`,
        pass: false,
      };
    }
  },
});

declare global {
  namespace jest {
    interface Matchers<R> {
      toBeWithinRange(floor: number, ceiling: number): R;
    }
  }
}
