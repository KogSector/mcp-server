# Multi-stage build for MCP Service
FROM rust:1.75-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy shared dependencies first
COPY shared ./shared

# Copy mcp service
COPY mcp ./mcp

# Build the application
WORKDIR /app/mcp
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1001 conhub

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/mcp/target/release/mcp-service /app/mcp-service

# Copy .env.example as template
COPY mcp/.env.example /app/.env.example

# Set ownership
RUN chown -R conhub:conhub /app

USER conhub

EXPOSE 3004

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3004/health || exit 1

CMD ["/app/mcp-service"]
