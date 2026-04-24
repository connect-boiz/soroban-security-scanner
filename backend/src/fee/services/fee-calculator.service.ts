import { Injectable, Logger } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';

export interface FeeCalculationParams {
  type: 'scan' | 'api_call' | 'storage' | 'premium_feature';
  codeSize?: number;
  complexity?: number;
  processingTime?: number;
  resourceUsage?: any;
}

@Injectable()
export class FeeCalculatorService {
  private readonly logger = new Logger(FeeCalculatorService.name);

  constructor(private readonly configService: ConfigService) {}

  /**
   * Calculate fee based on operation type and parameters
   */
  calculateFee(params: FeeCalculationParams): number {
    const baseFee = this.getBaseFee(params.type);
    let multiplier = 1;
    
    switch (params.type) {
      case 'scan':
        multiplier = this.calculateScanMultiplier(params);
        break;
      case 'api_call':
        multiplier = this.calculateApiCallMultiplier(params);
        break;
      case 'storage':
        multiplier = this.calculateStorageMultiplier(params);
        break;
      case 'premium_feature':
        multiplier = this.calculatePremiumMultiplier(params);
        break;
    }

    const totalFee = Math.round(baseFee * multiplier);
    
    this.logger.debug(`Fee calculated: ${totalFee} for type=${params.type}, multiplier=${multiplier}`);
    
    return totalFee;
  }

  /**
   * Get base fee for operation type
   */
  private getBaseFee(type: string): number {
    const fees = {
      scan: this.configService.get<number>('FEES_BASE_SCAN_FEE', 100),
      api_call: this.configService.get<number>('FEES_BASE_API_CALL_FEE', 1),
      storage: this.configService.get<number>('FEES_BASE_STORAGE_FEE', 10),
      premium_feature: this.configService.get<number>('FEES_BASE_PREMIUM_FEE', 500),
    };

    return fees[type] || 0;
  }

  /**
   * Calculate multiplier for scan operations based on complexity
   */
  private calculateScanMultiplier(params: FeeCalculationParams): number {
    let multiplier = 1;
    
    // Code size multiplier
    if (params.codeSize) {
      if (params.codeSize > 100000) { // > 100KB
        multiplier *= 2;
      } else if (params.codeSize > 50000) { // > 50KB
        multiplier *= 1.5;
      }
    }

    // Complexity multiplier
    if (params.complexity) {
      if (params.complexity > 8) {
        multiplier *= 1.8;
      } else if (params.complexity > 5) {
        multiplier *= 1.3;
      }
    }

    // Processing time multiplier
    if (params.processingTime) {
      if (params.processingTime > 300) { // > 5 minutes
        multiplier *= 1.5;
      } else if (params.processingTime > 120) { // > 2 minutes
        multiplier *= 1.2;
      }
    }

    return multiplier;
  }

  /**
   * Calculate multiplier for API calls
   */
  private calculateApiCallMultiplier(params: FeeCalculationParams): number {
    let multiplier = 1;
    
    // Resource usage multiplier
    if (params.resourceUsage?.cpu) {
      if (params.resourceUsage.cpu > 80) {
        multiplier *= 1.5;
      }
    }

    if (params.resourceUsage?.memory) {
      if (params.resourceUsage.memory > 512) { // > 512MB
        multiplier *= 1.3;
      }
    }

    return multiplier;
  }

  /**
   * Calculate multiplier for storage operations
   */
  private calculateStorageMultiplier(params: FeeCalculationParams): number {
    let multiplier = 1;
    
    if (params.resourceUsage?.storageSize) {
      const sizeMB = params.resourceUsage.storageSize / (1024 * 1024);
      if (sizeMB > 100) {
        multiplier *= 2;
      } else if (sizeMB > 10) {
        multiplier *= 1.5;
      }
    }

    return multiplier;
  }

  /**
   * Calculate multiplier for premium features
   */
  private calculatePremiumMultiplier(params: FeeCalculationParams): number {
    // Premium features have fixed pricing with optional complexity adjustments
    return params.complexity ? Math.max(1, params.complexity / 5) : 1;
  }

  /**
   * Validate if user has sufficient balance for an operation
   */
  canAffordOperation(currentBalance: number, params: FeeCalculationParams): boolean {
    const requiredFee = this.calculateFee(params);
    return currentBalance >= requiredFee;
  }

  /**
   * Get estimated fee before operation
   */
  getEstimatedFee(params: FeeCalculationParams): {
    estimatedFee: number;
    breakdown: {
      baseFee: number;
      multiplier: number;
      totalFee: number;
    };
  } {
    const baseFee = this.getBaseFee(params.type);
    let multiplier = 1;
    
    switch (params.type) {
      case 'scan':
        multiplier = this.calculateScanMultiplier(params);
        break;
      case 'api_call':
        multiplier = this.calculateApiCallMultiplier(params);
        break;
      case 'storage':
        multiplier = this.calculateStorageMultiplier(params);
        break;
      case 'premium_feature':
        multiplier = this.calculatePremiumMultiplier(params);
        break;
    }

    const totalFee = Math.round(baseFee * multiplier);

    return {
      estimatedFee: totalFee,
      breakdown: {
        baseFee,
        multiplier,
        totalFee,
      },
    };
  }
}
