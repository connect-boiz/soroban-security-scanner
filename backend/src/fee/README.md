# Fee Mechanism Implementation

This document describes the comprehensive fee mechanism implemented for the Soroban Security Scanner to prevent spam and fund maintenance operations.

## Overview

The fee mechanism provides:
- **Operation-based pricing** for scans, API calls, storage, and premium features
- **Dynamic fee calculation** based on complexity, code size, and resource usage
- **User balance management** with credit limits and usage tracking
- **Fee validation and collection** with automatic deduction
- **Refund capabilities** for failed operations or customer requests
- **Comprehensive audit trail** for all fee transactions

## Architecture

### Core Components

1. **Fee Entities**
   - `Fee`: Records all fee transactions with status tracking
   - `UserBalance`: Manages user balances and usage statistics

2. **Fee Services**
   - `FeeCalculatorService`: Calculates fees based on operation parameters
   - `FeeService`: Handles fee charging, balance management, and refunds

3. **Fee Guards**
   - `FeeGuard`: Validates user balance before operations
   - Decorators for specifying fee types and parameters

4. **Fee Controller**
   - REST API endpoints for balance management and fee operations

## Fee Structure

### Base Fees

| Operation Type | Base Fee (credits) | Description |
|---------------|-------------------|-------------|
| Scan | 100 | Basic security scan |
| API Call | 1 | Standard API request |
| Storage | 10 | Data storage operation |
| Premium Feature | 500 | Advanced analysis features |

### Fee Multipliers

#### Scan Operations
- **Code Size**:
  - ≤ 50KB: 1.0x
  - 50-100KB: 1.5x
  - > 100KB: 2.0x
- **Complexity**:
  - ≤ 5: 1.0x
  - 6-8: 1.3x
  - > 8: 1.8x
- **Processing Time**:
  - ≤ 2 minutes: 1.0x
  - 2-5 minutes: 1.2x
  - > 5 minutes: 1.5x

#### API Call Operations
- **CPU Usage** > 80%: 1.5x
- **Memory Usage** > 512MB: 1.3x

#### Storage Operations
- **Size** ≤ 10MB: 1.0x
- **Size** 10-100MB: 1.5x
- **Size** > 100MB: 2.0x

#### Premium Features
- **Complexity-based**: Base fee × (complexity / 5)

## API Endpoints

### Fee Management

#### Estimate Fee
```http
POST /api/fee/estimate
Content-Type: application/json

{
  "type": "scan",
  "codeSize": 50000,
  "complexity": 7
}
```

#### Get User Balance
```http
GET /api/fee/balance
Authorization: Bearer <token>
```

#### Add Balance
```http
POST /api/fee/balance/add
Content-Type: application/json
Authorization: Bearer <token>

{
  "amount": 1000,
  "description": "Balance deposit",
  "transactionId": "txn_123456"
}
```

#### Get Fee History
```http
GET /api/fee/history?page=1&limit=10
Authorization: Bearer <token>
```

#### Check Affordability
```http
POST /api/fee/can-afford
Content-Type: application/json
Authorization: Bearer <token>

{
  "type": "scan",
  "codeSize": 50000
}
```

#### Refund Fee
```http
POST /api/fee/refund/{feeId}
Content-Type: application/json

{
  "reason": "Customer request"
}
```

## Integration Examples

### Adding Fees to Controllers

```typescript
import { FeeGuard, SetFeeType, SetFeeParams } from '../fee/guards/fee.guard';

@Controller('scan')
export class ScanController {
  @Post()
  @UseGuards(FeeGuard)
  @SetFeeType('scan')
  @SetFeeParams((req) => ({
    codeSize: req.body.code?.length || 0,
    complexity: req.body.options?.complexity || 1,
  }))
  async createScan(@Body() createScanDto: CreateScanDto, @Request() req: any) {
    // Fee is automatically validated and charged
    const scan = await this.scanService.createScan(createScanDto, userId);
    
    // Fee information is available in req.feeInfo
    if (req.feeInfo) {
      await this.scanService.chargeScanFee(scan.id, userId, req.feeInfo);
    }
    
    return scan;
  }
}
```

### Manual Fee Charging

```typescript
import { FeeService } from '../fee/services/fee.service';

@Injectable()
export class CustomService {
  constructor(private feeService: FeeService) {}

  async performOperation(userId: string) {
    // Check if user can afford
    const canAfford = await this.feeService.canAffordOperation(userId, {
      type: 'premium_feature',
      complexity: 8,
    });

    if (!canAfford) {
      throw new Error('Insufficient balance');
    }

    // Charge the fee
    await this.feeService.createAndChargeFee({
      type: 'premium_feature',
      amount: 800, // Optional - will be calculated if not provided
      description: 'Advanced vulnerability analysis',
    }, userId);

    // Perform the operation
    return await this.executeOperation();
  }
}
```

## Configuration

### Environment Variables

```bash
# Base Fees
FEES_BASE_SCAN_FEE=100
FEES_BASE_API_CALL_FEE=1
FEES_BASE_STORAGE_FEE=10
FEES_BASE_PREMIUM_FEE=500

# User Limits
FEES_DEFAULT_CREDIT_LIMIT=1000
FEES_ENABLE_FREE_TIER=true
FEES_FREE_TIER_MONTHLY_LIMIT=1000
FEES_FREE_TIER_SCAN_LIMIT=10
```

### Database Schema

The fee mechanism adds the following tables:

#### fees
- `id` (UUID, Primary Key)
- `user_id` (UUID, Foreign Key)
- `scan_id` (UUID, Optional)
- `type` (ENUM: scan, api_call, storage, premium_feature)
- `amount` (INTEGER)
- `description` (TEXT, Optional)
- `status` (ENUM: pending, paid, failed, refunded)
- `metadata` (JSONB, Optional)
- `paid_at` (TIMESTAMP, Optional)
- `refunded_at` (TIMESTAMP, Optional)
- `transaction_id` (TEXT, Optional)
- `refund_reason` (TEXT, Optional)
- `created_at` (TIMESTAMP)
- `updated_at` (TIMESTAMP)

#### user_balances
- `id` (UUID, Primary Key)
- `user_id` (UUID, Unique, Foreign Key)
- `balance` (INTEGER, Default: 0)
- `total_spent` (INTEGER, Default: 0)
- `total_deposited` (INTEGER, Default: 0)
- `total_refunded` (INTEGER, Default: 0)
- `credit_limit` (INTEGER, Default: 1000)
- `last_fee_deducted_at` (TIMESTAMP, Optional)
- `usage_stats` (JSONB, Optional)
- `created_at` (TIMESTAMP)
- `updated_at` (TIMESTAMP)

## Security Considerations

### Access Control
- All fee operations require user authentication
- Users can only view their own balance and fee history
- Admin users can view global fee statistics

### Data Protection
- Fee amounts and balance information are sensitive
- All fee transactions are logged for audit purposes
- Database encryption should be enabled for fee tables

### Rate Limiting
- Fee operations are subject to rate limiting
- Multiple fee attempts should be monitored for abuse
- Credit limits prevent excessive spending

## Monitoring and Analytics

### Fee Metrics
- Total revenue per operation type
- Average fee per operation
- User spending patterns
- Refund rates and reasons

### Usage Statistics
- Daily/weekly/monthly fee volume
- Most expensive operations
- User balance distribution
- Credit limit utilization

## Testing

### Unit Tests
- Fee calculation logic
- Balance management
- Fee validation and charging
- Refund processing

### Integration Tests
- API endpoint functionality
- Database operations
- Fee guard behavior
- Error handling

### Test Coverage
- All fee calculation scenarios
- Edge cases (zero balance, insufficient funds)
- Refund workflows
- Configuration variations

## Migration Guide

### Existing Users
1. Create user balance records with initial balance
2. Set credit limits based on user tier
3. Migrate existing usage statistics

### Gradual Rollout
1. Enable fee calculation in read-only mode
2. Monitor fee estimates without charging
3. Gradually enable fee charging for new operations
4. Apply fees to existing operations if needed

## Troubleshooting

### Common Issues

#### Insufficient Balance Error
```typescript
// Check user balance
const balance = await feeService.getUserBalance(userId);
console.log('Current balance:', balance?.balance);

// Get estimated fee
const estimate = feeCalculator.getEstimatedFee(params);
console.log('Estimated fee:', estimate);
```

#### Fee Calculation Issues
```typescript
// Verify configuration
console.log('Base scan fee:', configService.get('FEES_BASE_SCAN_FEE'));

// Check calculation parameters
const result = feeCalculator.getEstimatedFee(params);
console.log('Fee breakdown:', result.breakdown);
```

#### Database Issues
- Verify fee and balance tables exist
- Check foreign key relationships
- Ensure proper indexing on user_id columns

## Future Enhancements

### Planned Features
- Subscription-based pricing
- Volume discounts
- Tiered user plans
- Advanced analytics dashboard
- Automated refund policies

### Performance Optimizations
- Fee calculation caching
- Balance update batching
- Database query optimization
- Real-time balance updates

## Support

For questions or issues related to the fee mechanism:
1. Check the troubleshooting section
2. Review the test cases for expected behavior
3. Consult the API documentation
4. Contact the development team
