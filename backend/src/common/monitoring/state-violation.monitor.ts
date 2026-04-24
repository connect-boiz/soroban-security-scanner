import { Injectable, Logger } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { StateValidationError } from '../validation/state-consistency.validator';

export interface StateViolationMetrics {
  totalViolations: number;
  violationsByEntity: Record<string, number>;
  violationsByTransition: Record<string, number>;
  recentViolations: StateValidationError[];
  violationRate: number; // violations per hour
}

export interface StateViolationAlert {
  severity: 'low' | 'medium' | 'high' | 'critical';
  message: string;
  entity: string;
  entityId: string;
  violation: StateValidationError;
  timestamp: Date;
}

@Injectable()
export class StateViolationMonitor {
  private readonly logger = new Logger(StateViolationMonitor.name);
  private readonly violations: StateValidationError[] = [];
  private readonly metrics: StateViolationMetrics = {
    totalViolations: 0,
    violationsByEntity: {},
    violationsByTransition: {},
    recentViolations: [],
    violationRate: 0,
  };

  constructor(private readonly configService: ConfigService) {
    // Set up periodic metrics calculation
    setInterval(() => this.calculateMetrics(), 60000); // Every minute
  }

  recordViolation(violation: StateValidationError): void {
    // Add to violations list
    this.violations.push(violation);
    
    // Keep only last 1000 violations to prevent memory issues
    if (this.violations.length > 1000) {
      this.violations.shift();
    }

    // Update metrics
    this.updateMetrics(violation);

    // Check for alerts
    this.checkAlerts(violation);

    // Log structured violation
    this.logViolation(violation);
  }

  private updateMetrics(violation: StateValidationError): void {
    this.metrics.totalViolations++;
    
    // Update entity metrics
    const entityKey = violation.entity;
    this.metrics.violationsByEntity[entityKey] = (this.metrics.violationsByEntity[entityKey] || 0) + 1;
    
    // Update transition metrics
    const transitionKey = `${violation.currentState} -> ${violation.targetState}`;
    this.metrics.violationsByTransition[transitionKey] = (this.metrics.violationsByTransition[transitionKey] || 0) + 1;
    
    // Update recent violations (last hour)
    const oneHourAgo = new Date(Date.now() - 3600000);
    this.metrics.recentViolations = this.violations.filter(v => 
      new Date(v.context?.timestamp || Date.now()) > oneHourAgo
    );
    
    // Calculate violation rate
    this.metrics.violationRate = this.metrics.recentViolations.length;
  }

  private checkAlerts(violation: StateValidationError): void {
    const alerts = this.generateAlerts(violation);
    
    for (const alert of alerts) {
      this.handleAlert(alert);
    }
  }

  private generateAlerts(violation: StateValidationError): StateViolationAlert[] {
    const alerts: StateViolationAlert[] = [];
    
    // High frequency violations
    const entityViolations = this.metrics.violationsByEntity[violation.entity] || 0;
    if (entityViolations >= 10) {
      alerts.push({
        severity: 'high',
        message: `High frequency of state violations for ${violation.entity}: ${entityViolations} total`,
        entity: violation.entity,
        entityId: violation.entityId,
        violation,
        timestamp: new Date(),
      });
    }
    
    // Critical entities
    if (['scan', 'escrow'].includes(violation.entity)) {
      alerts.push({
        severity: 'medium',
        message: `State violation in critical entity: ${violation.entity}`,
        entity: violation.entity,
        entityId: violation.entityId,
        violation,
        timestamp: new Date(),
      });
    }
    
    // Suspicious patterns (same transition failing repeatedly)
    const transitionKey = `${violation.currentState} -> ${violation.targetState}`;
    const transitionViolations = this.metrics.violationsByTransition[transitionKey] || 0;
    if (transitionViolations >= 5) {
      alerts.push({
        severity: 'medium',
        message: `Repeated failure for transition: ${transitionKey}`,
        entity: violation.entity,
        entityId: violation.entityId,
        violation,
        timestamp: new Date(),
      });
    }
    
    // Critical violations (completed -> running, released -> pending, etc.)
    const criticalTransitions = [
      'completed -> running',
      'released -> pending',
      'resolved -> detected',
    ];
    if (criticalTransitions.includes(transitionKey)) {
      alerts.push({
        severity: 'critical',
        message: `Critical state violation: ${transitionKey}`,
        entity: violation.entity,
        entityId: violation.entityId,
        violation,
        timestamp: new Date(),
      });
    }
    
    return alerts;
  }

  private handleAlert(alert: StateViolationAlert): void {
    // Log alert
    this.logger.warn(`State Violation Alert [${alert.severity.toUpperCase()}]: ${alert.message}`, {
      entity: alert.entity,
      entityId: alert.entityId,
      violation: alert.violation,
      timestamp: alert.timestamp,
    });

    // Send to monitoring systems in production
    if (this.configService.get('NODE_ENV') === 'production') {
      this.sendToMonitoringSystem(alert);
    }

    // Could also send to Slack, email, etc. based on severity
    if (alert.severity === 'critical') {
      this.sendCriticalAlert(alert);
    }
  }

  private sendToMonitoringSystem(alert: StateViolationAlert): void {
    // Integration with monitoring systems like DataDog, New Relic, etc.
    this.logger.log('Alert sent to monitoring system', {
      alertType: 'state_violation',
      severity: alert.severity,
      entity: alert.entity,
      message: alert.message,
    });
  }

  private sendCriticalAlert(alert: StateViolationAlert): void {
    // Send immediate notification for critical alerts
    // This could be Slack, PagerDuty, SMS, etc.
    this.logger.error('CRITICAL STATE VIOLATION DETECTED', {
      alert,
      requiresImmediateAttention: true,
    });
  }

  private logViolation(violation: StateValidationError): void {
    // Structured logging for analysis
    this.logger.error('State Consistency Violation', {
      entityType: violation.entity,
      entityId: violation.entityId,
      currentState: violation.currentState,
      targetState: violation.targetState,
      error: violation.error,
      context: violation.context,
      timestamp: new Date().toISOString(),
      violationId: this.generateViolationId(violation),
    });
  }

  private generateViolationId(violation: StateValidationError): string {
    const timestamp = Date.now().toString(36);
    const entity = violation.entity.substring(0, 3);
    const random = Math.random().toString(36).substring(2, 6);
    return `vio_${entity}_${timestamp}_${random}`;
  }

  private calculateMetrics(): void {
    // Recalculate metrics periodically
    const oneHourAgo = new Date(Date.now() - 3600000);
    this.metrics.recentViolations = this.violations.filter(v => 
      new Date(v.context?.timestamp || Date.now()) > oneHourAgo
    );
    this.metrics.violationRate = this.metrics.recentViolations.length;
  }

  getMetrics(): StateViolationMetrics {
    return { ...this.metrics };
  }

  getRecentViolations(limit: number = 50): StateValidationError[] {
    return this.violations.slice(-limit);
  }

  getViolationsByEntity(entity: string): StateValidationError[] {
    return this.violations.filter(v => v.entity === entity);
  }

  getViolationTrends(hours: number = 24): Array<{ hour: string; count: number }> {
    const now = new Date();
    const trends: Array<{ hour: string; count: number }> = [];
    
    for (let i = hours - 1; i >= 0; i--) {
      const hourStart = new Date(now.getTime() - (i + 1) * 3600000);
      const hourEnd = new Date(now.getTime() - i * 3600000);
      
      const count = this.violations.filter(v => {
        const violationTime = new Date(v.context?.timestamp || Date.now());
        return violationTime >= hourStart && violationTime < hourEnd;
      }).length;
      
      trends.push({
        hour: hourStart.getHours().toString(),
        count,
      });
    }
    
    return trends;
  }

  clearOldViolations(olderThanHours: number = 24): void {
    const cutoffTime = new Date(Date.now() - olderThanHours * 3600000);
    const initialLength = this.violations.length;
    
    // Remove old violations
    for (let i = this.violations.length - 1; i >= 0; i--) {
      const violationTime = new Date(this.violations[i].context?.timestamp || Date.now());
      if (violationTime < cutoffTime) {
        this.violations.splice(i, 1);
      }
    }
    
    const removed = initialLength - this.violations.length;
    if (removed > 0) {
      this.logger.log(`Cleared ${removed} old state violations`);
    }
  }

  // Health check method
  isHealthy(): boolean {
    // Consider system unhealthy if too many violations in last hour
    const recentViolations = this.metrics.recentViolations.length;
    const threshold = this.configService.get<number>('STATE_VIOLATION_THRESHOLD', 50);
    
    return recentViolations < threshold;
  }

  getHealthStatus(): {
    healthy: boolean;
    violations: number;
    threshold: number;
    lastViolation?: Date;
  } {
    const recentViolations = this.metrics.recentViolations.length;
    const threshold = this.configService.get<number>('STATE_VIOLATION_THRESHOLD', 50);
    
    return {
      healthy: recentViolations < threshold,
      violations: recentViolations,
      threshold,
      lastViolation: this.violations.length > 0 
        ? new Date(this.violations[this.violations.length - 1].context?.timestamp || Date.now())
        : undefined,
    };
  }
}
