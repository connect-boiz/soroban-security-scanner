import { Test, TestingModule } from '@nestjs/testing';
import { ConfigService } from '@nestjs/config';
import { StateConsistencyValidator, StateValidationError } from './state-consistency.validator';

describe('StateConsistencyValidator', () => {
  let validator: StateConsistencyValidator;
  let configService: ConfigService;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [
        StateConsistencyValidator,
        {
          provide: ConfigService,
          useValue: {
            get: jest.fn(),
          },
        },
      ],
    }).compile();

    validator = module.get<StateConsistencyValidator>(StateConsistencyValidator);
    configService = module.get<ConfigService>(ConfigService);
  });

  it('should be defined', () => {
    expect(validator).toBeDefined();
  });

  describe('validateStateTransition', () => {
    it('should allow valid scan state transition: pending -> queued', async () => {
      const scan = {
        id: 'scan-123',
        userId: 'user-123',
        code: 'contract code',
      };

      const result = await validator.validateStateTransition(
        'scan',
        'scan-123',
        'pending',
        'queued',
        scan,
        { userId: 'user-123' }
      );

      expect(result.valid).toBe(true);
      expect(result.error).toBeUndefined();
    });

    it('should reject invalid scan state transition: completed -> running', async () => {
      const scan = {
        id: 'scan-123',
        status: 'completed',
        metrics: { totalVulnerabilities: 5 },
      };

      const result = await validator.validateStateTransition(
        'scan',
        'scan-123',
        'completed',
        'running',
        scan,
        {}
      );

      expect(result.valid).toBe(false);
      expect(result.error).toBeDefined();
      expect(result.error!.error).toContain('Invalid state transition');
    });

    it('should reject scan transition without required fields', async () => {
      const scan = {
        id: 'scan-123',
        // Missing userId and code
      };

      const result = await validator.validateStateTransition(
        'scan',
        'scan-123',
        'pending',
        'queued',
        scan,
        {}
      );

      expect(result.valid).toBe(false);
      expect(result.error!.error).toContain('must have ID, user ID, and code');
    });

    it('should allow valid escrow state transition: pending -> released', async () => {
      const escrow = {
        id: 'escrow-123',
        amount: 100,
        depositor: 'user-123',
        beneficiary: 'user-456',
        lock_until: '2020-01-01T00:00:00Z', // Past date
      };

      const result = await validator.validateStateTransition(
        'escrow',
        'escrow-123',
        'pending',
        'released',
        escrow,
        { userId: 'user-123', conditions_met: true }
      );

      expect(result.valid).toBe(true);
    });

    it('should reject escrow release when not authorized', async () => {
      const escrow = {
        id: 'escrow-123',
        amount: 100,
        depositor: 'user-123',
        beneficiary: 'user-456',
      };

      const result = await validator.validateStateTransition(
        'escrow',
        'escrow-123',
        'pending',
        'released',
        escrow,
        { userId: 'unauthorized-user' }
      );

      expect(result.valid).toBe(false);
      expect(result.error!.error).toContain('authorized');
    });

    it('should reject escrow release when locked', async () => {
      const futureDate = new Date(Date.now() + 86400000).toISOString(); // Tomorrow
      const escrow = {
        id: 'escrow-123',
        amount: 100,
        depositor: 'user-123',
        beneficiary: 'user-456',
        lock_until: futureDate,
      };

      const result = await validator.validateStateTransition(
        'escrow',
        'escrow-123',
        'pending',
        'released',
        escrow,
        { userId: 'user-123' }
      );

      expect(result.valid).toBe(false);
      expect(result.error!.error).toContain('unlocked');
    });

    it('should allow valid vulnerability state transition: analyzing -> confirmed', async () => {
      const vulnerability = {
        id: 'vuln-123',
        scanId: 'scan-123',
        severity: 'high',
        description: 'Buffer overflow vulnerability',
      };

      const result = await validator.validateStateTransition(
        'vulnerability',
        'vuln-123',
        'analyzing',
        'confirmed',
        vulnerability,
        {}
      );

      expect(result.valid).toBe(true);
    });

    it('should reject vulnerability confirmation without required fields', async () => {
      const vulnerability = {
        id: 'vuln-123',
        scanId: 'scan-123',
        // Missing severity and description
      };

      const result = await validator.validateStateTransition(
        'vulnerability',
        'vuln-123',
        'analyzing',
        'confirmed',
        vulnerability,
        {}
      );

      expect(result.valid).toBe(false);
      expect(result.error!.error).toContain('severity and description');
    });
  });

  describe('validateInitialState', () => {
    it('should validate correct initial state for scan', () => {
      const isValid = validator.validateInitialState('scan', 'pending');
      expect(isValid).toBe(true);
    });

    it('should reject incorrect initial state for scan', () => {
      const isValid = validator.validateInitialState('scan', 'completed');
      expect(isValid).toBe(false);
    });

    it('should validate correct initial state for escrow', () => {
      const isValid = validator.validateInitialState('escrow', 'pending');
      expect(isValid).toBe(true);
    });

    it('should allow unknown entity types', () => {
      const isValid = validator.validateInitialState('unknown', 'any-state');
      expect(isValid).toBe(true);
    });
  });

  describe('getValidTransitions', () => {
    it('should return valid transitions from pending scan state', () => {
      const transitions = validator.getValidTransitions('scan', 'pending');
      expect(transitions).toContain('queued');
      expect(transitions).not.toContain('completed');
    });

    it('should return valid transitions from pending escrow state', () => {
      const transitions = validator.getValidTransitions('escrow', 'pending');
      expect(transitions).toContain('locked');
      expect(transitions).toContain('refunded');
      expect(transitions).not.toContain('released');
    });

    it('should return empty array for unknown entity', () => {
      const transitions = validator.getValidTransitions('unknown', 'any-state');
      expect(transitions).toEqual([]);
    });
  });

  describe('validateEntityConsistency', () => {
    it('should validate consistent scan entity', async () => {
      const scan = {
        status: 'completed',
        currentStep: 'completed',
        progress: 100,
        metrics: { totalVulnerabilities: 5 },
      };

      const result = await validator.validateEntityConsistency('scan', scan);
      expect(result.valid).toBe(true);
      expect(result.errors).toEqual([]);
    });

    it('should detect inconsistent scan entity', async () => {
      const scan = {
        status: 'completed',
        currentStep: 'running', // Inconsistent
        progress: 50, // Inconsistent
        metrics: { totalVulnerabilities: 5 },
      };

      const result = await validator.validateEntityConsistency('scan', scan);
      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
      expect(result.errors[0]).toContain('status is completed but currentStep is not completed');
    });

    it('should validate consistent escrow entity', async () => {
      const escrow = {
        status: 'released',
        conditions_met: true,
        amount: 100,
        lock_until: '2020-01-01T00:00:00Z',
      };

      const result = await validator.validateEntityConsistency('escrow', escrow);
      expect(result.valid).toBe(true);
    });

    it('should detect inconsistent escrow entity', async () => {
      const escrow = {
        status: 'released',
        conditions_met: false, // Inconsistent
        amount: 100,
      };

      const result = await validator.validateEntityConsistency('escrow', escrow);
      expect(result.valid).toBe(false);
      expect(result.errors[0]).toContain('Released escrow must have conditions_met=true');
    });

    it('should detect invalid escrow amount', async () => {
      const escrow = {
        status: 'pending',
        amount: -100, // Invalid
        conditions_met: false,
      };

      const result = await validator.validateEntityConsistency('escrow', escrow);
      expect(result.valid).toBe(false);
      expect(result.errors[0]).toContain('amount must be positive');
    });

    it('should allow unknown entity types', async () => {
      const entity = { any: 'data' };
      const result = await validator.validateEntityConsistency('unknown', entity);
      expect(result.valid).toBe(true);
      expect(result.errors).toEqual([]);
    });
  });

  describe('logStateViolation', () => {
    it('should log state violation', () => {
      const error: StateValidationError = {
        entity: 'scan',
        entityId: 'scan-123',
        currentState: 'completed',
        targetState: 'running',
        error: 'Invalid state transition',
      };

      // Mock the logger to avoid actual logging during tests
      const loggerSpy = jest.spyOn(validator['logger'], 'error').mockImplementation();

      validator.logStateViolation(error);

      expect(loggerSpy).toHaveBeenCalledWith(
        'State consistency violation: scan scan-123',
        expect.objectContaining({
          currentState: 'completed',
          targetState: 'running',
          error: 'Invalid state transition',
        })
      );

      loggerSpy.mockRestore();
    });
  });

  describe('getStateMachineConfig', () => {
    it('should return state machine config for known entity', () => {
      const config = validator.getStateMachineConfig('scan');
      expect(config).toBeDefined();
      expect(config!.entity).toBe('scan');
      expect(config!.states).toContain('pending');
      expect(config!.states).toContain('completed');
    });

    it('should return undefined for unknown entity', () => {
      const config = validator.getStateMachineConfig('unknown');
      expect(config).toBeUndefined();
    });
  });
});
