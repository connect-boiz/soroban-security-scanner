import { Injectable, Logger } from '@nestjs/common';
import { Cron, CronExpression } from '@nestjs/schedule';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository, LessThan } from 'typeorm';
import { Scan } from './entities/scan.entity';
import * as fs from 'fs';
import * as path from 'path';

@Injectable()
export class GarbageCollectorService {
  private readonly logger = new Logger(GarbageCollectorService.name);
  private readonly TMP_DIR = path.join(process.cwd(), 'tmp', 'scans');

  constructor(
    @InjectRepository(Scan)
    private readonly scanRepository: Repository<Scan>,
  ) {
    // Ensure tmp directory exists
    if (!fs.existsSync(this.TMP_DIR)) {
      fs.mkdirSync(this.TMP_DIR, { recursive: true });
    }
  }

  /**
   * Weekly routine to clean up raw temporary files after scans finish.
   * Requirement: "Implement a garbage collection routine to clean up raw temporary files after a scan finishes."
   */
  @Cron(CronExpression.EVERY_DAY_AT_MIDNIGHT)
  async handleTemporaryFileCleanup() {
    this.logger.log('Starting garbage collection: Cleaning up temporary scan files...');
    
    try {
      const files = fs.readdirSync(this.TMP_DIR);
      const now = Date.now();
      const expirationTime = 24 * 60 * 60 * 1000; // 24 hours

      let count = 0;
      for (const file of files) {
        const filePath = path.join(this.TMP_DIR, file);
        const stats = fs.statSync(filePath);
        
        if (now - stats.mtimeMs > expirationTime) {
          fs.unlinkSync(filePath);
          count++;
        }
      }
      
      this.logger.log(`Garbage collection finished. Removed ${count} temporary files.`);
    } catch (error) {
      this.logger.error(`Failed to clean up temporary files: ${error.message}`);
    }
  }

  /**
   * Periodic routine to clean up old failed scans from the database.
   */
  @Cron(CronExpression.EVERY_WEEK)
  async handleOldScanCleanup() {
    this.logger.log('Starting garbage collection: Removing old failed/stale scans...');
    
    const oneWeekAgo = new Date();
    oneWeekAgo.setDate(oneWeekAgo.getDate() - 7);

    try {
      const deleteResult = await this.scanRepository.delete({
        status: 'failed',
        createdAt: LessThan(oneWeekAgo),
      });
      
      this.logger.log(`Removed ${deleteResult.affected} stale scans from the database.`);
    } catch (error) {
      this.logger.error(`Failed to remove old scans: ${error.message}`);
    }
  }

  /**
   * Helper method to "register" a temporary file for a scan (used by other services)
   */
  registerTempFile(scanId: string, content: string): string {
    const filePath = path.join(this.TMP_DIR, `${scanId}.rs`);
    fs.writeFileSync(filePath, content);
    return filePath;
  }
}
