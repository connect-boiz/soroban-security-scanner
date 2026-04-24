import { SetMetadata } from '@nestjs/common';

export interface StateTransitionOptions {
  entityType: string;
  stateField?: string;
  contextField?: string;
  skipIfNoChange?: boolean;
}

export const STATE_TRANSITION_KEY = 'state_transition';

export const StateTransition = (options: StateTransitionOptions) => 
  SetMetadata(STATE_TRANSITION_KEY, options);

export const ValidateStateTransition = (entityType: string, stateField: string = 'status') =>
  StateTransition({
    entityType,
    stateField,
    skipIfNoChange: true,
  });

// Predefined decorators for common entities
export const ValidateScanStateTransition = () => 
  ValidateStateTransition('scan', 'status');

export const ValidateEscrowStateTransition = () => 
  ValidateStateTransition('escrow', 'status');

export const ValidateVulnerabilityStateTransition = () => 
  ValidateStateTransition('vulnerability', 'status');

export const ValidateApiKeyStateTransition = () => 
  ValidateStateTransition('apiKey', 'status');
