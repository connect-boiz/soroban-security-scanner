FROM rust:1.75 as builder

WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create dummy lib.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/lib.rs

# Build dependencies
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src

# Build the library
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

WORKDIR /app

# Copy library from builder
COPY --from=builder /app/target/release/lib*.so ./

# Change ownership to appuser
RUN chown appuser:appuser *.so

USER appuser

EXPOSE 8081

# We'll create a simple server to expose the scanner as a service
COPY --from=builder /app/target/release/deps/lib*.so ./
CMD ["sh", "-c", "echo 'Core scanner service running' && sleep infinity"]
