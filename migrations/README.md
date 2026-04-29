# Database Migrations

This directory contains SQL migration files for the Soroban Security Scanner database schema.

## Migration Files

### 001_initial_schema.sql
**Description**: Creates the core database schema including:
- Users table with authentication and security fields
- Wallets table for Stellar wallet management
- Transactions table for comprehensive transaction tracking
- Multi-signature operations tables
- User sessions and audit logging
- Basic security monitoring tables

**Features**:
- Proper foreign key relationships
- Comprehensive indexing strategy
- Enum types for data consistency
- Triggers for automatic timestamp updates
- Views for common queries
- Constraints for data integrity

### 002_add_security_features.sql
**Description**: Adds enhanced security features:
- Security alerts system
- Rate limiting functionality
- Device tracking and fingerprinting
- Access pattern analysis
- Enhanced user security fields
- Risk assessment capabilities

**Features**:
- Security alert management
- Rate limiting with blocking capabilities
- Device trust management
- Behavioral analysis
- Risk scoring algorithms
- Security monitoring procedures

### 003_add_bounty_system.sql
**Description**: Implements bounty marketplace and escrow system:
- Projects for organizing bounties
- Comprehensive bounty management
- Application and submission tracking
- Escrow account management
- Review and payment processing
- Activity logging

**Features**:
- Full bounty lifecycle management
- Secure escrow system
- Multi-stage review process
- Payment tracking
- Analytics and reporting
- Tag-based categorization

### 004_add_analytics_and_reporting.sql
**Description**: Adds analytics and reporting capabilities:
- Raw analytics event collection
- Aggregated metrics storage
- User activity summaries
- Platform metrics tracking
- Performance monitoring
- Report generation system

**Features**:
- Event-driven analytics
- Time-series metrics
- Automated aggregation
- Custom report generation
- Performance monitoring
- Health check procedures

## Running Migrations

### Using SQLx CLI

```bash
# Install SQLx CLI
cargo install sqlx-cli

# Run all pending migrations
sqlx migrate run --database-url "postgresql://user:password@localhost:5432/soroban_security_scanner"

# Check migration status
sqlx migrate info --database-url "postgresql://user:password@localhost:5432/soroban_security_scanner"

# Revert last migration
sqlx migrate revert --database-url "postgresql://user:password@localhost:5432/soroban_security_scanner"
```

### Using Rust Code

```rust
use soroban_security_scanner::database::{Database, DatabaseConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load database configuration
    let config = DatabaseConfig::from_env()?;
    
    // Create database connection
    let database = Database::new(config).await?;
    
    // Run migrations
    database.run_migrations().await?;
    
    println!("Migrations completed successfully!");
    Ok(())
}
```

### Environment Variables

Required environment variables for database connection:

```bash
DATABASE_HOST=localhost
DATABASE_PORT=5432
DATABASE_NAME=soroban_security_scanner
DATABASE_USER=postgres
DATABASE_PASSWORD=your_password
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5
DATABASE_CONNECT_TIMEOUT=30
DATABASE_IDLE_TIMEOUT=600
DATABASE_MAX_LIFETIME=1800
```

## Migration Best Practices

### Before Running Migrations

1. **Backup Database**: Always create a backup before running migrations
2. **Test on Staging**: Test migrations on a staging environment first
3. **Review Changes**: Carefully review migration SQL before execution
4. **Check Dependencies**: Ensure all dependencies are satisfied

### During Migration

1. **Monitor Progress**: Monitor migration execution progress
2. **Check Logs**: Review any error messages carefully
3. **Verify Results**: Verify that changes were applied correctly
4. **Test Functionality**: Test application functionality after migration

### After Migration

1. **Update Documentation**: Update any relevant documentation
2. **Monitor Performance**: Monitor database performance
3. **Check Indexes**: Verify indexes are being used properly
4. **Validate Data**: Validate data integrity

## Creating New Migrations

### Naming Convention

Use the following naming convention for migration files:
```
{number}_{description}.sql
```

Where:
- `{number}`: Sequential migration number (e.g., 005)
- `{description}`: Short, descriptive name in snake_case

### Migration Template

```sql
-- Migration {number}: {description}
-- {detailed description of what this migration does}

-- Add any new types
-- CREATE TYPE new_type AS ENUM (...);

-- Create new tables
-- CREATE TABLE new_table (
--     id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
--     -- other columns...
--     created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
--     updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
-- );

-- Add new indexes
-- CREATE INDEX idx_new_table_column ON new_table(column);

-- Add constraints
-- ALTER TABLE new_table ADD CONSTRAINT check_positive_value CHECK (value >= 0);

-- Create views if needed
-- CREATE VIEW new_view AS SELECT * FROM new_table WHERE condition;

-- Create stored procedures if needed
-- CREATE OR REPLACE FUNCTION new_function() RETURNS VOID AS $$
-- BEGIN
--     -- function body
-- END;
-- $$ LANGUAGE plpgsql;

-- Apply triggers for timestamp updates
-- CREATE TRIGGER update_new_table_updated_at BEFORE UPDATE ON new_table FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
```

### Migration Guidelines

1. **Idempotent**: Migrations should be safe to run multiple times
2. **Backwards Compatible**: Consider backwards compatibility where possible
3. **Performance**: Consider performance impact of large operations
4. **Rollback**: Always include rollback procedures
5. **Testing**: Test migrations with realistic data volumes

## Rollback Procedures

### Manual Rollback

For each migration, create a corresponding rollback file:

```sql
-- Rollback {number}: {description}
-- Rollback script for migration {number}

-- Drop tables in reverse order
DROP TABLE IF EXISTS table_name;

-- Drop types
DROP TYPE IF EXISTS type_name;

-- Drop indexes
DROP INDEX IF EXISTS index_name;

-- Drop views
DROP VIEW IF EXISTS view_name;

-- Drop functions
DROP FUNCTION IF EXISTS function_name();
```

### Automated Rollback

```bash
# Rollback last migration
sqlx migrate revert --database-url "postgresql://user:password@localhost:5432/soroban_security_scanner"

# Rollback to specific version
sqlx migrate revert --database-url "postgresql://user:password@localhost:5432/soroban_security_scanner" --target-version 002
```

## Troubleshooting

### Common Issues

1. **Connection Errors**: Check database connection parameters
2. **Permission Errors**: Ensure database user has required permissions
3. **Lock Conflicts**: Check for long-running transactions
4. **Disk Space**: Ensure sufficient disk space for migrations
5. **Dependencies**: Verify all dependencies are installed

### Error Resolution

1. **Check Logs**: Review detailed error messages
2. **Verify State**: Check current migration state
3. **Manual Cleanup**: Clean up partial migrations if needed
4. **Restore Backup**: Restore from backup if necessary
5. **Re-run Migration**: Re-run migration after fixing issues

## Performance Considerations

### Large Tables

For large tables, consider:

1. **Batch Processing**: Process data in batches
2. **Index Management**: Create/drop indexes strategically
3. **Lock Minimization**: Minimize table locks
4. **Connection Pooling**: Use appropriate connection pooling
5. **Monitoring**: Monitor system resources during migration

### Index Creation

For large indexes:

1. **Create Concurrently**: Use `CREATE INDEX CONCURRENTLY`
2. **Monitor Progress**: Monitor index creation progress
3. **Disk Space**: Ensure sufficient disk space
4. **Performance**: Monitor impact on query performance

## Security Considerations

### Migration Security

1. **Access Control**: Limit migration execution privileges
2. **Audit Logging**: Log all migration activities
3. **Validation**: Validate migration SQL for security issues
4. **Encryption**: Consider encryption for sensitive data

### Data Protection

1. **PII Protection**: Protect personally identifiable information
2. **Backup Security**: Secure backup files
3. **Access Logs**: Maintain access logs
4. **Data Masking**: Use data masking in development

## Monitoring and Maintenance

### Migration Monitoring

1. **Execution Time**: Monitor migration execution times
2. **Resource Usage**: Monitor system resource usage
3. **Error Rates**: Track migration error rates
4. **Success Rates**: Monitor migration success rates

### Regular Maintenance

1. **Cleanup**: Clean up old migration files
2. **Documentation**: Keep documentation up to date
3. **Testing**: Regularly test migration procedures
4. **Review**: Review migration strategies periodically

## Development Workflow

### Local Development

1. **Setup**: Set up local database for development
2. **Test Migrations**: Test migrations locally first
3. **Data Seeding**: Use seed data for testing
4. **Validation**: Validate schema changes

### Team Collaboration

1. **Code Review**: Review migration SQL in pull requests
2. **Documentation**: Document schema changes
3. **Communication**: Communicate schema changes to team
4. **Planning**: Plan migrations in team meetings

This migration system provides a robust foundation for managing database schema evolution while maintaining data integrity and system stability.
