import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn, UpdateDateColumn, ManyToOne } from 'typeorm';
import { User } from '../../user/entities/user.entity';

@Entity('fees')
export class Fee {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'user_id' })
  userId: string;

  @Column({ name: 'scan_id', nullable: true })
  scanId: string;

  @Column({
    type: 'enum',
    enum: ['scan', 'api_call', 'storage', 'premium_feature'],
  })
  type: 'scan' | 'api_call' | 'storage' | 'premium_feature';

  @Column('integer')
  amount: number;

  @Column('text', { nullable: true })
  description: string;

  @Column({
    type: 'enum',
    enum: ['pending', 'paid', 'failed', 'refunded'],
    default: 'pending',
  })
  status: 'pending' | 'paid' | 'failed' | 'refunded';

  @Column('jsonb', { nullable: true })
  metadata: {
    scanComplexity?: number;
    codeSize?: number;
    processingTime?: number;
    resourceUsage?: any;
  };

  @Column('timestamp', { nullable: true })
  paidAt: Date;

  @Column('timestamp', { nullable: true })
  refundedAt: Date;

  @Column('text', { nullable: true })
  transactionId: string;

  @Column('text', { nullable: true })
  refundReason: string;

  @ManyToOne(() => User, user => user.fees, { onDelete: 'CASCADE' })
  user: User;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
