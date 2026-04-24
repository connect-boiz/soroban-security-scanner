import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn, UpdateDateColumn, OneToMany, ManyToOne } from 'typeorm';
import { Fee } from './fee.entity';
import { User } from '../../user/entities/user.entity';

@Entity('user_balances')
export class UserBalance {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'user_id', unique: true })
  userId: string;

  @Column('integer', { default: 0 })
  balance: number;

  @Column('integer', { default: 0 })
  totalSpent: number;

  @Column('integer', { default: 0 })
  totalDeposited: number;

  @Column('integer', { default: 0 })
  totalRefunded: number;

  @Column('integer', { default: 0 })
  creditLimit: number;

  @Column('timestamp', { nullable: true })
  lastFeeDeductedAt: Date;

  @Column('jsonb', { nullable: true })
  usageStats: {
    totalScans: number;
    totalApiCalls: number;
    totalStorageUsed: number;
    lastResetDate: string;
  };

  @OneToMany(() => Fee, fee => fee.user)
  fees: Fee[];

  @ManyToOne(() => User, user => user.balances)
  user: User;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
