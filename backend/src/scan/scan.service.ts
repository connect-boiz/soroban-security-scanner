import { Injectable, Logger } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { v4 as uuidv4 } from 'uuid';
import { Scan } from './entities/scan.entity';
import { Vulnerability } from './entities/vulnerability.entity';
import { CreateScanDto } from './dto/create-scan.dto';

@Injectable()
export class ScanService {
  private readonly logger = new Logger(ScanService.name);

  constructor(
    @InjectRepository(Scan)
    private readonly scanRepository: Repository<Scan>,
    @InjectRepository(Vulnerability)
    private readonly vulnerabilityRepository: Repository<Vulnerability>,
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
    await this.scanRepository.save(scan);

    try {
      const result = await this.performAnalysis(scan);
      
      // Update scan with results
      scan.status = 'completed';
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
    } catch (error) {
      scan.status = 'failed';
      scan.errorMessage = error instanceof Error ? error.message : 'Unknown error';
      await this.scanRepository.save(scan);
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
  }> {
    const startTime = Date.now();
    this.logger.log(`Starting scan analysis for scan ${scan.id}`);

    // Mock vulnerability detection (replace with actual core scanner integration)
    const vulnerabilities = this.detectMockVulnerabilities(scan.code);
    
    // Calculate metrics
    const metrics = this.calculateMetrics(vulnerabilities, scan.code);
    
    const scanTime = Date.now() - startTime;

    this.logger.log(`Scan analysis completed for scan ${scan.id}, found ${vulnerabilities.length} vulnerabilities`);

    return {
      vulnerabilities,
      metrics,
      scanTime,
    };
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
}
