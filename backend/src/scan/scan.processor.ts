import { Processor, WorkerHost, OnWorkerEvent } from '@nestjs/bullmq';
import { Job } from 'bullmq';
import { Logger } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { Scan } from './entities/scan.entity';
import { Vulnerability } from './entities/vulnerability.entity';
import { ScanProgressGateway } from './scan-progress.gateway';
import axios from 'axios';
import { ConfigService } from '@nestjs/config';

@Processor('scan-queue')
export class ScanProcessor extends WorkerHost {
  private readonly logger = new Logger(ScanProcessor.name);

  constructor(
    @InjectRepository(Scan)
    private readonly scanRepository: Repository<Scan>,
    @InjectRepository(Vulnerability)
    private readonly vulnerabilityRepository: Repository<Vulnerability>,
    private readonly scanProgressGateway: ScanProgressGateway,
    private readonly configService: ConfigService,
  ) {
    super();
  }

  async process(job: Job<any, any, string>): Promise<any> {
    const { scanId } = job.data;
    this.logger.log(`Processing scan job ${job.id} for scan ${scanId}`);

    const scan = await this.scanRepository.findOne({ where: { id: scanId } });
    if (!scan) {
      throw new Error(`Scan ${scanId} not found`);
    }

    try {
      // Step 1: Parsing
      await this.updateProgress(scan, 'parsing', 20, 'Parsing Soroban contract AST...');
      const astResponse = await axios.post(
        this.configService.get('AST_PARSER_URL', 'http://localhost:8080/parse'),
        { code: scan.code }
      );
      const ast = astResponse.data;

      // Step 2: Static Analysis (Vulnerability Pattern Matching)
      await this.updateProgress(scan, 'analysis', 50, 'Running static analysis patterns...');
      const analysisResponse = await axios.post(
        this.configService.get('VULN_SCANNER_URL', 'http://localhost:8082/scan'),
        { code: scan.code, filename: 'contract.rs', format: 'json' }
      );
      const analysisResults = parseRustScannerResults(analysisResponse.data);

      // Step 3: Invariant Fuzzing
      await this.updateProgress(scan, 'fuzzing', 80, 'Executing invariant fuzzing campaign...');
      const fuzzResponse = await axios.post(
        this.configService.get('FUZZER_URL', 'http://localhost:8081/fuzz'),
        { 
            wasm_base64: Buffer.from(scan.code).toString('base64'), // Simplified, should be compiled WASM
            function: 'deposit', 
            iterations: 100 
        }
      );
      const fuzzResults = fuzzResponse.data;

      // Step 4: Aggregation
      await this.updateProgress(scan, 'completed', 100, 'Aggregating security results...');
      
      const allVulnerabilities = [...analysisResults];
      if (!fuzzResults.success) {
          allVulnerabilities.push({
              type: 'Invariant Violation',
              severity: 'critical',
              title: 'Fuzzing Invariant Failed',
              description: fuzzResults.error_message || 'Contract invariant was violated during fuzzing',
              location: { file: 'contract.rs', line: 0, column: 0 },
              recommendation: 'Review the failing input sequence and fix state transition logic',
          });
      }

      // Save to database
      scan.status = 'completed';
      scan.metrics = this.calculateMetrics(allVulnerabilities, scan.code);
      await this.scanRepository.save(scan);

      for (const vuln of allVulnerabilities) {
        const entity = this.vulnerabilityRepository.create({
          scanId: scan.id,
          ...vuln,
        });
        await this.vulnerabilityRepository.save(entity);
      }

      this.scanProgressGateway.emitScanComplete(scan.id, {
        vulnerabilities: allVulnerabilities,
        metrics: scan.metrics,
      });

      return { success: true };

    } catch (error) {
      this.logger.error(`Scan ${scanId} failed: ${error.message}`);
      scan.status = 'failed';
      scan.errorMessage = error.message;
      await this.scanRepository.save(scan);
      this.scanProgressGateway.emitScanError(scanId, error.message);
      throw error; // Rethrow to trigger BullMQ retry
    }
  }

  private async updateProgress(scan: Scan, step: any, progress: number, message: string) {
    scan.currentStep = step;
    scan.progress = progress;
    scan.logs = [...(scan.logs || []), message];
    await this.scanRepository.save(scan);
    
    this.scanProgressGateway.emitScanProgress(scan.id, {
      currentStep: step,
      progress,
      message,
    });
    this.scanProgressGateway.emitScanLog(scan.id, message);
  }

  private calculateMetrics(vulnerabilities: any[], code: string): any {
    // Implementation omitted for brevity, same as in ScanService
    return { totalVulnerabilities: vulnerabilities.length };
  }

  @OnWorkerEvent('failed')
  onFailed(job: Job, error: Error) {
    this.logger.error(`Job ${job.id} failed: ${error.message}`);
  }
}

function parseRustScannerResults(data: any): any[] {
    // Map Rust scanner JSON to Backend's Vulnerability format
    if (typeof data === 'string') data = JSON.parse(data);
    return data.vulnerabilities || [];
}
