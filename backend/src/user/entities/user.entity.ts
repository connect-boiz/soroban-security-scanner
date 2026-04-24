import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn, UpdateDateColumn, OneToMany } from 'typeorm';
import { ApiKey } from '../../api-key/entities/api-key.entity';
import { Fee } from '../../fee/entities/fee.entity';
import { UserBalance } from '../../fee/entities/user-balance.entity';

@Entity('users')
export class User {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column('text', { unique: true })
  email: string;

  @Column('text')
  passwordHash: string;

  @Column('text', { nullable: true })
  firstName: string;

  @Column('text', { nullable: true })
  lastName: string;

  @Column({
    type: 'enum',
    enum: ['admin', 'developer', 'viewer'],
    default: 'developer',
  })
  role: 'admin' | 'developer' | 'viewer';

  @Column('boolean', { default: true })
  isActive: boolean;

  @Column('timestamp', { nullable: true })
  lastLoginAt: Date;

  @OneToMany(() => ApiKey, apiKey => apiKey.user, { cascade: true })
  apiKeys: ApiKey[];

  @OneToMany(() => Fee, fee => fee.user, { cascade: true })
  fees: Fee[];

  @OneToMany(() => UserBalance, balance => balance.user, { cascade: true })
  balances: UserBalance[];

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
