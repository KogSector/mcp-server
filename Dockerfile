# =============================================================================
# MCP Service - Dockerfile
# Port: 3004
# Role: Model Context Protocol server for AI agent connections
# =============================================================================
# Build from workspace root: podman build -f mcp-server/Dockerfile -t confuse/mcp-server .
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

WORKDIR /workspace

# Copy shared middleware library first
COPY shared-middleware-confuse ./shared-middleware-confuse

# Copy mcp-server
COPY mcp-server ./mcp-server

WORKDIR /workspace/mcp-server

# Build the application
RUN cargo build --release

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
COPY --from=builder /workspace/mcp-server/target/release/mcp-service /app/mcp-service

# Set ownership
RUN chown -R conhub:conhub /app

USER conhub

EXPOSE 3004

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3004/health || exit 1

CMD ["/app/mcp-service"]
