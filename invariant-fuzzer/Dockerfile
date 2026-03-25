# Stage 1: Build binary using musl-builder for Alpine compatibility
FROM rust:1.75-alpine as builder

# Install build dependencies
RUN apk add --no-cache musl-dev gcc pkgconfig openssl-dev openssl-libs-static perl make

WORKDIR /usr/src/invariant-fuzzer

# Copy Cargo.toml and Cargo.lock first to cache dependencies
COPY Cargo.toml ./
# Create dummy src/main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && touch src/executor.rs src/ledger.rs src/fuzzer.rs
RUN cargo build --release

# Copy the actual source files
COPY src ./src/

# Re-build with actual source
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime image using minimal Alpine Linux
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache libgcc ca-certificates openssl

# Set working directory
WORKDIR /app

# Copy the binary from the builder
COPY --from=builder /usr/src/invariant-fuzzer/target/release/soroban-invariant-fuzzer /app/soroban-invariant-fuzzer

# Expose port
EXPOSE 8081

# Run the service
CMD ["/app/soroban-invariant-fuzzer"]
