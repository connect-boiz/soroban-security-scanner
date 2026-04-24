import { Injectable, CanActivate, ExecutionContext, BadRequestException } from '@nestjs/common';
import { Reflector } from '@nestjs/core';
import { FeeService } from '../services/fee.service';
import { FeeCalculatorService, FeeCalculationParams } from '../services/fee-calculator.service';

export const FEE_TYPE_KEY = 'feeType';
export const FEE_PARAMS_KEY = 'feeParams';

@Injectable()
export class FeeGuard implements CanActivate {
  constructor(
    private readonly feeService: FeeService,
    private readonly feeCalculator: FeeCalculatorService,
    private readonly reflector: Reflector,
  ) {}

  async canActivate(context: ExecutionContext): Promise<boolean> {
    const request = context.switchToHttp().getRequest();
    const feeType = this.reflector.get<string>(FEE_TYPE_KEY, context.getHandler());
    const feeParamsExtractor = this.reflector.get<(req: any) => FeeCalculationParams>(FEE_PARAMS_KEY, context.getHandler());

    if (!feeType) {
      return true; // No fee required for this endpoint
    }

    const userId = request.user?.id || 'anonymous';
    
    // Extract fee parameters
    let feeParams: FeeCalculationParams;
    if (feeParamsExtractor) {
      feeParams = { type: feeType, ...feeParamsExtractor(request) };
    } else {
      feeParams = { type: feeType };
    }

    // Check if user can afford the operation
    const canAfford = await this.feeService.canAffordOperation(userId, feeParams);
    
    if (!canAfford) {
      const balance = await this.feeService.getUserBalance(userId);
      const estimatedFee = this.feeCalculator.getEstimatedFee(feeParams);
      
      throw new BadRequestException(
        `Insufficient balance. Required: ${estimatedFee.estimatedFee}, Available: ${balance?.balance || 0}`
      );
    }

    // Store fee info in request for later use
    request.feeInfo = {
      type: feeType,
      params: feeParams,
      estimatedFee: this.feeCalculator.getEstimatedFee(feeParams),
    };

    return true;
  }
}

export const SetFeeType = (type: string) => (target: any, propertyKey: string, descriptor: PropertyDescriptor) => {
  Reflect.defineMetadata(FEE_TYPE_KEY, type, descriptor.value);
};

export const SetFeeParams = (extractor: (req: any) => any) => (target: any, propertyKey: string, descriptor: PropertyDescriptor) => {
  Reflect.defineMetadata(FEE_PARAMS_KEY, extractor, descriptor.value);
};
