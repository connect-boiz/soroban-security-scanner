import { Injectable, Logger, BadRequestException } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { StateConsistencyValidator } from '../common/validation/state-consistency.validator';

export interface EscrowEntry {
  id: string;
  depositor: string;
  beneficiary: string;
  amount: number;
  token: string;
  purpose: 'bounty' | 'reward' | 'emergency';
  status: 'pending' | 'locked' | 'released' | 'refunded';
  created_at: string;
  lock_until?: string;
  conditions_met: boolean;
  release_signature?: string;
  contract_address?: string;
}

export interface CreateEscrowDto {
  beneficiary: string;
  amount: number;
  token: string;
  purpose: 'bounty' | 'reward' | 'emergency';
  lock_until?: string;
  conditions?: any;
  contract_address?: string;
}

export interface ReleaseEscrowDto {
  release_signature?: string;
  conditions_met?: boolean;
  release_reason?: string;
}

@Injectable()
export class EscrowService {
  private readonly logger = new Logger(EscrowService.name);
  private readonly escrows: Map<string, EscrowEntry> = new Map();

  constructor(
    private readonly configService: ConfigService,
    private readonly stateValidator: StateConsistencyValidator,
  ) {}

  async createEscrow(createEscrowDto: CreateEscrowDto, userId: string): Promise<EscrowEntry> {
    const escrowId = this.generateEscrowId();
    
    const escrow: EscrowEntry = {
      id: escrowId,
      depositor: userId,
      beneficiary: createEscrowDto.beneficiary,
      amount: createEscrowDto.amount,
      token: createEscrowDto.token,
      purpose: createEscrowDto.purpose,
      status: 'pending',
      created_at: new Date().toISOString(),
      lock_until: createEscrowDto.lock_until,
      conditions_met: false,
      contract_address: createEscrowDto.contract_address,
    };

    // Validate initial state
    const isStateValid = this.stateValidator.validateInitialState('escrow', escrow.status);
    if (!isStateValid) {
      throw new BadRequestException('Invalid initial escrow state');
    }

    // Validate entity consistency
    const consistencyCheck = await this.stateValidator.validateEntityConsistency('escrow', escrow);
    if (!consistencyCheck.valid) {
      throw new BadRequestException(`Escrow consistency validation failed: ${consistencyCheck.errors.join(', ')}`);
    }

    // Store escrow (in production, this would be in a database)
    this.escrows.set(escrowId, escrow);

    this.logger.log(`Created escrow ${escrowId} for ${createEscrowDto.amount} ${createEscrowDto.token}`);
    
    return escrow;
  }

  async releaseEscrow(
    escrowId: string,
    releaseEscrowDto: ReleaseEscrowDto,
    userId: string,
  ): Promise<{ success: boolean; data?: any; error?: string }> {
    const escrow = this.escrows.get(escrowId);
    
    if (!escrow) {
      return {
        success: false,
        error: 'Escrow not found',
      };
    }

    const originalStatus = escrow.status;

    // CHECKS: Perform all validations before any state changes
    // Validate state transition
    const validation = await this.stateValidator.validateStateTransition(
      'escrow',
      escrowId,
      originalStatus,
      'released',
      escrow,
      { 
        userId,
        conditions_met: releaseEscrowDto.conditions_met ?? true,
        release_signature: releaseEscrowDto.release_signature,
      }
    );

    if (!validation.valid) {
      this.stateValidator.logStateViolation(validation.error!);
      return {
        success: false,
        error: `Cannot release escrow: ${validation.error!.error}`,
      };
    }

    // Create a copy of the escrow with proposed changes for consistency validation
    const proposedEscrow = {
      ...escrow,
      status: 'released' as const,
      conditions_met: releaseEscrowDto.conditions_met ?? true,
      release_signature: releaseEscrowDto.release_signature,
    };

    // Validate consistency with proposed changes (before actual state change)
    const consistencyCheck = await this.stateValidator.validateEntityConsistency('escrow', proposedEscrow);
    if (!consistencyCheck.valid) {
      return {
        success: false,
        error: `Escrow consistency validation failed: ${consistencyCheck.errors.join(', ')}`,
      };
    }

    // EFFECTS: Apply state changes only after all validations pass
    escrow.status = 'released';
    escrow.conditions_met = releaseEscrowDto.conditions_met ?? true;
    escrow.release_signature = releaseEscrowDto.release_signature;

    // Store updated escrow
    this.escrows.set(escrowId, escrow);

    this.logger.log(`Released escrow ${escrowId} to ${escrow.beneficiary}`);

    return {
      success: true,
      data: escrow,
    };
  }

  async getEscrow(escrowId: string, userId: string): Promise<EscrowEntry> {
    const escrow = this.escrows.get(escrowId);
    
    if (!escrow) {
      throw new Error('Escrow not found');
    }

    // Check authorization (user can view their own escrows or ones where they are beneficiary)
    if (escrow.depositor !== userId && escrow.beneficiary !== userId) {
      throw new Error('Unauthorized to view this escrow');
    }

    return escrow;
  }

  async getEscrowHistory(
    userId: string,
    page: number = 1,
    limit: number = 10,
  ): Promise<{
    escrows: EscrowEntry[];
    total: number;
    page: number;
    limit: number;
    totalPages: number;
  }> {
    const userEscrows = Array.from(this.escrows.values()).filter(
      escrow => escrow.depositor === userId || escrow.beneficiary === userId,
    );

    // Sort by created_at (newest first)
    userEscrows.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());

    const total = userEscrows.length;
    const totalPages = Math.ceil(total / limit);
    const startIndex = (page - 1) * limit;
    const endIndex = startIndex + limit;
    const paginatedEscrows = userEscrows.slice(startIndex, endIndex);

    return {
      escrows: paginatedEscrows,
      total,
      page,
      limit,
      totalPages,
    };
  }

  async getEscrowStats(): Promise<{
    total_escrows: number;
    pending_escrows: number;
    released_escrows: number;
    total_amount_locked: number;
    total_amount_released: number;
    by_purpose: {
      bounty: number;
      reward: number;
      emergency: number;
    };
  }> {
    const allEscrows = Array.from(this.escrows.values());
    
    const stats = {
      total_escrows: allEscrows.length,
      pending_escrows: allEscrows.filter(e => e.status === 'pending').length,
      released_escrows: allEscrows.filter(e => e.status === 'released').length,
      total_amount_locked: allEscrows
        .filter(e => e.status === 'pending' || e.status === 'locked')
        .reduce((sum, e) => sum + e.amount, 0),
      total_amount_released: allEscrows
        .filter(e => e.status === 'released')
        .reduce((sum, e) => sum + e.amount, 0),
      by_purpose: {
        bounty: allEscrows.filter(e => e.purpose === 'bounty').length,
        reward: allEscrows.filter(e => e.purpose === 'reward').length,
        emergency: allEscrows.filter(e => e.purpose === 'emergency').length,
      },
    };

    return stats;
  }

  private generateEscrowId(): string {
    const timestamp = Date.now().toString(36);
    const random = Math.random().toString(36).substring(2, 8);
    return `escrow_${timestamp}_${random}`;
  }

  // Validation helpers
  private validateEscrowAmount(amount: number): boolean {
    const minAmount = this.configService.get<number>('ESCROW_MIN_AMOUNT', 1);
    const maxAmount = this.configService.get<number>('ESCROW_MAX_AMOUNT', 1000000);
    
    return amount >= minAmount && amount <= maxAmount;
  }

  private validateTokenAddress(token: string): boolean {
    // Basic validation for token address (in production, this would validate against actual token contracts)
    return token && token.length >= 42 && token.startsWith('0x');
  }

  private validateUserAddress(address: string): boolean {
    // Basic validation for user address (in production, this would validate against actual addresses)
    return address && address.length >= 42 && address.startsWith('0x');
  }
}
