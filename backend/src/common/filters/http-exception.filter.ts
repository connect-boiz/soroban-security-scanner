import {
  ExceptionFilter,
  Catch,
  ArgumentsHost,
  HttpException,
  HttpStatus,
  Logger,
} from '@nestjs/common';
import { Request, Response } from 'express';

@Catch()
export class HttpExceptionFilter implements ExceptionFilter {
  private readonly logger = new Logger(HttpExceptionFilter.name);

  catch(exception: unknown, host: ArgumentsHost): void {
    const ctx = host.switchToHttp();
    const response = ctx.getResponse<Response>();
    const request = ctx.getRequest<Request>();

    let status: number;
    let message: string;
    let code: string;

    if (exception instanceof HttpException) {
      status = exception.getStatus();
      const exceptionResponse = exception.getResponse();
      
      if (typeof exceptionResponse === 'string') {
        message = exceptionResponse;
      } else if (typeof exceptionResponse === 'object' && exceptionResponse !== null) {
        message = (exceptionResponse as any).message || exception.message;
        code = (exceptionResponse as any).code || this.getErrorCode(status);
      } else {
        message = exception.message;
      }
      
      code = code || this.getErrorCode(status);
    } else {
      status = HttpStatus.INTERNAL_SERVER_ERROR;
      message = 'Internal server error';
      code = 'INTERNAL_SERVER_ERROR';
    }

    // Log the error
    this.logger.error(
      `${request.method} ${request.url} - Status: ${status} - Error: ${message}`,
      exception instanceof Error ? exception.stack : exception,
    );

    // Send error response
    response.status(status).json({
      success: false,
      error: {
        message,
        code,
        timestamp: new Date().toISOString(),
        path: request.url,
        method: request.method,
        ...(process.env.NODE_ENV === 'development' && {
          stack: exception instanceof Error ? exception.stack : undefined,
        }),
      },
    });
  }

  private getErrorCode(status: number): string {
    const errorCodes: Record<number, string> = {
      400: 'BAD_REQUEST',
      401: 'UNAUTHORIZED',
      403: 'FORBIDDEN',
      404: 'NOT_FOUND',
      409: 'CONFLICT',
      422: 'UNPROCESSABLE_ENTITY',
      429: 'TOO_MANY_REQUESTS',
      500: 'INTERNAL_SERVER_ERROR',
      502: 'BAD_GATEWAY',
      503: 'SERVICE_UNAVAILABLE',
    };

    return errorCodes[status] || 'UNKNOWN_ERROR';
  }
}
