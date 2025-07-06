# Jarvis Daemon (jarvisd) Deployment Guide

The Jarvis Daemon (`jarvisd`) is the autonomous variant of Jarvis designed for hands-free blockchain monitoring and management. It runs as a background service and can be deployed on bare metal, virtual machines, or containers with full NVIDIA GPU support.

## Features

- **Autonomous Operation**: Runs continuously without user intervention
- **Blockchain Monitoring**: Real-time monitoring across multiple networks including GhostChain
- **AI-Powered Analysis**: Automated anomaly detection and pattern recognition
- **Zero-Trust Security**: Built-in security with encryption and access controls
- **Modern Networking**: IPv6, QUIC, HTTP/3, and gRPC support
- **Resource Management**: Automatic resource optimization and scaling
- **Container Ready**: Docker and NVIDIA container support
- **Systemd Integration**: Native Linux service integration

## Quick Start

### 1. Automated Installation (Recommended)

```bash
# Clone the repository
git clone https://github.com/ghostkellz/jarvis.git
cd jarvis

# Run the automated deployment
sudo ./deployment/deploy.sh install
```

### 2. Docker Deployment

```bash
# Standard Docker deployment
./deployment/deploy.sh docker

# NVIDIA GPU-enabled deployment
./deployment/deploy.sh nvidia
```

### 3. Manual Installation

See the [Manual Installation](#manual-installation) section below.

## Deployment Options

### Option 1: Systemd Service (Recommended for Production)

This method installs `jarvisd` as a native Linux systemd service.

**Advantages:**
- Native OS integration
- Automatic startup on boot
- Service management via systemctl
- System resource isolation
- Log integration with journald

**Requirements:**
- Linux with systemd
- Root access for installation
- Rust toolchain (installed automatically)

**Installation:**
```bash
sudo ./deployment/deploy.sh install
```

**Service Management:**
```bash
# Check status
sudo systemctl status jarvisd

# Start/stop/restart
sudo systemctl start jarvisd
sudo systemctl stop jarvisd
sudo systemctl restart jarvisd

# View logs
sudo journalctl -u jarvisd -f

# Enable/disable auto-start
sudo systemctl enable jarvisd
sudo systemctl disable jarvisd
```

### Option 2: Docker Container

This method runs `jarvisd` in a Docker container with optional monitoring stack.

**Advantages:**
- Easy deployment and scaling
- Isolated environment
- Consistent across platforms
- Built-in monitoring with Prometheus/Grafana
- GPU support via NVIDIA Container Toolkit

**Requirements:**
- Docker and Docker Compose
- NVIDIA Container Toolkit (for GPU support)

**Standard Deployment:**
```bash
cd deployment/docker
docker-compose up -d
```

**GPU-Enabled Deployment:**
```bash
cd deployment/docker
docker-compose -f docker-compose.yml -f docker-compose.nvidia.yml up -d
```

**Container Management:**
```bash
# View logs
docker-compose logs -f jarvisd

# Scale service
docker-compose up -d --scale jarvisd=2

# Update configuration
# Edit deployment/docker/config/jarvis.toml
docker-compose restart jarvisd

# Access monitoring (if enabled)
# Prometheus: http://localhost:9090
# Grafana: http://localhost:3000 (admin/jarvis123)
```

### Option 3: NVIDIA Container with GPU Acceleration

For AI-heavy workloads requiring GPU acceleration.

**Requirements:**
- NVIDIA GPU with compute capability 6.0+
- NVIDIA Container Toolkit
- Docker with NVIDIA runtime

**Installation:**
```bash
# Install NVIDIA Container Toolkit (Ubuntu/Debian)
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -
curl -s -L https://nvidia.github.io/nvidia-docker/$distribution/nvidia-docker.list | sudo tee /etc/apt/sources.list.d/nvidia-docker.list

sudo apt-get update && sudo apt-get install -y nvidia-container-toolkit
sudo systemctl restart docker

# Deploy with GPU support
./deployment/deploy.sh nvidia
```

## Configuration

### Default Configuration

The daemon uses `/etc/jarvis/jarvis.toml` for system-wide configuration. A template is provided at `deployment/config/jarvisd.toml`.

### Key Configuration Sections

#### Blockchain Settings
```toml
[blockchain]
enabled = true
network = "mainnet"
node_urls = ["https://rpc.ghostchain.io:443"]
monitor_interval = "30s"
```

#### AI Configuration
```toml
[ai]
provider = "ollama"
model = "llama2:7b"
api_endpoint = "http://localhost:11434"
enable_gpu_acceleration = true
```

#### Security Settings
```toml
[security]
enable_zero_trust = true
encryption_at_rest = true
session_timeout = "1h"
```

#### Agent Configuration
```toml
[agents.blockchain_monitor]
enabled = true
priority = "high"
restart_policy = "always"
```

### Environment Variables

Key environment variables for containerized deployments:

```bash
RUST_LOG=jarvisd=info,jarvis_core=info,jarvis_agent=info
JARVIS_CONFIG_DIR=/etc/jarvis
JARVIS_DATA_DIR=/var/lib/jarvis
JARVIS_LOG_DIR=/var/log/jarvis
```

## Manual Installation

For custom deployments or unsupported platforms:

### 1. Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install system dependencies (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install -y build-essential libssl-dev libsqlite3-dev protobuf-compiler

# Install system dependencies (Arch Linux)
sudo pacman -S base-devel openssl sqlite protobuf

# Install system dependencies (Fedora/RHEL)
sudo dnf install gcc gcc-c++ openssl-devel sqlite-devel protobuf-compiler
```

### 2. Build and Install

```bash
# Build the daemon
cargo build --release --bin jarvisd

# Install binary
sudo cp target/release/jarvisd /usr/local/bin/
sudo chmod +x /usr/local/bin/jarvisd

# Create user and directories
sudo useradd -r -s /bin/false -d /var/lib/jarvis -c "Jarvis Daemon" jarvis
sudo mkdir -p /etc/jarvis /var/lib/jarvis /var/log/jarvis
sudo chown jarvis:jarvis /var/lib/jarvis /var/log/jarvis

# Install configuration
sudo cp deployment/config/jarvisd.toml /etc/jarvis/jarvis.toml

# Install systemd service
sudo cp deployment/systemd/jarvisd.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable jarvisd
```

### 3. Start Service

```bash
sudo systemctl start jarvisd
sudo systemctl status jarvisd
```

## Monitoring and Management

### Health Checks

```bash
# Check daemon status
jarvisd status

# Health check (if daemon is running)
curl http://localhost:8080/health

# Detailed status via systemd
sudo systemctl status jarvisd
```

### Log Management

```bash
# View real-time logs (systemd)
sudo journalctl -u jarvisd -f

# View daemon logs (file-based)
sudo tail -f /var/log/jarvis/jarvisd.log

# View audit logs
sudo tail -f /var/log/jarvis/audit.log
```

### Metrics and Monitoring

If Prometheus is enabled, metrics are available at:
- `http://localhost:9090/metrics` - Prometheus metrics
- `http://localhost:8080/status` - Daemon status
- `http://localhost:3000` - Grafana dashboard (Docker deployment)

### Configuration Reload

```bash
# Reload configuration (systemd)
sudo systemctl reload jarvisd

# Reload via API (if enabled)
curl -X POST http://localhost:8080/reload
```

## Security Considerations

### Network Security

- The daemon binds to `127.0.0.1` by default (localhost only)
- Use a reverse proxy (nginx/traefik) for external access
- Enable TLS for all external communications
- Configure firewall rules appropriately

### Access Control

- Run as dedicated `jarvis` user (non-root)
- Limit file system access via systemd
- Enable audit logging for security events
- Use strong authentication for API access

### Data Protection

- Enable encryption at rest for database
- Secure API keys and secrets
- Regular backup of configuration and data
- Monitor for unauthorized access attempts

## Troubleshooting

### Common Issues

1. **Service fails to start**
   ```bash
   # Check logs
   sudo journalctl -u jarvisd -n 50
   
   # Verify configuration
   jarvisd --config /etc/jarvis/jarvis.toml --help
   
   # Check file permissions
   ls -la /var/lib/jarvis /var/log/jarvis
   ```

2. **High memory usage**
   ```bash
   # Check resource usage
   sudo systemctl show jarvisd --property=MemoryCurrent
   
   # Adjust limits in service file
   sudo systemctl edit jarvisd
   ```

3. **Connection issues**
   ```bash
   # Test network connectivity
   curl -v https://rpc.ghostchain.io:443/
   
   # Check DNS resolution
   nslookup rpc.ghostchain.io
   
   # Verify firewall rules
   sudo ufw status
   ```

4. **AI model issues**
   ```bash
   # Check Ollama connection
   curl http://localhost:11434/api/tags
   
   # Verify GPU access (if using NVIDIA)
   nvidia-smi
   docker run --rm --gpus all nvidia/cuda:11.0-base nvidia-smi
   ```

### Debug Mode

Enable debug logging for troubleshooting:

```bash
# Set environment variable
export RUST_LOG=debug

# Or edit configuration
[logging]
level = "debug"
```

### Support

For additional support:
- Check the project documentation
- Review GitHub issues
- Join the community discussions
- Contact the development team

## Production Deployment Checklist

- [ ] Install on dedicated server/VM
- [ ] Configure firewall rules
- [ ] Set up TLS certificates
- [ ] Configure monitoring and alerting
- [ ] Set up log rotation
- [ ] Configure backup strategy
- [ ] Test disaster recovery procedures
- [ ] Document operational procedures
- [ ] Train operations team
- [ ] Set up health monitoring
- [ ] Configure resource limits
- [ ] Enable security auditing
