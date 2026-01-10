# Multi-stage build for MCP Service (Rust 1.92)
FROM rust:1.92-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy shared workspace dependencies
COPY shared-middleware-confuse ../shared-middleware-confuse

# Copy source files
COPY mcp-server/Cargo.toml mcp-server/Cargo.lock ./
COPY mcp-server/src ./src

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
