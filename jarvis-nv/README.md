# JARVIS-NV: NVIDIA-Accelerated AI Agent for GhostChain Nodes

A GPU-accelerated, node-integrated AI agent designed specifically for GhostChain and ZVM environments. JARVIS-NV provides real-time monitoring, analytics, and AI-enhanced operations for blockchain nodes in high-performance, containerized deployments.

## 🚀 Features

### Core Capabilities
- **GPU Acceleration**: NVIDIA CUDA-powered AI inference and model operations
- **Blockchain Integration**: Native GhostChain and ZVM node monitoring and interaction
- **Web5 Stack**: Modern networking with IPv6, QUIC, HTTP/3 support
- **Real-time Analytics**: AI-driven anomaly detection and performance optimization
- **Metrics & Monitoring**: Comprehensive Prometheus metrics and observability

### AI Agent Features
- **Anomaly Detection**: Real-time identification of unusual patterns
- **Performance Optimization**: Automated tuning and resource optimization
- **Predictive Analytics**: Forecasting of network congestion and transaction volume
- **Learning Algorithms**: Continuous improvement through pattern recognition

### Networking & Communication
- **QUIC Protocol**: Ultra-low latency communication
- **HTTP/3**: Modern web protocol support
- **gRPC Services**: High-performance API endpoints
- **IPv6 Ready**: Future-proof networking

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     JARVIS-NV                               │
├─────────────────┬─────────────────┬─────────────────────────┤
│   GPU Manager   │   AI Agent      │    Metrics Collector    │
│   - CUDA Ops    │   - Anomaly Det │    - Prometheus         │
│   - Model Load  │   - Optimization│    - System Metrics     │
│   - Inference   │   - Prediction  │    - Custom Metrics     │
├─────────────────┼─────────────────┼─────────────────────────┤
│  Node Manager   │  GhostBridge    │    Web5 Stack           │
│  - GhostChain   │  - gRPC/QUIC    │    - IPv6/QUIC          │
│  - ZVM          │  - Communication│    - HTTP/3             │
│  - Monitoring   │  - Security     │    - Performance        │
├─────────────────┼─────────────────┼─────────────────────────┤
│              NVIDIA Core & Configuration                    │
│              - Hardware Integration                         │
│              - Container Runtime                            │
│              - Environment Management                       │
└─────────────────────────────────────────────────────────────┘
```

## 📋 Prerequisites

### System Requirements
- **OS**: Linux (Ubuntu 20.04+ recommended)
- **GPU**: NVIDIA GPU with CUDA Compute Capability 6.0+
- **Memory**: 8GB+ RAM (16GB+ recommended)
- **Storage**: 20GB+ available space

### Software Dependencies
- **Rust**: 1.75+ (edition 2024)
- **CUDA**: 11.0+ or 12.0+ (for GPU features)
- **Docker**: 20.10+ with NVIDIA Container Runtime (optional)
- **Docker Compose**: 2.0+ (optional)

### Hardware Tested
- NVIDIA GeForce RTX 3070, 3080, 3090, 4070, 4080, 4090
- NVIDIA Tesla V100, A100
- NVIDIA Quadro RTX 4000, 5000, 6000

## 🛠️ Installation

### Quick Start with Docker

1. **Clone the repository**:
   ```bash
   git clone https://github.com/ghostkellz/jarvis.git
   cd jarvis/jarvis-nv
   ```

2. **Configure environment**:
   ```bash
   cp config.example.toml config.toml
   # Edit config.toml as needed
   ```

3. **Start with Docker Compose**:
   ```bash
   docker-compose up -d
   ```

4. **Verify deployment**:
   ```bash
   curl http://localhost:9090/metrics
   curl http://localhost:3000/health
   ```

### Build from Source

1. **Install Rust**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Install system dependencies** (Ubuntu/Debian):
   ```bash
   sudo apt update
   sudo apt install -y build-essential pkg-config libssl-dev protobuf-compiler
   ```

3. **Install NVIDIA CUDA** (for GPU features):
   ```bash
   # Follow NVIDIA CUDA installation guide for your distro
   # https://developer.nvidia.com/cuda-downloads
   ```

4. **Build JARVIS-NV**:
   ```bash
   cd jarvis/jarvis-nv
   
   # Basic build (no GPU)
   cargo build --no-default-features --features web5
   
   # Full build with GPU support
   cargo build --features gpu,node-integration,web5
   ```

5. **Run**:
   ```bash
   ./target/debug/jarvis-nv start
   ```

## ⚙️ Configuration

JARVIS-NV uses a comprehensive TOML configuration system with environment variable support:

### Configuration File
```toml
[gpu]
enabled = true
device_id = 0
benchmark_on_startup = false

[node]
ghostchain_enabled = true
ghostchain_url = "http://localhost:8545"

[web5]
enabled = true
bind_address = "[::]:3000"
quic_enabled = true

[metrics]
enabled = true
bind_address = "127.0.0.1:9090"

[agent]
enabled = true
anomaly_detection = true
performance_optimization = true
```

### Environment Variables
```bash
export JARVIS_NV_GPU_ENABLED=true
export JARVIS_NV_GPU_DEVICE_ID=0
export JARVIS_NV_NODE_GHOSTCHAIN_URL=http://localhost:8545
export JARVIS_NV_WEB5_BIND_ADDRESS=[::]:3000
export JARVIS_NV_METRICS_BIND_ADDRESS=0.0.0.0:9090
```

## 🎯 Usage

### Command Line Interface

```bash
# Start the daemon
jarvis-nv start

# Check system status
jarvis-nv status

# Display GPU information
jarvis-nv gpu-info

# Show node information
jarvis-nv node-info

# Run GPU benchmark
jarvis-nv benchmark
```

### API Endpoints

#### Metrics (Prometheus)
```http
GET http://localhost:9090/metrics
```

#### Health Check
```http
GET http://localhost:3000/health
```

#### GPU Status
```http
GET http://localhost:3000/api/gpu/status
```

#### Node Status
```http
GET http://localhost:3000/api/node/status
```

#### Agent Status
```http
GET http://localhost:3000/api/agent/status
```

### gRPC Services

```protobuf
service GhostBridge {
  rpc GetNodeStatus(NodeStatusRequest) returns (NodeStatusResponse);
  rpc StreamMetrics(MetricsRequest) returns (stream MetricsResponse);
  rpc TriggerOptimization(OptimizationRequest) returns (OptimizationResponse);
}
```

## 📊 Monitoring & Observability

### Prometheus Metrics

JARVIS-NV exposes comprehensive metrics:

- **GPU Metrics**: Utilization, memory usage, temperature, power draw
- **Node Metrics**: Block height, transaction count, peer count, sync status
- **Network Metrics**: Connection count, bandwidth usage, latency
- **Agent Metrics**: Inference count, anomaly detections, optimizations
- **System Metrics**: CPU, memory, disk usage

### Grafana Dashboards

Pre-built dashboards are included for:
- GPU Performance Monitoring
- Blockchain Node Health
- Network Performance
- AI Agent Activity
- System Overview

Access Grafana at: `http://localhost:3001` (admin/jarvis123)

## 🔧 Development

### Project Structure

```
jarvis-nv/
├── src/
│   ├── main.rs           # Application entry point
│   ├── config.rs         # Configuration system
│   ├── gpu.rs           # GPU management
│   ├── node.rs          # Node integration
│   ├── agent.rs         # AI agent logic
│   ├── metrics.rs       # Metrics collection
│   ├── bridge.rs        # Communication bridge
│   ├── web5.rs          # Web5 stack
│   └── nvcore.rs        # NVIDIA core integration
├── proto/               # Protocol buffer definitions
├── certs/              # SSL/TLS certificates
├── Dockerfile          # Container build
├── docker-compose.yml  # Multi-service deployment
└── config.example.toml # Example configuration
```

### Building Features

```bash
# Build with specific features
cargo build --features gpu                    # GPU only
cargo build --features node-integration       # Blockchain only
cargo build --features web5                   # Networking only
cargo build --features gpu,node-integration   # Combined features
```

### Testing

```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test integration

# Run benchmarks
cargo bench
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes with tests
4. Submit a pull request

## 🚨 Troubleshooting

### Common Issues

#### GPU Not Detected
```bash
# Check NVIDIA driver
nvidia-smi

# Check CUDA installation
nvcc --version

# Verify NVIDIA Container Runtime (Docker)
docker run --rm --gpus all nvidia/cuda:12.0-base nvidia-smi
```

#### Build Errors
```bash
# Missing protobuf compiler
sudo apt install protobuf-compiler

# Missing SSL development libraries
sudo apt install libssl-dev

# CUDA headers not found
export CUDA_ROOT=/usr/local/cuda
```

#### Network Issues
```bash
# Check port availability
sudo netstat -tlnp | grep :3000

# Verify IPv6 support
ping6 ::1

# Test QUIC connectivity
nc -u localhost 4433
```

### Logs and Debugging

```bash
# Enable debug logging
RUST_LOG=jarvis_nv=debug cargo run

# View container logs
docker-compose logs jarvis-nv

# Follow real-time logs
docker-compose logs -f jarvis-nv
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.

## 🤝 Support

- **Documentation**: [JARVIS Documentation](../DOCS.md)
- **Issues**: [GitHub Issues](https://github.com/ghostkellz/jarvis/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ghostkellz/jarvis/discussions)

## 🔮 Future Roadmap

- [ ] Real-time ZVM integration
- [ ] Advanced ML model support (GPT, BERT variants)
- [ ] Multi-GPU support and orchestration
- [ ] Kubernetes deployment manifests
- [ ] WebAssembly plugin system
- [ ] Cross-chain bridge monitoring
- [ ] Advanced security hardening
- [ ] Performance optimization algorithms

---

**JARVIS-NV** is part of the larger JARVIS ecosystem, designed to bring AI capabilities to blockchain infrastructure and development workflows.
