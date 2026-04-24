import {
  Injectable,
  NestInterceptor,
  ExecutionContext,
  CallHandler,
  BadRequestException,
  InternalServerErrorException,
} from '@nestjs/common';
import { Observable, throwError } from 'rxjs';
import { catchError, tap } from 'rxjs/operators';
import { Reflector } from '@nestjs/core';
import { STATE_TRANSITION_KEY, StateTransitionOptions } from '../decorators/state-transition.decorator';
import { StateConsistencyValidator, StateValidationError } from '../validation/state-consistency.validator';

@Injectable()
export class StateConsistencyInterceptor implements NestInterceptor {
  constructor(
    private readonly reflector: Reflector,
    private readonly stateValidator: StateConsistencyValidator,
  ) {}

  intercept(context: ExecutionContext, next: CallHandler): Observable<any> {
    const stateTransitionOptions = this.reflector.get<StateTransitionOptions>(
      STATE_TRANSITION_KEY,
      context.getHandler(),
    );

    if (!stateTransitionOptions) {
      return next.handle();
    }

    const request = context.switchToHttp().getRequest();
    const { entityType, stateField, contextField, skipIfNoChange } = stateTransitionOptions;

    // Store original state before processing
    const originalEntity = this.extractEntity(request, entityType);
    const originalState = originalEntity?.[stateField];

    return next.handle().pipe(
      tap(async (response) => {
        // Validate state transition after successful processing
        await this.validateStateTransition(
          request,
          response,
          entityType,
          stateField,
          contextField,
          originalState,
          skipIfNoChange,
        );
      }),
      catchError((error) => {
        // Handle validation errors
        if (error instanceof StateTransitionError) {
          return throwError(() => new BadRequestException({
            error: 'State transition validation failed',
            message: error.error,
            details: error,
          }));
        }
        return throwError(() => error);
      }),
    );
  }

  private async validateStateTransition(
    request: any,
    response: any,
    entityType: string,
    stateField: string,
    contextField: string | undefined,
    originalState: string,
    skipIfNoChange: boolean,
  ) {
    try {
      const updatedEntity = this.extractEntityFromResponse(response, entityType);
      const newState = updatedEntity?.[stateField];

      if (!newState) {
        return; // No state to validate
      }

      // Skip if state hasn't changed and option is enabled
      if (skipIfNoChange && originalState === newState) {
        return;
      }

      const entityId = updatedEntity?.id || updatedEntity?.scanId || 'unknown';
      const context = contextField ? request.body?.[contextField] || request.user : request.user;

      const validation = await this.stateValidator.validateStateTransition(
        entityType,
        entityId,
        originalState,
        newState,
        updatedEntity,
        context,
      );

      if (!validation.valid) {
        this.stateValidator.logStateViolation(validation.error!);
        throw new StateTransitionError(validation.error!);
      }

      // Additional consistency checks
      const consistencyCheck = await this.stateValidator.validateEntityConsistency(
        entityType,
        updatedEntity,
      );

      if (!consistencyCheck.valid) {
        const error: StateValidationError = {
          entity: entityType,
          entityId,
          currentState: newState,
          targetState: newState,
          error: `Entity consistency check failed: ${consistencyCheck.errors.join(', ')}`,
        };
        this.stateValidator.logStateViolation(error);
        throw new StateTransitionError(error);
      }

    } catch (error) {
      if (error instanceof StateTransitionError) {
        throw error;
      }
      
      // Log unexpected errors but don't fail the request
      console.error('State validation error:', error);
    }
  }

  private extractEntity(request: any, entityType: string): any {
    // Try to extract entity from request body, params, or query
    return request.body || request.params || {};
  }

  private extractEntityFromResponse(response: any, entityType: string): any {
    // Try to extract entity from response data
    if (response?.data) {
      return response.data;
    }
    return response;
  }
}

class StateTransitionError extends Error {
  constructor(public error: StateValidationError) {
    super(error.error);
    this.name = 'StateTransitionError';
  }
}
