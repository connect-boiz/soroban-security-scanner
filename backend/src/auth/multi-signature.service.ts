import { Injectable, Logger } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { MultiSignatureConfig } from './multi-signature.decorator';

export interface MultiSignatureRequest {
  id: string;
  operationType: string;
  requestedBy: string;
  requiredSignatures: number;
  currentSignatures: Signature[];
  status: 'pending' | 'approved' | 'rejected' | 'expired';
  createdAt: Date;
  expiresAt?: Date;
  metadata?: any;
}

export interface Signature {
  userId: string;
  signature: string;
  timestamp: Date;
  role: string;
}

@Injectable()
export class MultiSignatureService {
  private readonly logger = new Logger(MultiSignatureService.name);
  private readonly pendingRequests: Map<string, MultiSignatureRequest> = new Map();

  constructor(private configService: ConfigService) {}

  async createMultiSignatureRequest(
    operationType: string,
    requestedBy: string,
    config: MultiSignatureConfig,
    metadata?: any,
  ): Promise<MultiSignatureRequest> {
    const requestId = this.generateRequestId();
    const expiresAt = config.timeoutMinutes 
      ? new Date(Date.now() + config.timeoutMinutes * 60 * 1000)
      : undefined;

    const request: MultiSignatureRequest = {
      id: requestId,
      operationType,
      requestedBy,
      requiredSignatures: config.requiredSignatures,
      currentSignatures: [],
      status: 'pending',
      createdAt: new Date(),
      expiresAt,
      metadata,
    };

    this.pendingRequests.set(requestId, request);
    this.logger.log(`Created multi-signature request ${requestId} for operation ${operationType}`);

    return request;
  }

  async addSignature(
    requestId: string,
    userId: string,
    userRole: string,
    signature: string,
  ): Promise<{ success: boolean; message: string; request?: MultiSignatureRequest }> {
    const request = this.pendingRequests.get(requestId);
    
    if (!request) {
      return { success: false, message: 'Multi-signature request not found' };
    }

    if (request.status !== 'pending') {
      return { success: false, message: `Request is ${request.status}` };
    }

    if (request.expiresAt && request.expiresAt < new Date()) {
      request.status = 'expired';
      return { success: false, message: 'Request has expired' };
    }

    // Check if user already signed
    const existingSignature = request.currentSignatures.find(sig => sig.userId === userId);
    if (existingSignature) {
      return { success: false, message: 'User has already signed this request' };
    }

    // Add signature
    const newSignature: Signature = {
      userId,
      signature,
      timestamp: new Date(),
      role: userRole,
    };

    request.currentSignatures.push(newSignature);

    // Check if required signatures reached
    if (request.currentSignatures.length >= request.requiredSignatures) {
      request.status = 'approved';
      this.logger.log(`Multi-signature request ${requestId} approved with ${request.currentSignatures.length} signatures`);
    }

    return { success: true, message: 'Signature added', request };
  }

  async validateMultiSignature(
    request: any,
    config: MultiSignatureConfig,
    user: any,
  ): Promise<boolean> {
    // For operations that require multi-signature, check if there's a valid request
    const multiSigRequestId = request.headers['x-multisig-request-id'];
    const signature = request.headers['x-signature'];

    if (!multiSigRequestId || !signature) {
      return false;
    }

    const multiSigRequest = this.pendingRequests.get(multiSigRequestId);
    
    if (!multiSigRequest || multiSigRequest.status !== 'approved') {
      return false;
    }

    // Verify the user's signature is part of the approved signatures
    const userSignature = multiSigRequest.currentSignatures.find(sig => sig.userId === user.userId);
    if (!userSignature || userSignature.signature !== signature) {
      return false;
    }

    // Verify the operation type matches
    if (multiSigRequest.operationType !== config.operationType) {
      return false;
    }

    return true;
  }

  async getPendingRequests(): Promise<MultiSignatureRequest[]> {
    return Array.from(this.pendingRequests.values()).filter(req => req.status === 'pending');
  }

  async getRequest(requestId: string): Promise<MultiSignatureRequest | null> {
    return this.pendingRequests.get(requestId) || null;
  }

  async cancelRequest(requestId: string, userId: string): Promise<{ success: boolean; message: string }> {
    const request = this.pendingRequests.get(requestId);
    
    if (!request) {
      return { success: false, message: 'Request not found' };
    }

    if (request.requestedBy !== userId) {
      return { success: false, message: 'Only request creator can cancel' };
    }

    if (request.status !== 'pending') {
      return { success: false, message: `Request is ${request.status}` };
    }

    request.status = 'rejected';
    this.logger.log(`Multi-signature request ${requestId} cancelled by ${userId}`);

    return { success: true, message: 'Request cancelled' };
  }

  private generateRequestId(): string {
    return `msig_${Date.now()}_${Math.random().toString(36).substring(2, 8)}`;
  }

  // Cleanup expired requests
  async cleanupExpiredRequests(): Promise<void> {
    const now = new Date();
    const expiredRequests: string[] = [];

    for (const [id, request] of this.pendingRequests.entries()) {
      if (request.expiresAt && request.expiresAt < now && request.status === 'pending') {
        request.status = 'expired';
        expiredRequests.push(id);
      }
    }

    if (expiredRequests.length > 0) {
      this.logger.log(`Cleaned up ${expiredRequests.length} expired multi-signature requests`);
    }
  }
}
