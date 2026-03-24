import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn, UpdateDateColumn, Index } from 'typeorm';

@Entity('risk_data')
@Index(['portfolioId', 'timestamp'])
@Index(['userId', 'riskType'])
export class RiskData {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'user_id' })
  userId: string;

  @Column({ name: 'portfolio_id' })
  portfolioId: string;

  @Column({
    type: 'enum',
    enum: ['market', 'credit', 'operational', 'liquidity', 'counterparty'],
  })
  riskType: 'market' | 'credit' | 'operational' | 'liquidity' | 'counterparty';

  @Column('decimal', { precision: 10, scale: 4 })
  riskScore: number;

  @Column('decimal', { precision: 15, scale: 2, nullable: true })
  exposure: number;

  @Column('decimal', { precision: 10, scale: 4, nullable: true })
  probability: number;

  @Column('decimal', { precision: 15, scale: 2, nullable: true })
  potentialLoss: number;

  @Column({
    type: 'enum',
    enum: ['low', 'medium', 'high', 'critical'],
    default: 'medium',
  })
  severity: 'low' | 'medium' | 'high' | 'critical';

  @Column('jsonb', { nullable: true })
  metrics: {
    var1d: number;
    var10d: number;
    var30d: number;
    expectedShortfall: number;
    beta: number;
    volatility: number;
    correlation: number;
    concentration: number;
  };

  @Column('jsonb', { nullable: true })
  marketFactors: {
    price: number;
    volume: number;
    volatility: number;
    interestRate: number;
    exchangeRate: number;
  };

  @Column('jsonb', { nullable: true })
  hedgingInfo: {
    strategy: string;
    hedgeRatio: number;
    effectiveness: number;
    cost: number;
  };

  @Column('text', { nullable: true })
  description: string;

  @Column('text', { nullable: true })
  mitigation: string;

  @Column({
    type: 'enum',
    enum: ['active', 'mitigated', 'monitored', 'closed'],
    default: 'active',
  })
  status: 'active' | 'mitigated' | 'monitored' | 'closed';

  @Column('timestamp', { name: 'timestamp' })
  timestamp: Date;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
