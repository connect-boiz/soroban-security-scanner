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

/**
 * Scan Service - Smart Contract Security Scanning Management
 *
 * Orchestrates the scanning workflow for Soroban smart contracts including:
 * - Scan creation and initialization
 * - Queueing scans for asynchronous processing
 * - Result retrieval and history management
 * - Progress tracking and real-time updates via WebSocket
 *
 * # Security Model
 *
 * ## Data Handling
 * - Scans are associated with users for access control
 * - Scan results contain vulnerability findings which may be sensitive
 * - Code is stored in the database; handle with care
 * - Scans are persisted for audit and compliance purposes
 *
 * ## Access Control
 * - All scan operations must verify user ownership
 * - Controllers should enforce permission checks
 * - Users can only access their own scans (except admins)
 *
 * ## Queue Management
 * - Uses BullMQ for distributed task processing
 * - Scans are queued asynchronously to prevent blocking
 * - Failed jobs are retained for debugging
 * - Job deduplication prevents duplicate processing
 *
 * ## Audit Trail
 * - Each scan has a unique ID (UUID v4) for tracking
 * - Status transitions are recorded (pending -> queued -> running -> completed/failed)
 * - Scan results include timestamp and findings
 * - Progress updates stream via WebSocket for real-time feedback
 *
 * # Threat Model
 * - **Unauthorized Access**: Users may try to access other users' scans
 * - **Resource Exhaustion**: Attackers may submit extremely large code for scanning
 * - **Privacy**: Scan results and code may contain sensitive information
 * - **Denial of Service**: Queue flooding could prevent legitimate scans
 *
 * # Security Considerations
 * - Always verify user owns the scan before returning results
 * - Implement rate limiting on scan creation
 * - Monitor queue depth for DoS attacks
 * - Archive old scans for compliance (retain for audit)
 * - Do not log sensitive code content in debug output
 */
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

  /**
   * Creates a new scan record for processing.
   *
   * Initializes a new scan in "pending" state. The actual scanning is performed
   * asynchronously via startScan(). This method prepares the database record.
   *
   * # Arguments
   * - `createScanDto`: Scan configuration including code and options
   *   - `code`: The contract code to scan (string)
   *   - `options`: Scan options (optional, passed to analyzer)
   * - `userId`: ID of user creating the scan (for access control)
   *
   * # Returns
   * `Scan` entity representing the created scan with:
   * - Unique ID (UUID v4)
   * - User ID for ownership tracking
   * - Code content (stored as-is)
   * - Options configuration
   * - Status: 'pending'
   * - Creation timestamp (auto-generated)
   *
   * # Security Considerations
   *
   * ## Data Storage
   * - Code is stored in database; ensure proper database encryption
   * - User ID is associated for access control enforcement
   * - No validation of code content here (done during analysis)
   * - Large code submissions could exhaust storage (implement limits in controller)
   *
   * ## Audit Trail
   * - Unique scan ID enables tracking through entire pipeline
   * - User ID attribution for accountability
   * - Creation timestamp for historical records
   * - Recommend logging scan creation for audit compliance
   *
   * ## Privacy
   * - Code may contain sensitive contract logic
   * - Database must be protected (encryption at rest)
   * - Access should be restricted to authorized users
   * - Consider GDPR/CCPA compliance for code retention
   *
   * # Errors
   * - May throw database errors if insert fails
   * - No validation errors (validation should happen in controller)
   *
   * # Important Notes
   * - Does NOT start the scan immediately (use startScan() for that)
   * - Does NOT submit to queue (use startScan() for that)
   * - Status is set to 'pending', not 'queued'
   * - Users should call startScan() after creating the scan
   *
   * # Example
   * ```typescript
   * const scan = await scanService.createScan(
   *   { code: contractCode, options: { deepAnalysis: true } },
   *   userId
   * );
   * // Returns scan with status='pending'
   * // User should then call startScan(scan.id)
   * ```
   */
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

  /**
   * Starts an asynchronous scan job.
   *
   * Transitions a pending scan to "queued" state and submits it to BullMQ for processing.
   * The actual scanning happens asynchronously in a worker process.
   *
   * # Arguments
   * - `scanId`: ID of the scan to start (must exist and be in pending state)
   *
   * # Security Considerations
   *
   * ## Access Control
   * - Controller must verify user owns this scan before calling
   * - This service method assumes scan ownership is already validated
   * - Does NOT check permissions (responsibility of controller)
   *
   * ## Queue Management
   * - Submits job to BullMQ with job ID = scan ID
   * - Job deduplication prevents duplicate processing
   * - Prevents both queue flooding and duplicate scans
   * - Failed jobs are retained for debugging (removeOnFail: false)
   * - Completed jobs are removed automatically (removeOnComplete: true)
   *
   * ## Progress Tracking
   * - Emits 'queued' status via WebSocket for real-time updates
   * - Initial progress message sent to clients
   * - Allows users to track scan progression
   * - Progress updates continue as scan runs
   *
   * ## Resource Management
   * - Queue depth should be monitored for DoS attacks
   * - Implement rate limiting in controller to prevent abuse
   * - Monitor worker pool for bottlenecks
   * - Consider implementing scan quotas per user
   *
   * # Errors
   * - Throws Error if scan not found
   * - May throw queue submission errors
   * - Database update errors propagate
   *
   * # Important Notes
   * - Status transitions from 'pending' to 'queued'
   * - Does not await actual scanning (async in background)
   * - WebSocket updates only sent to connected clients
   * - If client disconnects, no real-time updates
   *
   * # Example
   * ```typescript
   * // After createScan
   * const scan = await scanService.createScan(dto, userId);
   * // Start the actual scanning
   * await scanService.startScan(scan.id);
   * // Scan now runs asynchronously
   * // Users can poll getScan() or get real-time updates via WebSocket
   * ```
   */
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

  /**
   * Retrieves a scan with all associated vulnerabilities.
   *
   * Fetches complete scan data including results and findings.
   * This is the primary method for retrieving scan results to users.
   *
   * # Arguments
   * - `scanId`: ID of the scan to retrieve
   *
   * # Returns
   * `Scan` entity containing:
   * - Scan metadata (ID, user ID, creation time)
   * - Current status (pending, queued, running, completed, failed)
   * - Vulnerabilities array with all findings
   * - Analysis options and metrics
   *
   * # Security Considerations
   *
   * ## Access Control
   * - Controller must verify user owns this scan
   * - This method assumes ownership already verified
   * - Returns full scan data including potentially sensitive code
   * - Do not expose to unauthorized users
   *
   * ## Data Sensitivity
   * - Scan code may contain proprietary logic
   * - Vulnerability findings are sensitive security information
   * - Results should be shown only to scan owner and authorized users
   * - Consider audit logging for result access
   *
   * ## Performance
   * - Loads all vulnerabilities for the scan
   * - May be slow for large scans with many findings
   * - Consider pagination for very large result sets
   * - Database indices on scanId and userId are essential
   *
   * # Errors
   * - Throws Error if scan not found
   * - Returns null/undefined if scan doesn't exist (depends on impl)
   *
   * # Important Notes
   * - Returns vulnerabilities as related entities
   * - Includes both findings and scan metadata
   * - No filtering of results (caller responsibility)
   *
   * # Example
   * ```typescript
   * // After scan completes
   * const scan = await scanService.getScan(scanId);
   * console.log(`Status: ${scan.status}`);
   * console.log(`Found ${scan.vulnerabilities.length} issues`);
   * scan.vulnerabilities.forEach(v => {
   *   console.log(`[${v.severity}] ${v.name}`);
   * });
   * ```
   */
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

  /**
   * Retrieves paginated scan history for a user.
   *
   * Returns a list of scans created by the user with pagination support.
   * Used for scan list/history views and for compliance/audit purposes.
   *
   * # Arguments
   * - `userId`: ID of user whose scans to retrieve
   * - `page`: Page number (1-indexed, default 1)
   * - `limit`: Results per page (default 10)
   *
   * # Returns
   * Object containing:
   * - `scans`: Array of scans for current page
   * - `total`: Total number of scans (for pagination UI)
   *
   * Each scan includes all associated vulnerabilities.
   *
   * # Security Considerations
   *
   * ## Access Control
   * - Should only be called for the authenticated user
   * - Admin can view any user's history
   * - Non-admins can only view their own history
   * - Enforce in controller, not here
   *
   * ## Information Leakage
   * - Returns scan code and vulnerabilities
   * - Sensitive information must be protected
   * - Do not expose to other users
   * - Consider masking when viewing aggregate stats
   *
   * ## Pagination
   * - Essential for handling users with many scans
   * - Page numbers are 1-indexed for users
   * - Limit parameter controls results per page
   * - Total count enables pagination UI calculation
   * - Query is ordered by creation date (newest first)
   *
   * ## Performance
   * - Pagination prevents loading entire history
   * - Database indices on userId and createdAt essential
   * - Consider caching for frequently accessed pages
   * - Monitor query performance for users with 1000+ scans
   *
   * # Errors
   * - May throw database errors
   * - Returns empty array if no scans found
   *
   * # Important Notes
   * - Results ordered by creation date (DESC - newest first)
   * - Includes all vulnerabilities for each scan
   * - Large scans may cause performance issues
   * - Pagination helps manage large datasets
   *
   * # Example
   * ```typescript
   * // Get second page of user's scans (10 per page)
   * const result = await scanService.getScanHistory(userId, 2, 10);
   * console.log(`Showing scans ${11}-${20} of ${result.total}`);
   * result.scans.forEach(scan => {
   *   console.log(`${scan.createdAt}: ${scan.vulnerabilities.length} issues`);
   * });
   * ```
   */
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

  /**
   * Retrieves aggregate statistics across all scans.
   *
   * Provides system-wide scanning metrics for dashboards and reporting.
   * Currently returns placeholder stats; implementation needed.
   *
   * # Returns
   * Object containing:
   * - `totalScans`: Total number of scans in system
   * - `totalVulnerabilities`: Total findings across all scans
   * - `averageRiskScore`: Average risk across scans
   * - `topVulnerabilityTypes`: Most common vulnerability types
   * - `recentScans`: Recent scan summaries
   *
   * # Security Considerations
   *
   * ## Access Control
   * - Should be restricted to admins or organization role
   * - Aggregate stats can leak information about other users' scans
   * - Do not expose to regular users without authorization
   * - Enforce permission checks in controller
   *
   * ## Information Disclosure
   * - Aggregate stats may reveal patterns in scanning behavior
   * - Top vulnerability types could reveal contract patterns
   * - Consider anonymizing or aggregating further for wider audiences
   * - Be careful exposing to competitors or public
   *
   * ## Performance
   * - Computing full aggregation is expensive
   * - Consider caching results (update hourly)
   * - May need database optimization queries
   * - Background job recommended for large datasets
   *
   * ## Implementation Notes
   * - Currently returns hardcoded placeholder values
   * - Needs implementation to compute actual statistics
   * - Consider aggregating in separate background job
   * - Store pre-computed stats for fast retrieval
   *
   * # Errors
   * - May throw database errors during calculation
   * - Returns placeholder if calculation fails
   *
   * # Future Work
   * - Implement actual aggregation queries
   * - Add filtering by date range
   * - Add organization/team filtering
   * - Cache results for performance
   * - Archive historical stats
   *
   * # Example
   * ```typescript
   * const stats = await scanService.getScanStats();
   * console.log(`Total scans: ${stats.totalScans}`);
   * console.log(`Total vulnerabilities: ${stats.totalVulnerabilities}`);
   * console.log(`Average risk score: ${stats.averageRiskScore.toFixed(2)}`);
   * ```
   */
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
