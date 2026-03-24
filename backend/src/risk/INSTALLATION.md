# Risk Management System Installation Guide

## Prerequisites

### System Requirements
- **Node.js**: >= 18.0.0
- **PostgreSQL**: >= 13.0
- **Redis**: >= 6.0
- **Memory**: Minimum 4GB RAM (8GB recommended)
- **Storage**: Minimum 100GB available space
- **CPU**: Minimum 4 cores (8 cores recommended)

### Network Requirements
- **Port 3000**: Application server
- **Port 5432**: PostgreSQL database
- **Port 6379**: Redis server
- **Port 9229**: Debugging (optional)

### Development Tools
- **Git**: Version control
- **Docker**: Containerization (optional)
- **Docker Compose**: Multi-container orchestration (optional)

## Installation Steps

### 1. Repository Setup

```bash
# Clone the repository
git clone https://github.com/connect-boiz/soroban-security-scanner.git
cd soroban-security-scanner

# Switch to risk management branch
git checkout risk-management-system

# Navigate to backend directory
cd backend
```

### 2. Dependencies Installation

```bash
# Install Node.js dependencies
npm install

# Verify installation
npm run type-check
```

### 3. Database Setup

#### PostgreSQL Installation

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
sudo systemctl enable postgresql
```

**macOS:**
```bash
brew install postgresql
brew services start postgresql
```

**Windows:**
```bash
# Download and install from https://www.postgresql.org/download/windows/
```

#### Database Configuration

```bash
# Create database user
sudo -u postgres createuser --interactive risk_user

# Create database
sudo -u postgres createdb risk_management_db

# Set password
sudo -u postgres psql
ALTER USER risk_user PASSWORD 'your_secure_password';
GRANT ALL PRIVILEGES ON DATABASE risk_management_db TO risk_user;
\q
```

### 4. Redis Setup

**Ubuntu/Debian:**
```bash
sudo apt install redis-server
sudo systemctl start redis-server
sudo systemctl enable redis-server
```

**macOS:**
```bash
brew install redis
brew services start redis
```

**Windows:**
```bash
# Download and install from https://redis.io/download
```

### 5. Environment Configuration

Create `.env` file in the backend directory:

```bash
cp .env.example .env
```

Edit `.env` with your configuration:

```env
# Database Configuration
DATABASE_URL=postgresql://risk_user:your_secure_password@localhost:5432/risk_management_db
DATABASE_SSL=false
DATABASE_SYNCHRONIZE=false

# Redis Configuration
REDIS_URL=redis://localhost:6379
REDIS_KEY_PREFIX=risk_management:

# JWT Configuration
JWT_SECRET=your_super_secret_jwt_key_here
JWT_EXPIRES_IN=7d

# Risk Management Configuration
RISK_MONITORING_INTERVAL=10000
RISK_ALERT_THRESHOLDS_VAR=100000
RISK_ALERT_THRESHOLDS_VOLATILITY=0.04
RISK_STRESS_TEST_SCENARIOS=50
RISK_VAR_CONFIDENCE_LEVEL=0.95
RISK_HEDGING_COST_LIMIT=0.02

# Application Configuration
NODE_ENV=development
PORT=3000
API_PREFIX=api/v1

# Logging Configuration
LOG_LEVEL=info
LOG_FORMAT=json

# External API Configuration
BLOOMBERG_API_KEY=your_bloomberg_api_key
REUTERS_API_KEY=your_reuters_api_key
RISKMETRICS_API_KEY=your_riskmetrics_api_key
```

### 6. Database Migration

```bash
# Run database migrations
npm run migration:run

# (Optional) Seed database with initial data
npm run seed:run
```

### 7. Build and Start Application

```bash
# Build the application
npm run build

# Start development server
npm run start:dev

# Or start production server
npm run start:prod
```

## Docker Installation (Alternative)

### 1. Docker Compose Setup

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=production
      - DATABASE_URL=postgresql://postgres:password@postgres:5432/risk_management_db
      - REDIS_URL=redis://redis:6379
    depends_on:
      - postgres
      - redis
    volumes:
      - ./logs:/app/logs

  postgres:
    image: postgres:13
    environment:
      - POSTGRES_DB=risk_management_db
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  redis:
    image: redis:6-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data

volumes:
  postgres_data:
  redis_data:
```

### 2. Docker Commands

```bash
# Build and start all services
docker-compose up --build

# Run in background
docker-compose up -d

# View logs
docker-compose logs -f app

# Stop services
docker-compose down
```

## Verification

### 1. Health Check

```bash
# Check application health
curl http://localhost:3000/health

# Expected response:
# {"status":"ok","timestamp":"2026-03-24T...","uptime":...}
```

### 2. API Test

```bash
# Test risk assessment endpoint
curl -X POST http://localhost:3000/api/v1/risk/assess \
  -H "Content-Type: application/json" \
  -d '{
    "userId": "test_user",
    "portfolio": {
      "id": "test_portfolio",
      "positions": [
        {
          "id": "position_1",
          "type": "stock",
          "size": 100,
          "currentPrice": 50
        }
      ],
      "totalValue": 5000
    }
  }'
```

### 3. Database Connection Test

```bash
# Test PostgreSQL connection
psql -h localhost -U risk_user -d risk_management_db -c "SELECT 1;"

# Test Redis connection
redis-cli ping
# Expected response: PONG
```

## Configuration Guide

### Risk Thresholds

Configure risk alert thresholds in your environment:

```env
# VaR threshold in USD
RISK_ALERT_THRESHOLDS_VAR=100000

# Volatility threshold (decimal)
RISK_ALERT_THRESHOLDS_VOLATILITY=0.04

# Concentration threshold (decimal)
RISK_ALERT_THRESHOLDS_CONCENTRATION=0.25
```

### Monitoring Settings

Adjust monitoring frequency and performance:

```env
# Monitoring interval in milliseconds
RISK_MONITORING_INTERVAL=10000

# Number of stress test scenarios
RISK_STRESS_TEST_SCENARIOS=50

# VaR confidence level (0.90, 0.95, 0.99)
RISK_VAR_CONFIDENCE_LEVEL=0.95
```

### External API Keys

Configure external data providers:

```env
# Market data providers
BLOOMBERG_API_KEY=your_bloomberg_api_key
REUTERS_API_KEY=your_reuters_api_key
RISKMETRICS_API_KEY=your_riskmetrics_api_key

# Notification services
SLACK_WEBHOOK_URL=your_slack_webhook_url
EMAIL_SERVICE_API_KEY=your_email_api_key
```

## Performance Tuning

### Database Optimization

```sql
-- Create indexes for performance
CREATE INDEX CONCURRENTLY idx_risk_data_portfolio_timestamp 
ON risk_data(portfolio_id, timestamp);

CREATE INDEX CONCURRENTLY idx_risk_data_user_risk_type 
ON risk_data(user_id, risk_type);

-- Partition large tables (optional)
CREATE TABLE risk_data_y2024 PARTITION OF risk_data
FOR VALUES FROM ('2024-01-01') TO ('2025-01-01');
```

### Redis Configuration

```bash
# Edit redis.conf
maxmemory 2gb
maxmemory-policy allkeys-lru
save 900 1
save 300 10
save 60 10000
```

### Application Performance

```env
# Node.js performance
NODE_OPTIONS=--max-old-space-size=4096

# Connection pooling
DATABASE_POOL_SIZE=20
DATABASE_POOL_TIMEOUT=30000

# Cache settings
CACHE_TTL_ASSESSMENT=300
CACHE_TTL_METRICS=30
CACHE_TTL_ALERTS=86400
```

## Troubleshooting

### Common Issues

#### 1. Database Connection Failed
```bash
# Check PostgreSQL status
sudo systemctl status postgresql

# Check connection
psql -h localhost -U risk_user -d risk_management_db

# Reset password if needed
sudo -u postgres psql
ALTER USER risk_user PASSWORD 'new_password';
```

#### 2. Redis Connection Failed
```bash
# Check Redis status
sudo systemctl status redis

# Test connection
redis-cli ping

# Restart Redis
sudo systemctl restart redis
```

#### 3. Application Won't Start
```bash
# Check Node.js version
node --version  # Should be >= 18.0.0

# Check for port conflicts
netstat -tulpn | grep :3000

# Check logs
npm run start:dev --verbose
```

#### 4. Permission Errors
```bash
# Fix file permissions
chmod +x scripts/*.sh

# Fix database permissions
sudo -u postgres psql
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO risk_user;
```

### Debug Mode

```bash
# Start with debugging
npm run start:debug

# Or with inspect flag
node --inspect-brk dist/main.js
```

### Log Analysis

```bash
# View application logs
tail -f logs/app.log

# View error logs
grep ERROR logs/app.log

# Monitor performance
grep "Performance" logs/app.log
```

## Maintenance

### Regular Tasks

#### Daily
- Monitor application health
- Check error rates
- Review risk alerts

#### Weekly
- Database backup
- Log rotation
- Performance review

#### Monthly
- Security updates
- Dependency updates
- Capacity planning

### Backup Scripts

```bash
#!/bin/bash
# backup.sh

# Database backup
pg_dump -h localhost -U risk_user risk_management_db > backup_$(date +%Y%m%d).sql

# Redis backup
redis-cli BGSAVE
cp /var/lib/redis/dump.rdb backup_redis_$(date +%Y%m%d).rdb

# Upload to cloud storage (optional)
aws s3 cp backup_$(date +%Y%m%d).sql s3://your-backup-bucket/
```

### Update Process

```bash
# Update dependencies
npm update

# Run tests
npm test

# Build application
npm run build

# Restart service
sudo systemctl restart risk-management
```

## Support

### Documentation
- [API Documentation](http://localhost:3000/docs)
- [Architecture Guide](./ARCHITECTURE.md)
- [User Manual](./README.md)

### Community
- GitHub Issues: https://github.com/connect-boiz/soroban-security-scanner/issues
- Discussion Forum: https://github.com/connect-boiz/soroban-security-scanner/discussions

### Contact
- Technical Support: risk-support@company.com
- Emergency Contact: 24/7 hotline

---

**Version**: 1.0.0  
**Last Updated**: 2026-03-24  
**Installation Guide**: Risk Management Team
