import { Injectable, Logger } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { v4 as uuidv4 } from 'uuid';
import { Scan } from './entities/scan.entity';
import { Vulnerability } from './entities/vulnerability.entity';
import { CreateScanDto } from './dto/create-scan.dto';
import { ScanProgressGateway } from './scan-progress.gateway';

@Injectable()
export class ScanService {
  private readonly logger = new Logger(ScanService.name);

  constructor(
    @InjectRepository(Scan)
    private readonly scanRepository: Repository<Scan>,
    @InjectRepository(Vulnerability)
    private readonly vulnerabilityRepository: Repository<Vulnerability>,
    private readonly scanProgressGateway: ScanProgressGateway,
  ) {}

  async createScan(createScanDto: CreateScanDto, userId: string): Promise<Scan> {
    const scan = this.scanRepository.create({
      id: uuidv4(),
      userId,
      code: createScanDto.code,
      options: createScanDto.options || {},
      status: 'pending',
    });

    return await this.scanRepository.save(scan);
  }

  async startScan(scanId: string): Promise<void> {
    const scan = await this.scanRepository.findOne({ where: { id: scanId } });
    if (!scan) {
      throw new Error('Scan not found');
    }

    scan.status = 'running';
    scan.currentStep = 'uploading';
    scan.progress = 0;
    scan.logs = ['Starting scan...'];
    await this.scanRepository.save(scan);

    // Emit initial status
    this.scanProgressGateway.emitScanStatus(scanId, 'running');
    this.scanProgressGateway.emitScanProgress(scanId, {
      currentStep: 'uploading',
      progress: 0,
      message: 'Uploading contract code...'
    });

    try {
      const result = await this.performAnalysis(scan);
      
      // Update scan with results
      scan.status = 'completed';
      scan.progress = 100;
      scan.currentStep = 'completed';
      scan.metrics = result.metrics;
      scan.scanTime = result.scanTime;
      
      // Save vulnerabilities
      for (const vuln of result.vulnerabilities) {
        const vulnerability = this.vulnerabilityRepository.create({
          id: uuidv4(),
          scanId: scan.id,
          ...vuln,
        });
        await this.vulnerabilityRepository.save(vulnerability);
      }

      await this.scanRepository.save(scan);

      // Emit completion event
      this.scanProgressGateway.emitScanComplete(scanId, {
        vulnerabilities: result.vulnerabilities,
        metrics: result.metrics,
        scanTime: result.scanTime,
        zeroVulnerabilities: result.vulnerabilities.length === 0
      });

    } catch (error) {
      scan.status = 'failed';
      scan.errorMessage = error instanceof Error ? error.message : 'Unknown error';
      scan.currentStep = 'error';
      await this.scanRepository.save(scan);

      // Emit error event
      this.scanProgressGateway.emitScanError(scanId, scan.errorMessage);
      throw error;
    }
  }

  async getScan(scanId: string): Promise<Scan> {
    const scan = await this.scanRepository.findOne({
      where: { id: scanId },
      relations: ['vulnerabilities'],
    });

    if (!scan) {
      throw new Error('Scan not found');
    }

    return scan;
  }

  async getScanHistory(userId: string, page: number = 1, limit: number = 10): Promise<{ scans: Scan[], total: number }> {
    const [scans, total] = await this.scanRepository.findAndCount({
      where: { userId },
      relations: ['vulnerabilities'],
      order: { createdAt: 'DESC' },
      skip: (page - 1) * limit,
      take: limit,
    });

    return { scans, total };
  }

  async getScanStats(): Promise<any> {
    // Mock implementation - in reality, this would aggregate data from the database
    return {
      totalScans: 0,
      totalVulnerabilities: 0,
      averageRiskScore: 0,
      topVulnerabilityTypes: [],
      recentScans: [],
    };
  }

  private async performAnalysis(scan: Scan): Promise<{
    vulnerabilities: any[];
    metrics: any;
    scanTime: number;
    sarifReport?: any;
  }> {
    const startTime = Date.now();
    this.logger.log(`Starting scan analysis for scan ${scan.id}`);

    try {
      // Step 1: Parsing
      scan.currentStep = 'parsing';
      scan.progress = 25;
      scan.logs = [...(scan.logs || []), 'Parsing contract code...'];
      await this.scanRepository.save(scan);
      
      this.scanProgressGateway.emitScanProgress(scan.id, {
        currentStep: 'parsing',
        progress: 25,
        message: 'Parsing contract code...'
      });
      this.scanProgressGateway.emitScanLog(scan.id, 'Parsing contract code...');

      // Simulate parsing delay
      await this.delay(1000);

      // Step 2: Fuzzing
      scan.currentStep = 'fuzzing';
      scan.progress = 50;
      scan.logs = [...(scan.logs || []), 'Running fuzzing tests...'];
      await this.scanRepository.save(scan);
      
      this.scanProgressGateway.emitScanProgress(scan.id, {
        currentStep: 'fuzzing',
        progress: 50,
        message: 'Running fuzzing tests...'
      });
      this.scanProgressGateway.emitScanLog(scan.id, 'Running fuzzing tests...');

      // Simulate fuzzing delay
      await this.delay(2000);

      // Step 3: Analysis
      scan.currentStep = 'analysis';
      scan.progress = 75;
      scan.logs = [...(scan.logs || []), 'Analyzing for vulnerabilities...'];
      await this.scanRepository.save(scan);
      
      this.scanProgressGateway.emitScanProgress(scan.id, {
        currentStep: 'analysis',
        progress: 75,
        message: 'Analyzing for vulnerabilities...'
      });
      this.scanProgressGateway.emitScanLog(scan.id, 'Analyzing for vulnerabilities...');

      // Mock vulnerability detection (replace with actual core scanner integration)
      const vulnerabilities = this.detectMockVulnerabilities(scan.code);
      
      // Calculate metrics
      const metrics = this.calculateMetrics(vulnerabilities, scan.code);
      
      // Generate SARIF report
      const sarifReport = this.generateSarifReport(vulnerabilities, scan);
      
      // Step 4: Reporting
      scan.currentStep = 'reporting';
      scan.progress = 90;
      scan.logs = [...(scan.logs || []), 'Generating security report...'];
      await this.scanRepository.save(scan);
      
      this.scanProgressGateway.emitScanProgress(scan.id, {
        currentStep: 'reporting',
        progress: 90,
        message: 'Generating security report...'
      });
      this.scanProgressGateway.emitScanLog(scan.id, 'Generating security report...');

      // Simulate report generation delay
      await this.delay(1000);

      const scanTime = Date.now() - startTime;

      this.logger.log(`Scan analysis completed for scan ${scan.id}, found ${vulnerabilities.length} vulnerabilities`);

      return {
        vulnerabilities,
        metrics,
        scanTime,
        sarifReport,
      };
    } catch (error) {
      // Handle WASM compilation errors specifically
      if (error instanceof Error && error.message.includes('WASM compilation failed')) {
        this.scanProgressGateway.emitScanError(scan.id, `WASM Compilation Error: ${error.message}`);
        throw error;
      }
      throw error;
    }
  }

  private delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  private detectMockVulnerabilities(code: string): any[] {
    const vulnerabilities = [];

    // Example mock vulnerabilities
    if (code.includes('pub fn') && !code.includes('require_auth')) {
      vulnerabilities.push({
        type: 'Access Control',
        severity: 'critical',
        title: 'Missing Access Control',
        description: 'Public function lacks access control checks',
        location: {
          file: 'contract.rs',
          line: 1,
          column: 1,
        },
        recommendation: 'Add require_auth() or similar access control checks',
        cweId: 'CWE-284',
      });
    }

    if (code.includes('token.mint')) {
      vulnerabilities.push({
        type: 'Token Economics',
        severity: 'high',
        title: 'Potential Infinite Mint',
        description: 'Token minting function may lack proper limits',
        location: {
          file: 'contract.rs',
          line: 2,
          column: 1,
        },
        recommendation: 'Implement total supply limits and minting controls',
        cweId: 'CWE-400',
      });
    }

    return vulnerabilities;
  }

  private calculateMetrics(vulnerabilities: any[], code: string): any {
    const linesOfCode = code.split('\n').length;
    
    const severityCounts = vulnerabilities.reduce((acc, vuln) => {
      acc[vuln.severity + 'Count'] = (acc[vuln.severity + 'Count'] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);

    const riskScore = vulnerabilities.reduce((score, vuln) => {
      const severityScore = {
        critical: 10,
        high: 7,
        medium: 4,
        low: 1,
      };
      return score + severityScore[vuln.severity];
    }, 0);

    return {
      totalVulnerabilities: vulnerabilities.length,
      criticalCount: severityCounts.criticalCount || 0,
      highCount: severityCounts.highCount || 0,
      mediumCount: severityCounts.mediumCount || 0,
      lowCount: severityCounts.lowCount || 0,
      riskScore,
      linesOfCode,
    };
  }

  private generateSarifReport(vulnerabilities: any[], scan: Scan): any {
    const sarifReport = {
      version: '2.1.0',
      $schema: 'https://json.schemastore.org/sarif-2.1.0',
      runs: [{
        tool: {
          driver: {
            name: 'Soroban Security Scanner',
            version: '1.0.0',
            rules: this.generateSarifRules(vulnerabilities)
          }
        },
        results: vulnerabilities.map(vuln => this.convertVulnerabilityToSarifResult(vuln)),
        artifacts: [{
          location: {
            uri: 'contract.rs'
          },
          length: scan.code.length,
          mimeType: 'text/x-rust'
        }]
      }]
    };

    return sarifReport;
  }

  private generateSarifRules(vulnerabilities: any[]): any[] {
    const ruleMap = new Map();
    
    vulnerabilities.forEach(vuln => {
      if (!ruleMap.has(vuln.cweId || vuln.type)) {
        ruleMap.set(vuln.cweId || vuln.type, {
          id: vuln.cweId || vuln.type,
          name: vuln.title || vuln.type,
          shortDescription: {
            text: vuln.title || vuln.type
          },
          fullDescription: {
            text: vuln.description || vuln.recommendation
          },
          defaultConfiguration: {
            level: this.getSeverityLevel(vuln.severity)
          },
          help: {
            text: vuln.recommendation
          },
          properties: {
            category: vuln.type,
            tags: [vuln.type.toLowerCase().replace(/\s+/g, '-')]
          }
        });
      }
    });

    return Array.from(ruleMap.values());
  }

  private convertVulnerabilityToSarifResult(vuln: any): any {
    return {
      ruleId: vuln.cweId || vuln.type,
      level: this.getSeverityLevel(vuln.severity),
      message: {
        text: vuln.description || vuln.title
      },
      locations: [{
        physicalLocation: {
          artifactLocation: {
            uri: vuln.location.file || 'contract.rs'
          },
          region: {
            startLine: vuln.location.line,
            startColumn: vuln.location.column || 1,
            endLine: vuln.location.line,
            endColumn: (vuln.location.column || 1) + 20
          }
        }
      }]
    };
  }

  private getSeverityLevel(severity: string): string {
    switch (severity.toLowerCase()) {
      case 'critical':
        return 'error';
      case 'high':
        return 'warning';
      case 'medium':
        return 'note';
      case 'low':
        return 'info';
      default:
        return 'note';
    }
  }
}
