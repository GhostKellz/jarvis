# JARVIS-NV: NVIDIA-Accelerated AI Agent for GhostChain Nodes
# Multi-stage build for optimized container size

# Build stage
FROM nvidia/cuda:12.0-devel-ubuntu22.04 as builder

# Install Rust and system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set working directory
WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY jarvis-core/ ./jarvis-core/
COPY jarvis-nv/ ./jarvis-nv/

# Build the application
WORKDIR /build/jarvis-nv
RUN cargo build --release --features gpu,node-integration,web5

# Runtime stage
FROM nvidia/cuda:12.0-runtime-ubuntu22.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create jarvis user
RUN useradd --create-home --shell /bin/bash jarvis

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /build/jarvis-nv/target/release/jarvis-nv ./

# Copy configuration files and certificates
COPY jarvis-nv/certs/ ./certs/
COPY jarvis-nv/proto/ ./proto/

# Create data directories
RUN mkdir -p /app/data /app/models /app/logs \
    && chown -R jarvis:jarvis /app

# Switch to jarvis user
USER jarvis

# Expose ports
EXPOSE 3000 4433 9090 50051

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ./jarvis-nv status || exit 1

# Default command
CMD ["./jarvis-nv", "start"]
