# Multi-stage build for Stellar Security Scanner
FROM rust:1.75 as builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source code
COPY . .

# Build the scanner
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 scanner

# Copy binary from builder
COPY --from=builder /app/target/release/stellar-scanner /usr/local/bin/stellar-scanner

# Create scan directory
RUN mkdir -p /scan && chown scanner:scanner /scan

USER scanner

WORKDIR /scan

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD stellar-scanner --version || exit 1

ENTRYPOINT ["/usr/local/bin/stellar-scanner"]
