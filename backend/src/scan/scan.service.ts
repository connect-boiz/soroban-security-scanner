import { Injectable, Logger } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { v4 as uuidv4 } from 'uuid';
import { Scan } from './entities/scan.entity';
import { Vulnerability } from './entities/vulnerability.entity';
import { CreateScanDto } from './dto/create-scan.dto';
import { ScanProgressGateway } from './scan-progress.gateway';
import { InjectQueue } from '@nestjs/bullmq';
import { Queue } from 'bullmq';

@Injectable()
export class ScanService {
  private readonly logger = new Logger(ScanService.name);

  constructor(
    @InjectRepository(Scan)
    private readonly scanRepository: Repository<Scan>,
    @InjectRepository(Vulnerability)
    private readonly vulnerabilityRepository: Repository<Vulnerability>,
    private readonly scanProgressGateway: ScanProgressGateway,
    @InjectQueue('scan-queue')
    private readonly scanQueue: Queue,
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

    scan.status = 'queued';
    await this.scanRepository.save(scan);

    // Initial progress update
    this.scanProgressGateway.emitScanStatus(scanId, 'queued');
    this.scanProgressGateway.emitScanProgress(scanId, {
      currentStep: 'queued' as any,
      progress: 0,
      message: 'Scan has been queued and is waiting for a worker...'
    });

    // Add to BullMQ
    await this.scanQueue.add('process-scan', { scanId }, {
        jobId: scanId, // Prevent duplicate jobs for the same scan
        removeOnComplete: true,
        removeOnFail: false,
    });
    
    this.logger.log(`Scan ${scanId} added to the queue`);
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
    return {
      totalScans: 0,
      totalVulnerabilities: 0,
      averageRiskScore: 0,
      topVulnerabilityTypes: [],
      recentScans: [],
    };
  }
}
