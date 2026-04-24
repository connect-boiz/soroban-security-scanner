import { Injectable, Logger } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';

export interface StateTransitionRule {
  from: string | string[];
  to: string;
  validator?: (entity: any, context?: any) => Promise<boolean> | boolean;
  errorMessage?: string;
}

export interface StateMachineConfig {
  entity: string;
  states: string[];
  transitions: StateTransitionRule[];
  initialState: string;
}

export interface StateValidationError {
  entity: string;
  entityId: string;
  currentState: string;
  targetState: string;
  error: string;
  context?: any;
}

@Injectable()
export class StateConsistencyValidator {
  private readonly logger = new Logger(StateConsistencyValidator.name);
  private readonly stateMachines: Map<string, StateMachineConfig> = new Map();

  constructor(private readonly configService: ConfigService) {
    this.initializeStateMachines();
  }

  private initializeStateMachines() {
    // Scan state machine
    this.stateMachines.set('scan', {
      entity: 'scan',
      states: ['pending', 'queued', 'running', 'completed', 'failed'],
      initialState: 'pending',
      transitions: [
        {
          from: 'pending',
          to: 'queued',
          validator: (scan) => {
            return scan.id && scan.userId && scan.code;
          },
          errorMessage: 'Scan must have ID, user ID, and code to be queued'
        },
        {
          from: 'queued',
          to: 'running',
          validator: (scan) => {
            return scan.status === 'queued';
          },
          errorMessage: 'Scan must be queued to start running'
        },
        {
          from: 'running',
          to: 'completed',
          validator: (scan) => {
            return scan.status === 'running' && scan.metrics;
          },
          errorMessage: 'Scan must be running and have metrics to be completed'
        },
        {
          from: 'running',
          to: 'failed',
          validator: (scan) => {
            return scan.status === 'running' || scan.status === 'queued';
          },
          errorMessage: 'Scan must be running or queued to fail'
        },
        {
          from: ['queued', 'running'],
          to: 'failed',
          validator: (scan) => {
            return true; // Can fail from queued or running
          },
          errorMessage: 'Invalid transition to failed state'
        }
      ]
    });

    // Escrow state machine
    this.stateMachines.set('escrow', {
      entity: 'escrow',
      states: ['pending', 'locked', 'released', 'refunded'],
      initialState: 'pending',
      transitions: [
        {
          from: 'pending',
          to: 'locked',
          validator: (escrow) => {
            return escrow.amount > 0 && escrow.depositor && escrow.beneficiary;
          },
          errorMessage: 'Escrow must have valid amount, depositor, and beneficiary to be locked'
        },
        {
          from: 'locked',
          to: 'released',
          validator: (escrow, context) => {
            const now = new Date();
            const lockUntil = escrow.lock_until ? new Date(escrow.lock_until) : null;
            return (!lockUntil || lockUntil <= now) && 
                   (context?.conditions_met ?? true) &&
                   escrow.depositor === context?.userId;
          },
          errorMessage: 'Escrow must be unlocked, conditions met, and authorized to be released'
        },
        {
          from: ['pending', 'locked'],
          to: 'refunded',
          validator: (escrow, context) => {
            return escrow.depositor === context?.userId;
          },
          errorMessage: 'Only depositor can refund escrow'
        }
      ]
    });

    // Vulnerability state machine
    this.stateMachines.set('vulnerability', {
      entity: 'vulnerability',
      states: ['detected', 'analyzing', 'confirmed', 'false_positive', 'mitigated', 'resolved'],
      initialState: 'detected',
      transitions: [
        {
          from: 'detected',
          to: 'analyzing',
          validator: (vulnerability) => {
            return vulnerability.id && vulnerability.scanId;
          },
          errorMessage: 'Vulnerability must have ID and scan ID to be analyzed'
        },
        {
          from: 'analyzing',
          to: 'confirmed',
          validator: (vulnerability) => {
            return vulnerability.severity && vulnerability.description;
          },
          errorMessage: 'Vulnerability must have severity and description to be confirmed'
        },
        {
          from: 'analyzing',
          to: 'false_positive',
          validator: (vulnerability, context) => {
            return context?.falsePositiveReason;
          },
          errorMessage: 'False positive requires a reason'
        },
        {
          from: 'confirmed',
          to: 'mitigated',
          validator: (vulnerability, context) => {
            return context?.mitigationStrategy;
          },
          errorMessage: 'Mitigation requires a strategy'
        },
        {
          from: ['confirmed', 'mitigated'],
          to: 'resolved',
          validator: (vulnerability, context) => {
            return context?.resolutionProof;
          },
          errorMessage: 'Resolution requires proof of fix'
        }
      ]
    });

    // API Key state machine
    this.stateMachines.set('apiKey', {
      entity: 'apiKey',
      states: ['active', 'suspended', 'revoked'],
      initialState: 'active',
      transitions: [
        {
          from: 'active',
          to: 'suspended',
          validator: (apiKey, context) => {
            return context?.reason && context?.adminId;
          },
          errorMessage: 'API key suspension requires reason and admin ID'
        },
        {
          from: ['active', 'suspended'],
          to: 'revoked',
          validator: (apiKey, context) => {
            return context?.reason && context?.adminId;
          },
          errorMessage: 'API key revocation requires reason and admin ID'
        }
      ]
    });
  }

  async validateStateTransition(
    entityType: string,
    entityId: string,
    currentState: string,
    targetState: string,
    entity: any,
    context?: any
  ): Promise<{ valid: boolean; error?: StateValidationError }> {
    const stateMachine = this.stateMachines.get(entityType);
    
    if (!stateMachine) {
      this.logger.warn(`No state machine configured for entity type: ${entityType}`);
      return { valid: true }; // Allow if no state machine exists
    }

    // Validate states exist
    if (!stateMachine.states.includes(currentState)) {
      const error: StateValidationError = {
        entity: entityType,
        entityId,
        currentState,
        targetState,
        error: `Invalid current state: ${currentState}`
      };
      return { valid: false, error };
    }

    if (!stateMachine.states.includes(targetState)) {
      const error: StateValidationError = {
        entity: entityType,
        entityId,
        currentState,
        targetState,
        error: `Invalid target state: ${targetState}`
      };
      return { valid: false, error };
    }

    // Find transition rule
    const transition = stateMachine.transitions.find(t => {
      const fromStates = Array.isArray(t.from) ? t.from : [t.from];
      return fromStates.includes(currentState) && t.to === targetState;
    });

    if (!transition) {
      const error: StateValidationError = {
        entity: entityType,
        entityId,
        currentState,
        targetState,
        error: `Invalid state transition from ${currentState} to ${targetState}`
      };
      return { valid: false, error };
    }

    // Run validator if present
    if (transition.validator) {
      try {
        const isValid = await transition.validator(entity, context);
        if (!isValid) {
          const error: StateValidationError = {
            entity: entityType,
            entityId,
            currentState,
            targetState,
            error: transition.errorMessage || 'State transition validation failed',
            context
          };
          return { valid: false, error };
        }
      } catch (error) {
        const validationError: StateValidationError = {
          entity: entityType,
          entityId,
          currentState,
          targetState,
          error: `Validator error: ${error.message}`,
          context
        };
        return { valid: false, error: validationError };
      }
    }

    return { valid: true };
  }

  validateInitialState(entityType: string, state: string): boolean {
    const stateMachine = this.stateMachines.get(entityType);
    if (!stateMachine) {
      return true;
    }

    return state === stateMachine.initialState;
  }

  getValidTransitions(entityType: string, currentState: string): string[] {
    const stateMachine = this.stateMachines.get(entityType);
    if (!stateMachine) {
      return [];
    }

    return stateMachine.transitions
      .filter(t => {
        const fromStates = Array.isArray(t.from) ? t.from : [t.from];
        return fromStates.includes(currentState);
      })
      .map(t => t.to);
  }

  getStateMachineConfig(entityType: string): StateMachineConfig | undefined {
    return this.stateMachines.get(entityType);
  }

  async validateEntityConsistency(entityType: string, entity: any): Promise<{ valid: boolean; errors: string[] }> {
    const stateMachine = this.stateMachines.get(entityType);
    if (!stateMachine) {
      return { valid: true, errors: [] };
    }

    const errors: string[] = [];

    // Check if state is valid
    if (!stateMachine.states.includes(entity.status)) {
      errors.push(`Invalid state: ${entity.status}`);
    }

    // Run entity-specific consistency checks
    switch (entityType) {
      case 'scan':
        errors.push(...this.validateScanConsistency(entity));
        break;
      case 'escrow':
        errors.push(...this.validateEscrowConsistency(entity));
        break;
      case 'vulnerability':
        errors.push(...this.validateVulnerabilityConsistency(entity));
        break;
    }

    return { valid: errors.length === 0, errors };
  }

  private validateScanConsistency(scan: any): string[] {
    const errors: string[] = [];

    // Status vs currentStep consistency
    if (scan.status === 'completed' && scan.currentStep !== 'completed') {
      errors.push('Scan status is completed but currentStep is not completed');
    }

    if (scan.status === 'failed' && scan.currentStep !== 'error') {
      errors.push('Scan status is failed but currentStep is not error');
    }

    if (scan.status === 'running' && !['parsing', 'fuzzing', 'analysis', 'reporting'].includes(scan.currentStep)) {
      errors.push('Scan status is running but currentStep is not a running step');
    }

    // Progress consistency
    if (scan.status === 'completed' && scan.progress !== 100) {
      errors.push('Completed scan must have 100% progress');
    }

    if (scan.status === 'failed' && scan.progress > 0 && scan.progress < 100) {
      // This is OK - failed scans can have partial progress
    }

    // Metrics consistency
    if (scan.status === 'completed' && !scan.metrics) {
      errors.push('Completed scan must have metrics');
    }

    if (scan.metrics && scan.status === 'pending') {
      errors.push('Pending scan should not have metrics');
    }

    return errors;
  }

  private validateEscrowConsistency(escrow: any): string[] {
    const errors: string[] = [];

    // Amount consistency
    if (escrow.amount <= 0) {
      errors.push('Escrow amount must be positive');
    }

    // Status vs conditions_met consistency
    if (escrow.status === 'released' && !escrow.conditions_met) {
      errors.push('Released escrow must have conditions_met=true');
    }

    if (escrow.status === 'pending' && escrow.conditions_met) {
      errors.push('Pending escrow should not have conditions_met=true');
    }

    // Lock consistency
    if (escrow.lock_until && escrow.status === 'released') {
      const lockTime = new Date(escrow.lock_until);
      const now = new Date();
      if (lockTime > now) {
        errors.push('Released escrow cannot have future lock_until time');
      }
    }

    return errors;
  }

  private validateVulnerabilityConsistency(vulnerability: any): string[] {
    const errors: string[] = [];

    // Severity consistency
    if (!vulnerability.severity) {
      errors.push('Vulnerability must have severity');
    }

    if (vulnerability.severity && !['critical', 'high', 'medium', 'low', 'info'].includes(vulnerability.severity)) {
      errors.push('Invalid severity level');
    }

    // Scan consistency
    if (!vulnerability.scanId) {
      errors.push('Vulnerability must be associated with a scan');
    }

    return errors;
  }

  logStateViolation(error: StateValidationError) {
    this.logger.error(`State consistency violation: ${error.entity} ${error.entityId}`, {
      currentState: error.currentState,
      targetState: error.targetState,
      error: error.error,
      context: error.context
    });

    // Add timestamp to context if not present
    if (!error.context) {
      error.context = {};
    }
    error.context.timestamp = new Date().toISOString();

    // In production, you might want to send this to a monitoring system
    if (this.configService.get('NODE_ENV') === 'production') {
      // Send to monitoring service
      this.sendToMonitoring(error);
    }
  }

  private sendToMonitoring(error: StateValidationError) {
    // Implementation for sending to monitoring system
    // This could be Sentry, DataDog, etc.
    this.logger.warn('State violation sent to monitoring', error);
  }
}
