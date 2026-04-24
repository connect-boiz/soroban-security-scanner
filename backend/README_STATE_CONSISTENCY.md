# State Consistency Implementation

This document describes the comprehensive state consistency validation framework implemented for the Soroban Security Scanner backend, addressing issue #120 for complex state changes.

## Overview

The state consistency system ensures that all entity state transitions follow defined rules and maintain internal consistency. This prevents invalid states, data corruption, and business logic violations.

## Architecture

### Core Components

1. **StateConsistencyValidator** - Central validation engine with state machines
2. **StateTransitionInterceptor** - Automatic validation middleware
3. **StateViolationMonitor** - Monitoring and alerting system
4. **StateTransition Decorators** - Declarative validation configuration

### State Machines

#### Scan State Machine
```
pending → queued → running → completed/failed
```

**Valid Transitions:**
- `pending → queued` (requires ID, user ID, and code)
- `queued → running` (requires queued status)
- `running → completed` (requires metrics)
- `running/queued → failed` (allows failure from running or queued)

#### Escrow State Machine
```
pending → locked → released/refunded
```

**Valid Transitions:**
- `pending → locked` (requires valid amount, depositor, beneficiary)
- `locked → released` (requires unlocked, conditions met, authorized)
- `pending/locked → refunded` (requires depositor authorization)

#### Vulnerability State Machine
```
detected → analyzing → confirmed/false_positive → mitigated → resolved
```

**Valid Transitions:**
- `detected → analyzing` (requires ID and scan ID)
- `analyzing → confirmed` (requires severity and description)
- `analyzing → false_positive` (requires reason)
- `confirmed → mitigated` (requires mitigation strategy)
- `confirmed/mitigated → resolved` (requires resolution proof)

#### API Key State Machine
```
active → suspended → revoked
```

**Valid Transitions:**
- `active → suspended` (requires reason and admin ID)
- `active/suspended → revoked` (requires reason and admin ID)

## Implementation Details

### Validation Rules

#### State Transition Validation
- **Pre-transition checks**: Validate current state and transition validity
- **Custom validators**: Entity-specific business logic validation
- **Context validation**: User permissions, business rules, timing constraints
- **Post-transition checks**: Entity consistency validation

#### Entity Consistency Checks

**Scan Consistency:**
- Status vs currentStep alignment
- Progress vs status consistency
- Metrics presence for completed scans
- No metrics for pending scans

**Escrow Consistency:**
- Positive amount validation
- Status vs conditions_met consistency
- Lock period validation
- Release signature requirements

**Vulnerability Consistency:**
- Severity validation
- Scan association requirements
- Status-specific field requirements

### Monitoring and Alerting

#### Violation Metrics
- Total violations count
- Violations by entity type
- Violations by transition type
- Recent violations (last hour)
- Violation rate per hour

#### Alert Levels
- **Low**: Informational violations
- **Medium**: Critical entity violations, repeated failures
- **High**: High-frequency violations (>10 per entity)
- **Critical**: Impossible transitions (completed → running, released → pending)

#### Monitoring Features
- Real-time violation tracking
- Historical trend analysis
- Health status monitoring
- Automated alert routing

## Usage Examples

### Service Integration

```typescript
// In service methods
const validation = await this.stateValidator.validateStateTransition(
  'scan',
  scanId,
  originalStatus,
  'queued',
  scan,
  { userId: scan.userId }
);

if (!validation.valid) {
  this.stateValidator.logStateViolation(validation.error!);
  throw new BadRequestException(`Invalid state transition: ${validation.error!.error}`);
}
```

### Decorator Usage

```typescript
@ValidateScanStateTransition()
async updateScanStatus(scanId: string, newStatus: string) {
  // Automatic validation via interceptor
}
```

### Custom Validators

```typescript
transitions: [
  {
    from: 'pending',
    to: 'queued',
    validator: (scan) => {
      return scan.id && scan.userId && scan.code;
    },
    errorMessage: 'Scan must have ID, user ID, and code to be queued'
  }
]
```

## Configuration

### Environment Variables

```bash
# State consistency monitoring
STATE_VIOLATION_THRESHOLD=50  # Max violations per hour for healthy status
NODE_ENV=production             # Enable monitoring in production
```

### Module Configuration

```typescript
providers: [
  StateConsistencyValidator,
  StateViolationMonitor,
  {
    provide: APP_INTERCEPTORS,
    useClass: StateConsistencyInterceptor,
  },
],
```

## Error Handling

### Validation Errors

```json
{
  "error": "State transition validation failed",
  "message": "Cannot start scan: Scan must be queued to start running",
  "details": {
    "entity": "scan",
    "entityId": "scan-123",
    "currentState": "pending",
    "targetState": "running",
    "error": "Invalid state transition from pending to running"
  }
}
```

### Consistency Errors

```json
{
  "error": "Entity consistency validation failed",
  "message": "Scan consistency validation failed: Scan status is completed but currentStep is not completed, Completed scan must have 100% progress"
}
```

## Testing

### Unit Tests
- State transition validation
- Entity consistency checks
- Custom validator logic
- Error handling scenarios

### Integration Tests
- End-to-end state transitions
- Interceptor functionality
- Monitoring integration
- Alert generation

### Test Coverage

```typescript
// Example test
it('should reject invalid scan state transition', async () => {
  const result = await validator.validateStateTransition(
    'scan',
    'scan-123',
    'completed',
    'running',
    completedScan,
    {}
  );

  expect(result.valid).toBe(false);
  expect(result.error!.error).toContain('Invalid state transition');
});
```

## Monitoring Dashboard

### Metrics Available
- Real-time violation count
- Violation trends (24-hour window)
- Entity-specific violation rates
- Most problematic transitions
- System health status

### Health Endpoints

```typescript
// Health check integration
health.dependencies.stateConsistency = monitor.isHealthy() ? 'healthy' : 'unhealthy';
```

## Performance Considerations

### Optimization Strategies
- **In-memory state machines**: Fast lookup and validation
- **Async validation**: Non-blocking state checks
- **Batch monitoring**: Periodic metrics calculation
- **Memory management**: Automatic cleanup of old violations

### Scaling
- **Distributed validation**: Each instance validates independently
- **Centralized monitoring**: Aggregate violations across instances
- **Rate limiting**: Prevent validation abuse
- **Caching**: Cache frequently accessed state machines

## Security Considerations

### Access Control
- **User context validation**: Ensure users can only transition their own entities
- **Admin overrides**: Allow administrators to bypass certain validations
- **Audit trail**: Log all state violations for security analysis

### Data Protection
- **Sensitive data handling**: Don't log sensitive entity data
- **Privacy compliance**: Anonymize violation data where required
- **Retention policies**: Automatic cleanup of old violation data

## Troubleshooting

### Common Issues

1. **State transition failures**
   - Check entity prerequisites
   - Verify user permissions
   - Validate business rules

2. **Consistency check failures**
   - Review entity state alignment
   - Check required field presence
   - Validate business logic

3. **High violation rates**
   - Monitor for application bugs
   - Check for malicious activity
   - Review business rule changes

### Debug Mode

```bash
LOG_LEVEL=debug
```

Enables detailed logging of:
- State transition decisions
- Validation rule execution
- Consistency check results
- Monitoring metrics

## Future Enhancements

### Planned Features
1. **Dynamic state machines**: Runtime state machine configuration
2. **Visual state diagrams**: Interactive state flow visualization
3. **Machine learning**: Anomaly detection in state transitions
4. **GraphQL integration**: State consistency in API layer

### Extensibility
- **Plugin architecture**: Custom validation plugins
- **Webhook integration**: External monitoring systems
- **API access**: Programmatic state management
- **Multi-tenant support**: Organization-specific state rules

## Migration Guide

### Adding New Entities
1. Define state machine in `StateConsistencyValidator`
2. Add entity-specific consistency checks
3. Create decorators for common transitions
4. Add monitoring rules
5. Write comprehensive tests

### Updating Existing Rules
1. Modify state machine configuration
2. Update custom validators
3. Review impact on existing data
4. Update documentation
5. Test thoroughly

This implementation provides robust state consistency validation that prevents data corruption, ensures business rule compliance, and provides comprehensive monitoring for the Soroban Security Scanner platform.
