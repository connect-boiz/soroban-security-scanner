import { Entity, PrimaryGeneratedColumn, Column, CreateDateColumn, UpdateDateColumn, OneToMany } from 'typeorm';
import { Vulnerability } from './vulnerability.entity';

@Entity('scans')
export class Scan {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column({ name: 'user_id' })
  userId: string;

  @Column('text')
  code: string;

  @Column('jsonb', { nullable: true })
  options: Record<string, any>;

  @Column({
    type: 'enum',
    enum: ['pending', 'running', 'completed', 'failed'],
    default: 'pending',
  })
  status: 'pending' | 'running' | 'completed' | 'failed';

  @Column('jsonb', { nullable: true })
  metrics: {
    totalVulnerabilities: number;
    criticalCount: number;
    highCount: number;
    mediumCount: number;
    lowCount: number;
    riskScore: number;
    linesOfCode: number;
  };

  @Column('integer', { nullable: true })
  scanTime: number;

  @Column('text', { nullable: true })
  errorMessage: string;

  @OneToMany(() => Vulnerability, vulnerability => vulnerability.scan, { cascade: true })
  vulnerabilities: Vulnerability[];

  @CreateDateColumn({ name: 'created_at' })
  createdAt: Date;

  @UpdateDateColumn({ name: 'updated_at' })
  updatedAt: Date;
}
