import { Test, TestingModule } from '@nestjs/testing';
import { HealthController } from './health.controller';
import { HealthModule } from './health.module';

describe('HealthController', () => {
  let controller: HealthController;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      imports: [HealthModule],
    }).compile();

    controller = module.get<HealthController>(HealthController);
  });

  it('should be defined', () => {
    expect(controller).toBeDefined();
  });

  it('should return health status', () => {
    const result = controller.check();
    expect(result).toHaveProperty('status');
    expect(result).toHaveProperty('timestamp');
    expect(result).toHaveProperty('uptime');
    expect(result.status).toBe('ok');
  });
});
