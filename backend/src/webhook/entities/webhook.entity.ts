import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn, UpdateDateColumn } from 'typeorm';

@Entity('webhooks')
export class Webhook {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'user_id' })
  userId: string;

  @Column()
  name: string;

  @Column()
  url: string;

  @Column({
    type: 'enum',
    enum: ['slack', 'discord', 'custom'],
    default: 'custom',
  })
  type: 'slack' | 'discord' | 'custom';

  @Column('jsonb', { nullable: true })
  config: {
    secret?: string;
    channel?: string;
    username?: string;
    icon_url?: string;
  };

  @Column({
    type: 'enum',
    enum: ['all', 'critical', 'high', 'medium', 'low'],
    default: 'all',
  })
  severityFilter: 'all' | 'critical' | 'high' | 'medium' | 'low';

  @Column({ default: true })
  enabled: boolean;

  @Column('jsonb', { nullable: true })
  lastSent: {
    timestamp: Date;
    scanId: string;
    status: string;
  };

  @Column('integer', { default: 0 })
  successCount: number;

  @Column('integer', { default: 0 })
  failureCount: number;

  @Column('text', { nullable: true })
  lastError: string;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
