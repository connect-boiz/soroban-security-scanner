import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn, UpdateDateColumn, ManyToOne } from 'typeorm';
import { User } from '../../user/entities/user.entity';

@Entity('api_keys')
export class ApiKey {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'user_id' })
  userId: string;

  @Column('text')
  keyHash: string;

  @Column('text')
  keyPrefix: string;

  @Column('text')
  name: string;

  @Column({
    type: 'enum',
    enum: ['active', 'revoked'],
    default: 'active',
  })
  status: 'active' | 'revoked';

  @Column('timestamp', { nullable: true })
  lastUsedAt: Date;

  @Column('timestamp', { nullable: true })
  expiresAt: Date;

  @Column('jsonb', { nullable: true })
  permissions: string[];

  @Column('text', { nullable: true })
  description: string;

  @ManyToOne(() => User, user => user.apiKeys)
  user: User;

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
