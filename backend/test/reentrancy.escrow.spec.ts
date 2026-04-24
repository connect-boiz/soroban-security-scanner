import { Test, TestingModule } from '@nestjs/testing';
import { EscrowService, EscrowEntry, CreateEscrowDto, ReleaseEscrowDto } from '../src/escrow/escrow.service';
import { StateConsistencyValidator } from '../src/common/validation/state-consistency.validator';
import { ConfigService } from '@nestjs/config';

describe('EscrowService - Reentrancy Protection', () => {
  let service: EscrowService;
  let stateValidator: StateConsistencyValidator;
  let configService: ConfigService;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [
        EscrowService,
        StateConsistencyValidator,
        {
          provide: ConfigService,
          useValue: {
            get: jest.fn((key: string, defaultValue?: any) => defaultValue),
          },
        },
      ],
    }).compile();

    service = module.get<EscrowService>(EscrowService);
    stateValidator = module.get<StateConsistencyValidator>(StateConsistencyValidator);
    configService = module.get<ConfigService>(ConfigService);
  });

  describe('releaseEscrow - Reentrancy Protection', () => {
    let escrow: EscrowEntry;
    const userId = 'test-user';
    const beneficiary = 'test-beneficiary';

    beforeEach(async () => {
      const createDto: CreateEscrowDto = {
        beneficiary,
        amount: 100,
        token: '0x1234567890123456789012345678901234567890',
        purpose: 'bounty',
      };

      escrow = await service.createEscrow(createDto, userId);
      
      // Manually set status to 'locked' to allow release
      escrow.status = 'locked';
      (service as any).escrows.set(escrow.id, escrow);
    });

    it('should prevent reentrancy by validating before state change', async () => {
      const releaseDto: ReleaseEscrowDto = {
        conditions_met: true,
        release_signature: 'test-signature',
      };

      // Mock the consistency validator to simulate an external call that could trigger reentrancy
      const originalValidateEntityConsistency = stateValidator.validateEntityConsistency;
      let callCount = 0;
      
      jest.spyOn(stateValidator, 'validateEntityConsistency').mockImplementation(async (entityType, entity) => {
        callCount++;
        
        // Simulate a reentrancy attempt on the first call
        if (callCount === 1) {
          // Try to call releaseEscrow again during validation
          const reentrancyResult = await service.releaseEscrow(escrow.id, releaseDto, userId);
          
          // This should fail because the original escrow status hasn't changed yet
          expect(reentrancyResult.success).toBe(false);
          expect(reentrancyResult.error).toContain('Cannot release escrow');
        }
        
        // Return valid on second call (after reentrancy attempt)
        return { valid: true, errors: [] };
      });

      const result = await service.releaseEscrow(escrow.id, releaseDto, userId);

      // The original call should succeed
      expect(result.success).toBe(true);
      expect(result.data.status).toBe('released');
      
      // Verify the validator was called twice (once for reentrancy attempt, once for actual validation)
      expect(callCount).toBe(2);
      
      // Verify the escrow was only released once
      const finalEscrow = await service.getEscrow(escrow.id, userId);
      expect(finalEscrow.status).toBe('released');
    });

    it('should not change state if consistency validation fails', async () => {
      const releaseDto: ReleaseEscrowDto = {
        conditions_met: true,
        release_signature: 'test-signature',
      };

      // Mock consistency validation to fail
      jest.spyOn(stateValidator, 'validateEntityConsistency').mockResolvedValue({
        valid: false,
        errors: ['Consistency check failed'],
      });

      const result = await service.releaseEscrow(escrow.id, releaseDto, userId);

      expect(result.success).toBe(false);
      expect(result.error).toContain('Consistency validation failed');
      
      // Verify the escrow status was not changed
      const unchangedEscrow = await service.getEscrow(escrow.id, userId);
      expect(unchangedEscrow.status).toBe('locked');
    });

    it('should follow checks-effects-interactions pattern', async () => {
      const releaseDto: ReleaseEscrowDto = {
        conditions_met: true,
        release_signature: 'test-signature',
      };

      let validationCallOrder: string[] = [];
      
      // Track the order of calls to verify the pattern
      jest.spyOn(stateValidator, 'validateStateTransition').mockImplementation(async (...args) => {
        validationCallOrder.push('state-transition');
        return { valid: true };
      });
      
      jest.spyOn(stateValidator, 'validateEntityConsistency').mockImplementation(async (...args) => {
        validationCallOrder.push('entity-consistency');
        return { valid: true, errors: [] };
      });

      const result = await service.releaseEscrow(escrow.id, releaseDto, userId);

      expect(result.success).toBe(true);
      
      // Verify the order: checks first, then effects
      expect(validationCallOrder).toEqual(['state-transition', 'entity-consistency']);
      
      // Verify final state
      const finalEscrow = await service.getEscrow(escrow.id, userId);
      expect(finalEscrow.status).toBe('released');
    });

    it('should handle concurrent release attempts safely', async () => {
      const releaseDto: ReleaseEscrowDto = {
        conditions_met: true,
        release_signature: 'test-signature',
      };

      // Create two concurrent release attempts
      const promise1 = service.releaseEscrow(escrow.id, releaseDto, userId);
      const promise2 = service.releaseEscrow(escrow.id, releaseDto, userId);

      const [result1, result2] = await Promise.all([promise1, promise2]);

      // Only one should succeed due to state transition validation
      const successCount = [result1, result2].filter(r => r.success).length;
      expect(successCount).toBe(1);
      
      // Verify final state is consistent
      const finalEscrow = await service.getEscrow(escrow.id, userId);
      expect(finalEscrow.status).toBe('released');
    });
  });
});
