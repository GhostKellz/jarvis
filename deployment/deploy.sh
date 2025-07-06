#!/bin/bash
set -euo pipefail

# Jarvis Daemon Deployment Script
# This script automates the deployment of Jarvis Daemon on various platforms

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Configuration
JARVIS_USER="jarvis"
JARVIS_GROUP="jarvis"
INSTALL_DIR="/opt/jarvis"
CONFIG_DIR="/etc/jarvis"
DATA_DIR="/var/lib/jarvis"
LOG_DIR="/var/log/jarvis"
PID_FILE="/var/run/jarvisd.pid"
BINARY_PATH="/usr/local/bin/jarvisd"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root"
        exit 1
    fi
}

# Detect operating system
detect_os() {
    if [[ -f /etc/os-release ]]; then
        . /etc/os-release
        OS=$ID
        OS_VERSION=$VERSION_ID
    else
        log_error "Cannot detect operating system"
        exit 1
    fi
    
    log_info "Detected OS: $OS $OS_VERSION"
}

# Install system dependencies
install_dependencies() {
    log_info "Installing system dependencies..."
    
    case $OS in
        "arch"|"manjaro")
            pacman -Syu --needed --noconfirm \
                base-devel \
                openssl \
                sqlite \
                protobuf \
                systemd \
                docker \
                docker-compose
            ;;
        "ubuntu"|"debian")
            apt-get update
            apt-get install -y \
                build-essential \
                libssl-dev \
                libsqlite3-dev \
                protobuf-compiler \
                systemd \
                docker.io \
                docker-compose
            ;;
        "fedora"|"rhel"|"centos")
            dnf install -y \
                gcc \
                gcc-c++ \
                openssl-devel \
                sqlite-devel \
                protobuf-compiler \
                systemd \
                docker \
                docker-compose
            ;;
        *)
            log_warn "Unsupported OS: $OS. Proceeding anyway..."
            ;;
    esac
    
    log_success "Dependencies installed"
}

# Install Rust if not present
install_rust() {
    if ! command -v cargo &> /dev/null; then
        log_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source ~/.cargo/env
        log_success "Rust installed"
    else
        log_info "Rust already installed"
    fi
}

# Create system user
create_user() {
    if ! id "$JARVIS_USER" &>/dev/null; then
        log_info "Creating system user: $JARVIS_USER"
        useradd -r -s /bin/false -d "$DATA_DIR" -c "Jarvis Daemon" "$JARVIS_USER"
        log_success "User $JARVIS_USER created"
    else
        log_info "User $JARVIS_USER already exists"
    fi
}

# Create directories
create_directories() {
    log_info "Creating directories..."
    
    mkdir -p "$INSTALL_DIR" "$CONFIG_DIR" "$DATA_DIR" "$LOG_DIR"
    chown -R "$JARVIS_USER:$JARVIS_GROUP" "$DATA_DIR" "$LOG_DIR"
    chmod 755 "$INSTALL_DIR" "$CONFIG_DIR"
    chmod 750 "$DATA_DIR" "$LOG_DIR"
    
    log_success "Directories created"
}

# Build and install binary
build_and_install() {
    log_info "Building Jarvis Daemon..."
    
    cd "$PROJECT_ROOT"
    
    # Build in release mode
    cargo build --release --bin jarvisd
    
    # Install binary
    cp target/release/jarvisd "$BINARY_PATH"
    chmod 755 "$BINARY_PATH"
    
    log_success "Binary installed to $BINARY_PATH"
}

# Install configuration
install_config() {
    log_info "Installing configuration..."
    
    if [[ ! -f "$CONFIG_DIR/jarvis.toml" ]]; then
        cp "$PROJECT_ROOT/jarvis.toml.example" "$CONFIG_DIR/jarvis.toml"
        chown root:root "$CONFIG_DIR/jarvis.toml"
        chmod 644 "$CONFIG_DIR/jarvis.toml"
        log_success "Configuration installed"
    else
        log_info "Configuration already exists, skipping"
    fi
}

# Install systemd service
install_systemd_service() {
    log_info "Installing systemd service..."
    
    cp "$PROJECT_ROOT/deployment/systemd/jarvisd.service" /etc/systemd/system/
    systemctl daemon-reload
    systemctl enable jarvisd
    
    log_success "Systemd service installed and enabled"
}

# Start service
start_service() {
    log_info "Starting Jarvis Daemon service..."
    
    systemctl start jarvisd
    sleep 2
    
    if systemctl is-active --quiet jarvisd; then
        log_success "Jarvis Daemon started successfully"
    else
        log_error "Failed to start Jarvis Daemon"
        systemctl status jarvisd
        exit 1
    fi
}

# Docker deployment
deploy_docker() {
    log_info "Deploying with Docker..."
    
    cd "$PROJECT_ROOT/deployment/docker"
    
    # Create config directory for Docker
    mkdir -p ./config
    if [[ ! -f ./config/jarvis.toml ]]; then
        cp "$PROJECT_ROOT/jarvis.toml.example" ./config/jarvis.toml
    fi
    
    # Build and start containers
    docker-compose up -d --build
    
    log_success "Docker deployment completed"
}

# NVIDIA container deployment
deploy_nvidia_container() {
    log_info "Deploying with NVIDIA container support..."
    
    # Check if NVIDIA container toolkit is installed
    if ! command -v nvidia-container-runtime &> /dev/null; then
        log_warn "NVIDIA Container Toolkit not found. Please install it first."
        log_info "Visit: https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html"
        return 1
    fi
    
    cd "$PROJECT_ROOT/deployment/docker"
    
    # Use NVIDIA runtime
    COMPOSE_FILE="docker-compose.yml" \
    COMPOSE_FILE_OVERRIDE="docker-compose.nvidia.yml" \
    docker-compose up -d --build
    
    log_success "NVIDIA container deployment completed"
}

# Cleanup function
cleanup() {
    log_info "Cleaning up temporary files..."
    # Add cleanup logic here if needed
}

# Show usage
usage() {
    echo "Usage: $0 [OPTIONS] COMMAND"
    echo ""
    echo "Commands:"
    echo "  install       Install Jarvis Daemon (systemd service)"
    echo "  docker        Deploy with Docker"
    echo "  nvidia        Deploy with NVIDIA container support"
    echo "  uninstall     Uninstall Jarvis Daemon"
    echo "  status        Show service status"
    echo ""
    echo "Options:"
    echo "  -h, --help    Show this help message"
}

# Uninstall function
uninstall() {
    log_info "Uninstalling Jarvis Daemon..."
    
    # Stop and disable service
    if systemctl is-active --quiet jarvisd; then
        systemctl stop jarvisd
    fi
    systemctl disable jarvisd 2>/dev/null || true
    
    # Remove files
    rm -f /etc/systemd/system/jarvisd.service
    rm -f "$BINARY_PATH"
    systemctl daemon-reload
    
    # Optionally remove user and data
    read -p "Remove user $JARVIS_USER and data directories? [y/N]: " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        userdel "$JARVIS_USER" 2>/dev/null || true
        rm -rf "$DATA_DIR" "$LOG_DIR"
        log_info "User and data directories removed"
    fi
    
    log_success "Jarvis Daemon uninstalled"
}

# Show status
show_status() {
    echo "=== Jarvis Daemon Status ==="
    echo ""
    
    # Systemd status
    if systemctl is-enabled --quiet jarvisd 2>/dev/null; then
        echo "Systemd service: enabled"
        if systemctl is-active --quiet jarvisd; then
            echo "Service status: running"
        else
            echo "Service status: stopped"
        fi
    else
        echo "Systemd service: not installed"
    fi
    
    # Binary check
    if [[ -f "$BINARY_PATH" ]]; then
        echo "Binary: installed ($BINARY_PATH)"
        "$BINARY_PATH" --version 2>/dev/null || echo "Binary version: unknown"
    else
        echo "Binary: not installed"
    fi
    
    # Docker check
    if docker ps --format "table {{.Names}}\t{{.Status}}" | grep -q jarvisd; then
        echo "Docker container: running"
    else
        echo "Docker container: not running"
    fi
    
    echo ""
}

# Main execution
main() {
    case "${1:-}" in
        "install")
            check_root
            detect_os
            install_dependencies
            install_rust
            create_user
            create_directories
            build_and_install
            install_config
            install_systemd_service
            start_service
            log_success "Jarvis Daemon installation completed!"
            ;;
        "docker")
            deploy_docker
            ;;
        "nvidia")
            deploy_nvidia_container
            ;;
        "uninstall")
            check_root
            uninstall
            ;;
        "status")
            show_status
            ;;
        "-h"|"--help"|"help")
            usage
            ;;
        *)
            log_error "Unknown command: ${1:-}"
            usage
            exit 1
            ;;
    esac
}

# Set trap for cleanup
trap cleanup EXIT

# Run main function
main "$@"
