# =============================================================================
# MCP Service - Dockerfile
# Port: 3004
# Role: Model Context Protocol server for AI agent connections
# =============================================================================

# Multi-stage build for MCP Service (Rust 1.84)
FROM rust:1.84-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    curl \
    cmake \
    build-essential \
    libsasl2-dev \
    librdkafka-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true

# Copy actual source files
COPY src ./src

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    librdkafka1 \
    libsasl2-2 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1001 conhub

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/mcp-service /app/mcp-service

# Set ownership
RUN chown -R conhub:conhub /app

USER conhub

EXPOSE 3004

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3004/health || exit 1

CMD ["/app/mcp-service"]
