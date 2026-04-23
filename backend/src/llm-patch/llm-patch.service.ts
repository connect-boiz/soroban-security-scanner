import { Injectable, Logger, OnModuleInit } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { HttpService } from '@nestjs/axios';
import { firstValueFrom } from 'rxjs';
import { AxiosRequestConfig } from 'axios';

export interface VulnerabilityReport {
  id: string;
  file_path: string;
  vulnerability_type: string;
  severity: string;
  title: string;
  description: string;
  code_snippet: string;
  line_number: number;
  sarif_report?: any;
}

export interface PatchRequest {
  vulnerability: VulnerabilityReport;
  original_code: string;
  context?: string;
}

export interface PatchResponse {
  id: string;
  vulnerability_id: string;
  patch: {
    original_code: string;
    patched_code: string;
    explanation: string;
    security_improvements: string[];
  };
  confidence_score: number;
  verification_status: 'Passed' | 'Failed' | 'Skipped';
  git_diff: string;
  fallback_provided: boolean;
  created_at: string;
}

export interface ApplyPatchRequest {
  target_dir: string;
}

export interface ApplyPatchResponse {
  success: boolean;
  message?: string;
  error?: string;
  remediation_id: string;
}

@Injectable()
export class LlmPatchService implements OnModuleInit {
  private readonly logger = new Logger(LlmPatchService.name);
  private readonly baseUrl: string;
  private readonly apiKey?: string;

  constructor(
    private readonly httpService: HttpService,
    private readonly configService: ConfigService,
  ) {
    this.baseUrl = this.configService.get<string>('LLM_PATCH_SERVICE_URL', 'http://localhost:8080');
    this.apiKey = this.configService.get<string>('LLM_PATCH_SERVICE_API_KEY');
  }

  async onModuleInit() {
    try {
      await this.healthCheck();
      this.logger.log('LLM Patch Service is healthy and ready');
    } catch (error) {
      this.logger.error('LLM Patch Service is not available:', error);
    }
  }

  private async makeRequest<T>(
    endpoint: string,
    data?: any,
    method: 'GET' | 'POST' = 'GET',
  ): Promise<T> {
    const config: AxiosRequestConfig = {
      method,
      url: `${this.baseUrl}${endpoint}`,
      headers: {
        'Content-Type': 'application/json',
        ...(this.apiKey && { Authorization: `Bearer ${this.apiKey}` }),
      },
      timeout: 120000, // 2 minutes timeout
    };

    if (data && method === 'POST') {
      config.data = data;
    }

    try {
      const response = await firstValueFrom(this.httpService.request<T>(config));
      return response.data;
    } catch (error) {
      this.logger.error(`Request to ${endpoint} failed:`, error.response?.data || error.message);
      throw new Error(`LLM Patch Service request failed: ${error.message}`);
    }
  }

  async healthCheck(): Promise<boolean> {
    try {
      const response = await this.makeRequest<{ status: string }>('/health');
      return response.status === 'healthy';
    } catch (error) {
      throw new Error(`Health check failed: ${error.message}`);
    }
  }

  async generatePatch(request: PatchRequest): Promise<PatchResponse> {
    this.logger.log(`Generating patch for vulnerability: ${request.vulnerability.id}`);
    
    try {
      const response = await this.makeRequest<PatchResponse>('/patch', request, 'POST');
      
      this.logger.log(
        `Patch generated successfully with confidence score: ${response.confidence_score}`,
      );
      
      return response;
    } catch (error) {
      this.logger.error(`Failed to generate patch for vulnerability ${request.vulnerability.id}:`, error);
      throw error;
    }
  }

  async applyPatch(
    remediationId: string,
    targetDir: string,
  ): Promise<ApplyPatchResponse> {
    this.logger.log(`Applying patch ${remediationId} to ${targetDir}`);
    
    try {
      const request: ApplyPatchRequest = { target_dir: targetDir };
      const response = await this.makeRequest<ApplyPatchResponse>(
        `/patch/${remediationId}/apply`,
        request,
        'POST',
      );
      
      if (response.success) {
        this.logger.log(`Patch ${remediationId} applied successfully`);
      } else {
        this.logger.warn(`Patch ${remediationId} application failed: ${response.error}`);
      }
      
      return response;
    } catch (error) {
      this.logger.error(`Failed to apply patch ${remediationId}:`, error);
      throw error;
    }
  }

  async getRemediationHistory(vulnerabilityId: string): Promise<{
    vulnerability_id: string;
    history: PatchResponse[];
    count: number;
  }> {
    try {
      const response = await this.makeRequest(
        `/history/${vulnerabilityId}`,
      );
      return response;
    } catch (error) {
      this.logger.error(`Failed to get remediation history for ${vulnerabilityId}:`, error);
      throw error;
    }
  }

  async getServiceStats(): Promise<{
    total_remediations: number;
    applied_remediations: number;
    avg_confidence: number;
    avg_success_rate: number;
    passed_verifications: number;
  }> {
    try {
      const response = await this.makeRequest('/stats');
      return response;
    } catch (error) {
      this.logger.error('Failed to get service stats:', error);
      throw error;
    }
  }

  /**
   * Converts a core scanner vulnerability to the LLM patch service format
   */
  convertVulnerabilityFromCoreScanner(
    coreVulnerability: any,
    originalCode: string,
  ): PatchRequest {
    return {
      vulnerability: {
        id: coreVulnerability.id,
        file_path: coreVulnerability.location?.file || '',
        vulnerability_type: coreVulnerability.vulnerability_type || 'Unknown',
        severity: coreVulnerability.severity || 'Medium',
        title: coreVulnerability.title || 'Security Vulnerability',
        description: coreVulnerability.description || 'No description available',
        code_snippet: coreVulnerability.code_snippet || '',
        line_number: coreVulnerability.location?.line || 0,
        // Convert SARIF report if available
        sarif_report: coreVulnerability.sarif_report || undefined,
      },
      original_code,
      context: coreVulnerability.context || undefined,
    };
  }

  /**
   * Batch process multiple vulnerabilities
   */
  async batchGeneratePatches(
    requests: PatchRequest[],
  ): Promise<{ results: PatchResponse[]; errors: string[] }> {
    const results: PatchResponse[] = [];
    const errors: string[] = [];

    for (const request of requests) {
      try {
        const patch = await this.generatePatch(request);
        results.push(patch);
      } catch (error) {
        errors.push(`Failed to generate patch for ${request.vulnerability.id}: ${error.message}`);
      }
    }

    return { results, errors };
  }

  /**
   * Get confidence level description
   */
  getConfidenceLevel(confidence: number): string {
    if (confidence >= 0.8) return 'High';
    if (confidence >= 0.6) return 'Medium';
    if (confidence >= 0.4) return 'Low';
    return 'Very Low';
  }

  /**
   * Check if patch should be applied based on confidence
   */
  shouldApplyPatch(confidence: number, minThreshold: number = 0.6): boolean {
    return confidence >= minThreshold;
  }

  /**
   * Generate summary of patch results
   */
  generatePatchSummary(patches: PatchResponse[]): {
    total: number;
    high_confidence: number;
    medium_confidence: number;
    low_confidence: number;
    passed_verification: number;
    fallback_used: number;
  } {
    const summary = {
      total: patches.length,
      high_confidence: 0,
      medium_confidence: 0,
      low_confidence: 0,
      passed_verification: 0,
      fallback_used: 0,
    };

    patches.forEach(patch => {
      // Confidence levels
      if (patch.confidence_score >= 0.8) summary.high_confidence++;
      else if (patch.confidence_score >= 0.6) summary.medium_confidence++;
      else summary.low_confidence++;

      // Verification status
      if (patch.verification_status === 'Passed') summary.passed_verification++;

      // Fallback usage
      if (patch.fallback_provided) summary.fallback_used++;
    });

    return summary;
  }
}
