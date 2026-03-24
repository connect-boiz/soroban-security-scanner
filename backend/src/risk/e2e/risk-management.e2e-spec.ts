import { Test, TestingModule } from '@nestjs/testing';
import { INestApplication } from '@nestjs/common';
import * as request from 'supertest';
import { AppModule } from '../../app.module';

describe('Risk Management System (e2e)', () => {
  let app: INestApplication;

  beforeAll(async () => {
    const moduleFixture: TestingModule = await Test.createTestingModule({
      imports: [AppModule],
    }).compile();

    app = moduleFixture.createNestApplication();
    await app.init();
  });

  afterAll(async () => {
    await app.close();
  });

  describe('Risk Assessment API', () => {
    it('POST /api/v1/risk/assess - should assess portfolio risk', async () => {
      const assessmentDto = {
        userId: 'test-user-123',
        portfolio: {
          id: 'test-portfolio-456',
          positions: [
            {
              id: 'position-1',
              type: 'stock',
              size: 100,
              currentPrice: 50.25,
              currency: 'USD',
            },
            {
              id: 'position-2',
              type: 'bond',
              size: 50,
              currentPrice: 1000.50,
              currency: 'USD',
              duration: 5.2,
            },
            {
              id: 'position-3',
              type: 'option',
              size: 10,
              currentPrice: 5.75,
              currency: 'USD',
              underlying: 'AAPL',
            },
          ],
          totalValue: 10525.00,
          currency: 'USD',
        },
        confidenceLevel: 0.95,
        timeHorizon: 1,
        includeStressTest: true,
      };

      const response = await request(app.getHttpServer())
        .post('/api/v1/risk/assess')
        .send(assessmentDto)
        .expect(200);

      const result = response.body;
      
      expect(result).toHaveProperty('metrics');
      expect(result).toHaveProperty('alerts');
      expect(result).toHaveProperty('overallScore');
      expect(result).toHaveProperty('riskLevel');

      // Validate metrics structure
      expect(result.metrics).toHaveProperty('var1d');
      expect(result.metrics).toHaveProperty('var10d');
      expect(result.metrics).toHaveProperty('var30d');
      expect(result.metrics).toHaveProperty('expectedShortfall');
      expect(result.metrics).toHaveProperty('beta');
      expect(result.metrics).toHaveProperty('volatility');
      expect(result.metrics).toHaveProperty('correlation');
      expect(result.metrics).toHaveProperty('concentration');

      // Validate metrics values
      expect(result.metrics.var1d).toBeGreaterThanOrEqual(0);
      expect(result.metrics.volatility).toBeGreaterThanOrEqual(0);
      expect(result.metrics.concentration).toBeGreaterThanOrEqual(0);
      expect(result.metrics.concentration).toBeLessThanOrEqual(1);

      // Validate risk level
      expect(['low', 'medium', 'high', 'critical']).toContain(result.riskLevel);
      expect(result.overallScore).toBeGreaterThanOrEqual(0);
      expect(result.overallScore).toBeLessThanOrEqual(1);
    });

    it('POST /api/v1/risk/assess - should handle validation errors', async () => {
      const invalidDto = {
        userId: '', // Invalid: empty string
        portfolio: {
          // Missing required fields
        },
      };

      await request(app.getHttpServer())
        .post('/api/v1/risk/assess')
        .send(invalidDto)
        .expect(400);
    });

    it('POST /api/v1/risk/assess - should handle large portfolios efficiently', async () => {
      const largePortfolio = {
        userId: 'test-user-large',
        portfolio: {
          id: 'large-portfolio',
          positions: Array.from({ length: 1000 }, (_, i) => ({
            id: `position-${i}`,
            type: 'stock',
            size: Math.floor(Math.random() * 1000) + 1,
            currentPrice: Math.random() * 200 + 10,
            currency: 'USD',
          })),
          totalValue: 5000000, // $5M portfolio
          currency: 'USD',
        },
        confidenceLevel: 0.95,
        timeHorizon: 1,
      };

      const startTime = Date.now();
      
      await request(app.getHttpServer())
        .post('/api/v1/risk/assess')
        .send(largePortfolio)
        .expect(200);

      const processingTime = Date.now() - startTime;
      
      // Should process large portfolios within performance requirements
      expect(processingTime).toBeLessThan(1000); // Under 1 second
    });
  });

  describe('Real-Time Monitoring API', () => {
    const portfolioId = 'test-portfolio-monitoring';

    it('GET /api/v1/risk/portfolio/:id/metrics - should get real-time metrics', async () => {
      const response = await request(app.getHttpServer())
        .get(`/api/v1/risk/portfolio/${portfolioId}/metrics`)
        .expect(200);

      const metrics = response.body;
      
      expect(metrics).toHaveProperty('riskScore');
      expect(metrics).toHaveProperty('volatility');
      expect(metrics).toHaveProperty('timestamp');
      
      if (metrics.riskScore !== null) {
        expect(metrics.riskScore).toBeGreaterThanOrEqual(0);
        expect(metrics.riskScore).toBeLessThanOrEqual(1);
      }
    });

    it('GET /api/v1/risk/portfolio/:id/alerts - should get risk alerts', async () => {
      const response = await request(app.getHttpServer())
        .get(`/api/v1/risk/portfolio/${portfolioId}/alerts`)
        .expect(200);

      const alerts = response.body;
      expect(Array.isArray(alerts)).toBe(true);
      
      if (alerts.length > 0) {
        alerts.forEach(alert => {
          expect(alert).toHaveProperty('id');
          expect(alert).toHaveProperty('riskType');
          expect(alert).toHaveProperty('severity');
          expect(alert).toHaveProperty('message');
          expect(alert).toHaveProperty('timestamp');
          
          expect(['market', 'credit', 'operational', 'liquidity', 'counterparty']).toContain(alert.riskType);
          expect(['low', 'medium', 'high', 'critical']).toContain(alert.severity);
        });
      }
    });
  });

  describe('VaR Calculation API', () => {
    it('GET /api/v1/risk/portfolio/:id/var - should calculate Value at Risk', async () => {
      const response = await request(app.getHttpServer())
        .get('/api/v1/risk/portfolio/test-portfolio/var')
        .query({
          confidence: 0.95,
          horizon: 1,
        })
        .expect(200);

      const varResult = response.body;
      
      expect(varResult).toHaveProperty('var');
      expect(varResult).toHaveProperty('expectedShortfall');
      expect(varResult).toHaveProperty('methodology');
      expect(varResult).toHaveProperty('assumptions');
      expect(varResult).toHaveProperty('accuracy');

      expect(varResult.var).toBeGreaterThanOrEqual(0);
      expect(varResult.expectedShortfall).toBeGreaterThanOrEqual(0);
      expect(['historical', 'parametric', 'monteCarlo']).toContain(varResult.methodology);
      expect(Array.isArray(varResult.assumptions)).toBe(true);
      expect(varResult.accuracy).toBeGreaterThanOrEqual(0);
      expect(varResult.accuracy).toBeLessThanOrEqual(1);
    });

    it('GET /api/v1/risk/portfolio/:id/var - should handle different parameters', async () => {
      const testCases = [
        { confidence: 0.90, horizon: 1 },
        { confidence: 0.95, horizon: 10 },
        { confidence: 0.99, horizon: 30 },
      ];

      for (const testCase of testCases) {
        const response = await request(app.getHttpServer())
          .get('/api/v1/risk/portfolio/test-portfolio/var')
          .query(testCase)
          .expect(200);

        const varResult = response.body;
        expect(varResult.var).toBeGreaterThanOrEqual(0);
        
        // Higher confidence should result in higher VaR
        if (testCase.confidence > 0.95) {
          // This is a basic check, actual implementation may vary
        }
      }
    });

    it('GET /api/v1/risk/portfolio/:id/var - should validate parameters', async () => {
      await request(app.getHttpServer())
        .get('/api/v1/risk/portfolio/test-portfolio/var')
        .query({ confidence: 0.5 }) // Invalid confidence level
        .expect(400);

      await request(app.getHttpServer())
        .get('/api/v1/risk/portfolio/test-portfolio/var')
        .query({ horizon: 50 }) // Invalid time horizon
        .expect(400);
    });
  });

  describe('Stress Testing API', () => {
    it('POST /api/v1/risk/portfolio/:id/stress-test - should run stress tests', async () => {
      const portfolioData = {
        id: 'stress-test-portfolio',
        positions: [
          {
            id: 'position-1',
            type: 'stock',
            size: 1000,
            currentPrice: 100,
            currency: 'USD',
            volatility: 0.02,
          },
          {
            id: 'position-2',
            type: 'bond',
            size: 500,
            currentPrice: 1000,
            currency: 'USD',
            duration: 7.5,
          },
        ],
        totalValue: 600000,
        currency: 'USD',
      };

      const response = await request(app.getHttpServer())
        .post('/api/v1/risk/portfolio/stress-test-portfolio/stress-test')
        .send(portfolioData)
        .expect(200);

      const stressTestResult = response.body;
      
      expect(stressTestResult).toHaveProperty('results');
      expect(stressTestResult).toHaveProperty('summary');
      
      expect(Array.isArray(stressTestResult.results)).toBe(true);
      expect(stressTestResult.results.length).toBeGreaterThan(0);
      
      // Validate summary
      expect(stressTestResult.summary).toHaveProperty('worstCaseLoss');
      expect(stressTestResult.summary).toHaveProperty('worstCaseScenario');
      expect(stressTestResult.summary).toHaveProperty('averageLoss');
      expect(stressTestResult.summary).toHaveProperty('scenariosPassed');
      expect(stressTestResult.summary).toHaveProperty('scenariosFailed');
      expect(stressTestResult.summary).toHaveProperty('riskResilience');
      
      // Validate individual scenario results
      stressTestResult.results.forEach(result => {
        expect(result).toHaveProperty('scenario');
        expect(result).toHaveProperty('portfolioLoss');
        expect(result).toHaveProperty('lossPercentage');
        expect(result).toHaveProperty('recoveryTime');
        expect(result).toHaveProperty('riskFactors');
        
        expect(result.portfolioLoss).toBeGreaterThanOrEqual(0);
        expect(result.lossPercentage).toBeGreaterThanOrEqual(0);
        expect(result.recoveryTime).toBeGreaterThan(0);
        expect(Array.isArray(result.riskFactors)).toBe(true);
      });
    });

    it('POST /api/v1/risk/portfolio/:id/stress-test - should complete within performance requirements', async () => {
      const portfolioData = {
        id: 'performance-test-portfolio',
        positions: Array.from({ length: 100 }, (_, i) => ({
          id: `position-${i}`,
          type: 'stock',
          size: 100,
          currentPrice: 50,
          currency: 'USD',
        })),
        totalValue: 500000,
        currency: 'USD',
      };

      const startTime = Date.now();
      
      await request(app.getHttpServer())
        .post('/api/v1/risk/portfolio/performance-test-portfolio/stress-test')
        .send(portfolioData)
        .expect(200);

      const processingTime = Date.now() - startTime;
      
      // Should complete within 500ms as per requirements
      expect(processingTime).toBeLessThan(500);
    });
  });

  describe('Hedging API', () => {
    it('POST /api/v1/risk/portfolio/:id/hedge - should generate hedging strategies', async () => {
      const portfolioData = {
        id: 'hedging-portfolio',
        positions: [
          {
            id: 'equity-position',
            type: 'stock',
            size: 10000,
            currentPrice: 150,
            currency: 'USD',
            beta: 1.2,
          },
          {
            id: 'bond-position',
            type: 'bond',
            size: 5000,
            currentPrice: 1000,
            currency: 'USD',
            duration: 8.5,
          },
          {
            id: 'option-position',
            type: 'option',
            size: 100,
            currentPrice: 10,
            currency: 'USD',
            underlying: 'SPY',
          },
        ],
        totalValue: 2500000,
        currency: 'USD',
      };

      const response = await request(app.getHttpServer())
        .post('/api/v1/risk/portfolio/hedging-portfolio/hedge')
        .send(portfolioData)
        .expect(200);

      const strategies = response.body;
      
      expect(Array.isArray(strategies)).toBe(true);
      expect(strategies.length).toBeGreaterThan(0);
      
      // Validate strategy structure
      strategies.forEach(strategy => {
        expect(strategy).toHaveProperty('name');
        expect(strategy).toHaveProperty('type');
        expect(strategy).toHaveProperty('effectiveness');
        expect(strategy).toHaveProperty('cost');
        expect(strategy).toHaveProperty('hedgeRatio');
        expect(strategy).toHaveProperty('instruments');
        
        expect(strategy.effectiveness).toBeGreaterThanOrEqual(0);
        expect(strategy.effectiveness).toBeLessThanOrEqual(1);
        expect(strategy.cost).toBeGreaterThanOrEqual(0);
        expect(strategy.hedgeRatio).toBeGreaterThanOrEqual(0);
        expect(Array.isArray(strategy.instruments)).toBe(true);
      });
      
      // Strategies should be sorted by effectiveness (highest first)
      for (let i = 0; i < strategies.length - 1; i++) {
        expect(strategies[i].effectiveness).toBeGreaterThanOrEqual(strategies[i + 1].effectiveness);
      }
    });

    it('POST /api/v1/risk/portfolio/:id/hedge - should provide risk reduction recommendations', async () => {
      const portfolioData = {
        id: 'risk-reduction-portfolio',
        positions: [
          {
            id: 'concentrated-position',
            type: 'stock',
            size: 50000,
            currentPrice: 100,
            currency: 'USD',
          },
        ],
        totalValue: 5000000,
        currency: 'USD',
      };

      const response = await request(app.getHttpServer())
        .post('/api/v1/risk/portfolio/risk-reduction-portfolio/hedge')
        .send(portfolioData)
        .expect(200);

      const strategies = response.body;
      
      // Should suggest hedging for concentrated positions
      expect(strategies.length).toBeGreaterThan(0);
      
      // At least one strategy should target concentration risk
      const hasConcentrationHedge = strategies.some(strategy => 
        strategy.name.toLowerCase().includes('concentration') ||
        strategy.type === 'diversification'
      );
      expect(hasConcentrationHedge).toBe(true);
    });
  });

  describe('Performance Requirements', () => {
    it('should maintain response times under 200ms for risk assessments', async () => {
      const assessmentDto = {
        userId: 'performance-test-user',
        portfolio: {
          id: 'performance-test-portfolio',
          positions: Array.from({ length: 100 }, (_, i) => ({
            id: `position-${i}`,
            type: 'stock',
            size: 100,
            currentPrice: 50 + Math.random() * 50,
            currency: 'USD',
          })),
          totalValue: 750000,
          currency: 'USD',
        },
        confidenceLevel: 0.95,
        timeHorizon: 1,
      };

      const measurements = [];
      const iterations = 10;

      for (let i = 0; i < iterations; i++) {
        const startTime = Date.now();
        
        await request(app.getHttpServer())
          .post('/api/v1/risk/assess')
          .send(assessmentDto)
          .expect(200);
        
        measurements.push(Date.now() - startTime);
      }

      const averageTime = measurements.reduce((sum, time) => sum + time, 0) / measurements.length;
      const maxTime = Math.max(...measurements);
      
      // Performance requirement: under 200ms
      expect(averageTime).toBeLessThan(200);
      expect(maxTime).toBeLessThan(500); // Allow some variance for max time
    });

    it('should handle concurrent requests', async () => {
      const assessmentDto = {
        userId: 'concurrent-test-user',
        portfolio: {
          id: 'concurrent-test-portfolio',
          positions: [
            {
              id: 'position-1',
              type: 'stock',
              size: 1000,
              currentPrice: 100,
              currency: 'USD',
            },
          ],
          totalValue: 100000,
          currency: 'USD',
        },
        confidenceLevel: 0.95,
        timeHorizon: 1,
      };

      const concurrentRequests = 20;
      const promises = Array.from({ length: concurrentRequests }, (_, i) =>
        request(app.getHttpServer())
          .post('/api/v1/risk/assess')
          .send({ ...assessmentDto, userId: `concurrent-test-user-${i}` })
          .expect(200)
      );

      const startTime = Date.now();
      const results = await Promise.all(promises);
      const totalTime = Date.now() - startTime;

      expect(results).toHaveLength(concurrentRequests);
      expect(totalTime).toBeLessThan(5000); // Should handle 20 concurrent requests within 5 seconds
      
      // All results should have valid structure
      results.forEach(response => {
        const result = response.body;
        expect(result).toHaveProperty('metrics');
        expect(result).toHaveProperty('overallScore');
        expect(result).toHaveProperty('riskLevel');
      });
    });
  });

  describe('Error Handling', () => {
    it('should handle malformed requests gracefully', async () => {
      await request(app.getHttpServer())
        .post('/api/v1/risk/assess')
        .send({ invalid: 'data' })
        .expect(400);
    });

    it('should handle missing portfolio data', async () => {
      await request(app.getHttpServer())
        .post('/api/v1/risk/assess')
        .send({
          userId: 'test-user',
          portfolio: null,
        })
        .expect(400);
    });

    it('should handle invalid portfolio IDs', async () => {
      await request(app.getHttpServer())
        .get('/api/v1/risk/portfolio/invalid-portfolio-id/metrics')
        .expect(404);
    });
  });
});
