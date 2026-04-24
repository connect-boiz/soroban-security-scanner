import { SetMetadata } from '@nestjs/common';

export const MULTI_SIG_KEY = 'multi_signature';

export interface MultiSignatureConfig {
  requiredSignatures: number;
  timeoutMinutes?: number;
  allowedRoles?: string[];
  operationType: string;
}

export const RequireMultiSignature = (config: MultiSignatureConfig) => 
  SetMetadata(MULTI_SIG_KEY, config);
