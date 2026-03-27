import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn } from 'typeorm';

@Entity('notification_queue')
export class NotificationQueue {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column('uuid')
  webhookId: string;

  @Column('uuid')
  scanId: string;

  @Column({ name: 'user_id' })
  userId: string;

  @Column({
    type: 'enum',
    enum: ['pending', 'sent', 'failed', 'deduplicated'],
    default: 'pending',
  })
  status: 'pending' | 'sent' | 'failed' | 'deduplicated';

  @Column('text')
  payload: string;

  @Column('text', { nullable: true })
  errorMessage: string;

  @Column('integer', { default: 0 })
  attempts: number;

  @Column('timestamp', { nullable: true })
  nextAttemptAt: Date;

  @Column('timestamp', { nullable: true })
  sentAt: Date;

  @Column('text', { nullable: true })
  deduplicationKey: string;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;
}
