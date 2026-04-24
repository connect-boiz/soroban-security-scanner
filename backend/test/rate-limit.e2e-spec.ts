import { Test, TestingModule } from '@nestjs/testing';
import { INestApplication, ValidationPipe } from '@nestjs/common';
import { AppModule } from '../src/app.module';
import { ThrottlerModule } from '@nestjs/throttler';
import * as request from 'supertest';

describe('Rate Limiting (e2e)', () => {
  let app: INestApplication;

  beforeAll(async () => {
    const moduleFixture: TestingModule = await Test.createTestingModule({
      imports: [
        AppModule,
        ThrottlerModule.forRoot([
          {
            name: 'test',
            ttl: 1000, // 1 second for faster testing
            limit: 2, // 2 requests per second
          },
        ]),
      ],
    }).compile();

    app = moduleFixture.createNestApplication();
    app.useGlobalPipes(new ValidationPipe());
    await app.init();
  });

  afterAll(async () => {
    await app.close();
  });

  describe('Vulnerability Reporting Rate Limit', () => {
    it('should allow requests within rate limit', async () => {
      // First request should succeed
      const response1 = await request(app.getHttpServer())
        .post('/llm-patch/generate')
        .set('Authorization', 'Bearer valid-token')
        .send({
          vulnerability: {
            id: 'test-1',
            vulnerability_type: 'test',
            severity: 'medium',
            title: 'Test Vulnerability',
            description: 'Test description',
            code_snippet: 'test code',
            line_number: 1,
          },
          original_code: 'test code',
        })
        .expect(401); // Will fail due to auth, but should not be rate limited

      // Second request should also succeed (within limit)
      const response2 = await request(app.getHttpServer())
        .post('/llm-patch/generate')
        .set('Authorization', 'Bearer valid-token')
        .send({
          vulnerability: {
            id: 'test-2',
            vulnerability_type: 'test',
            severity: 'medium',
            title: 'Test Vulnerability',
            description: 'Test description',
            code_snippet: 'test code',
            line_number: 1,
          },
          original_code: 'test code',
        })
        .expect(401); // Will fail due to auth, but should not be rate limited

      expect(response1.status).toBe(401);
      expect(response2.status).toBe(401);
    });

    it('should rate limit excessive requests', async () => {
      // Make multiple requests quickly to trigger rate limit
      const promises = Array(5).fill(null).map((_, i) =>
        request(app.getHttpServer())
          .post('/llm-patch/generate')
          .set('Authorization', 'Bearer valid-token')
          .send({
            vulnerability: {
              id: `test-${i}`,
              vulnerability_type: 'test',
              severity: 'medium',
              title: 'Test Vulnerability',
              description: 'Test description',
              code_snippet: 'test code',
              line_number: 1,
            },
            original_code: 'test code',
          })
      );

      const responses = await Promise.all(promises);
      
      // At least some responses should be rate limited
      const rateLimitedResponses = responses.filter(res => res.status === 403);
      expect(rateLimitedResponses.length).toBeGreaterThan(0);
      
      // Check rate limit response format
      const rateLimitedResponse = rateLimitedResponses[0];
      expect(rateLimitedResponse.body).toHaveProperty('error', 'Rate limit exceeded');
      expect(rateLimitedResponse.body).toHaveProperty('message');
      expect(rateLimitedResponse.body).toHaveProperty('retryAfter');
      expect(rateLimitedResponse.headers).toHaveProperty('retry-after');
    });
  });

  describe('Escrow Creation Rate Limit', () => {
    it('should allow escrow creation within rate limit', async () => {
      const response1 = await request(app.getHttpServer())
        .post('/escrow')
        .set('Authorization', 'Bearer valid-token')
        .send({
          beneficiary: '0x1234567890123456789012345678901234567890',
          amount: 100,
          token: '0x1234567890123456789012345678901234567890',
          purpose: 'bounty',
        })
        .expect(401); // Will fail due to auth, but should not be rate limited

      const response2 = await request(app.getHttpServer())
        .post('/escrow')
        .set('Authorization', 'Bearer valid-token')
        .send({
          beneficiary: '0x1234567890123456789012345678901234567890',
          amount: 200,
          token: '0x1234567890123456789012345678901234567890',
          purpose: 'reward',
        })
        .expect(401); // Will fail due to auth, but should not be rate limited

      expect(response1.status).toBe(401);
      expect(response2.status).toBe(401);
    });

    it('should rate limit excessive escrow creation', async () => {
      // Make multiple escrow creation requests quickly
      const promises = Array(5).fill(null).map((_, i) =>
        request(app.getHttpServer())
          .post('/escrow')
          .set('Authorization', 'Bearer valid-token')
          .send({
            beneficiary: '0x1234567890123456789012345678901234567890',
            amount: 100 + i,
            token: '0x1234567890123456789012345678901234567890',
            purpose: 'bounty',
          })
      );

      const responses = await Promise.all(promises);
      
      // At least some responses should be rate limited
      const rateLimitedResponses = responses.filter(res => res.status === 403);
      expect(rateLimitedResponses.length).toBeGreaterThan(0);
    });
  });

  describe('Batch Operations Rate Limit', () => {
    it('should rate limit batch operations', async () => {
      // Make multiple batch requests quickly
      const promises = Array(5).fill(null).map((_, i) =>
        request(app.getHttpServer())
          .post('/llm-patch/batch-generate')
          .set('Authorization', 'Bearer valid-token')
          .send({
            requests: [
              {
                vulnerability: {
                  id: `batch-test-${i}-1`,
                  vulnerability_type: 'test',
                  severity: 'medium',
                  title: 'Test Vulnerability',
                  description: 'Test description',
                  code_snippet: 'test code',
                  line_number: 1,
                },
                original_code: 'test code',
              },
            ],
          })
      );

      const responses = await Promise.all(promises);
      
      // At least some responses should be rate limited
      const rateLimitedResponses = responses.filter(res => res.status === 403);
      expect(rateLimitedResponses.length).toBeGreaterThan(0);
    });
  });

  describe('Rate Limit Headers', () => {
    it('should include rate limit headers', async () => {
      const response = await request(app.getHttpServer())
        .post('/llm-patch/generate')
        .set('Authorization', 'Bearer valid-token')
        .send({
          vulnerability: {
            id: 'headers-test',
            vulnerability_type: 'test',
            severity: 'medium',
            title: 'Test Vulnerability',
            description: 'Test description',
            code_snippet: 'test code',
            line_number: 1,
          },
          original_code: 'test code',
        })
        .expect(401);

      // Check for rate limit headers (these should be added by our interceptor)
      expect(response.headers).toHaveProperty('x-ratelimit-limit');
      expect(response.headers).toHaveProperty('x-ratelimit-ttl');
    });
  });
});
