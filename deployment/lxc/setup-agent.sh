#!/bin/bash

# Jarvis Agent Setup Script for LXC Containers
# This script runs inside the LXC container to set up the Jarvis agent

set -e

AGENT_TYPE="$1"
CAPABILITIES="$2"
JARVIS_VERSION="${JARVIS_VERSION:-latest}"
JARVIS_USER="jarvis"
JARVIS_HOME="/opt/jarvis"
DATA_DIR="/data/jarvis"
LOG_DIR="/var/log/jarvis"

echo "🔧 Setting up Jarvis Agent: $AGENT_TYPE"
echo "Capabilities: $CAPABILITIES"

# Update system
apt-get update
apt-get upgrade -y

# Install required packages
apt-get install -y \
    curl \
    wget \
    unzip \
    ca-certificates \
    gnupg \
    lsb-release \
    systemd \
    sudo \
    htop \
    iotop \
    iftop \
    tcpdump \
    netcat \
    jq \
    git

# Install Docker (for container orchestration agents)
if [[ "$AGENT_TYPE" == "infra-controller" ]]; then
    echo "Installing Docker for infrastructure controller..."
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null
    apt-get update
    apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
    systemctl enable docker
    systemctl start docker
fi

# Install Rust (for building Jarvis)
echo "Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Create jarvis user
echo "Creating Jarvis user..."
useradd -r -s /bin/bash -d $JARVIS_HOME $JARVIS_USER || true
mkdir -p $JARVIS_HOME
mkdir -p $DATA_DIR
mkdir -p $LOG_DIR
chown -R $JARVIS_USER:$JARVIS_USER $JARVIS_HOME $DATA_DIR $LOG_DIR

# Add jarvis user to docker group if needed
if [[ "$AGENT_TYPE" == "infra-controller" ]]; then
    usermod -aG docker $JARVIS_USER
fi

# Download and install Jarvis binary
echo "Installing Jarvis binary..."
JARVIS_BINARY_URL="${JARVIS_BINARY_URL:-https://github.com/ghostchain/jarvis/releases/download/${JARVIS_VERSION}/jarvis-${AGENT_TYPE}-linux-amd64}"

curl -L "$JARVIS_BINARY_URL" -o "$JARVIS_HOME/jarvis-agent"
chmod +x "$JARVIS_HOME/jarvis-agent"
chown $JARVIS_USER:$JARVIS_USER "$JARVIS_HOME/jarvis-agent"

# Create agent configuration
echo "Creating agent configuration..."
cat > "$JARVIS_HOME/config.toml" << EOF
[agent]
id = "$(uuidgen)"
name = "jarvis-${AGENT_TYPE}-$(hostname)"
type = "${AGENT_TYPE}"
capabilities = [$(echo "$CAPABILITIES" | sed 's/,/", "/g' | sed 's/^/"/' | sed 's/$/"/')] 

[network]
listen_port = 7777
api_port = 8080
coordinator_url = "${JARVIS_COORDINATOR_URL:-http://jarvis-core:8080}"
discovery_methods = ["multicast", "dns", "manual"]

[storage]
data_path = "${DATA_DIR}"
log_path = "${LOG_DIR}"
database_path = "${DATA_DIR}/agent.db"

[security]
enable_tls = true
cert_path = "${JARVIS_HOME}/certs"
auto_update = true

[monitoring]
enable_metrics = true
metrics_port = 9090
log_level = "info"
health_check_interval = "30s"

# Agent-specific configurations
$(generate_agent_config "$AGENT_TYPE")
EOF

chown $JARVIS_USER:$JARVIS_USER "$JARVIS_HOME/config.toml"

# Generate agent-specific configuration
generate_agent_config() {
    local agent_type="$1"
    
    case "$agent_type" in
        "network-monitor")
            cat << 'EOF'
[network_monitoring]
interfaces = ["eth0", "monitor"]
bandwidth_threshold = 80
latency_threshold = "100ms"
packet_loss_threshold = 5.0
monitoring_interval = "30s"
EOF
            ;;
        "blockchain-auditor")
            cat << 'EOF'
[blockchain_audit]
networks = ["ghostchain", "ethereum", "polygon"]
audit_interval = "300s"
security_rules_path = "/etc/jarvis/audit-rules"
contract_cache_size = 1000
max_parallel_audits = 5

[ghostchain]
rpc_url = "http://ghostchain:8545"
ws_url = "ws://ghostchain:8546"

[ethereum]
rpc_url = "https://mainnet.infura.io/v3/${INFURA_PROJECT_ID}"
EOF
            ;;
        "gas-optimizer")
            cat << 'EOF'
[gas_optimization]
networks = ["ghostchain", "ethereum", "polygon", "arbitrum"]
optimization_strategy = "ml_based"
price_sources = ["ghostchain", "ethgasstation", "blocknative"]
update_interval = "60s"
analysis_window = "24h"

[ml_models]
gas_prediction_model = "lstm"
congestion_model = "ensemble"
optimization_model = "reinforcement_learning"
EOF
            ;;
        "contract-maintainer")
            cat << 'EOF'
[contract_maintenance]
maintenance_schedule = "0 2 * * *"  # Daily at 2 AM
auto_maintenance = false
governance_required = true
backup_before_upgrade = true

[governance]
voting_threshold = 0.6
proposal_timeout = "168h"  # 7 days
emergency_threshold = 0.8

[ai_analysis]
security_model = "transformer"
gas_optimization_model = "genetic_algorithm"
code_quality_model = "static_analysis"
EOF
            ;;
        "infra-controller")
            cat << 'EOF'
[infrastructure]
platforms = ["docker", "lxc", "kubernetes"]
max_concurrent_deployments = 10
health_check_interval = "60s"
auto_scaling = true

[docker]
socket_path = "/var/run/docker.sock"
registry_url = "registry.local"

[lxc]
storage_pool = "local-lvm"
default_template = "ubuntu-22.04"

[proxmox]
api_url = "${PROXMOX_API_URL}"
node = "${PROXMOX_NODE:-pve}"
EOF
            ;;
    esac
}

# Create systemd service
echo "Creating systemd service..."
cat > "/etc/systemd/system/jarvis-agent.service" << EOF
[Unit]
Description=Jarvis AI Agent (${AGENT_TYPE})
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=${JARVIS_USER}
Group=${JARVIS_USER}
WorkingDirectory=${JARVIS_HOME}
ExecStart=${JARVIS_HOME}/jarvis-agent --config ${JARVIS_HOME}/config.toml
ExecReload=/bin/kill -HUP \$MAINPID
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=jarvis-agent

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=${DATA_DIR} ${LOG_DIR}

# Resource limits
LimitNOFILE=65536
LimitNPROC=32768

[Install]
WantedBy=multi-user.target
EOF

# Create log rotation
echo "Setting up log rotation..."
cat > "/etc/logrotate.d/jarvis-agent" << EOF
${LOG_DIR}/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 0644 ${JARVIS_USER} ${JARVIS_USER}
    postrotate
        systemctl reload jarvis-agent
    endscript
}
EOF

# Create health check script
echo "Creating health check script..."
cat > "$JARVIS_HOME/health-check.sh" << 'EOF'
#!/bin/bash

# Jarvis Agent Health Check Script

HEALTH_URL="http://localhost:8080/health"
TIMEOUT=10
RETRIES=3

for i in $(seq 1 $RETRIES); do
    if curl -f -s --max-time $TIMEOUT "$HEALTH_URL" > /dev/null; then
        echo "Health check passed"
        exit 0
    fi
    echo "Health check attempt $i failed"
    sleep 5
done

echo "Health check failed after $RETRIES attempts"
exit 1
EOF

chmod +x "$JARVIS_HOME/health-check.sh"
chown $JARVIS_USER:$JARVIS_USER "$JARVIS_HOME/health-check.sh"

# Create monitoring script
echo "Creating monitoring script..."
cat > "$JARVIS_HOME/monitor.sh" << 'EOF'
#!/bin/bash

# Jarvis Agent Monitoring Script

echo "=== Jarvis Agent Status ==="
systemctl status jarvis-agent --no-pager

echo ""
echo "=== Resource Usage ==="
echo "CPU: $(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | cut -d'%' -f1)%"
echo "Memory: $(free -h | awk '/^Mem:/ {print $3 "/" $2}')"
echo "Disk: $(df -h /data/jarvis | awk 'NR==2 {print $3 "/" $2 " (" $5 ")"}')"

echo ""
echo "=== Network Connections ==="
netstat -tlnp | grep jarvis-agent || echo "No network connections found"

echo ""
echo "=== Recent Logs ==="
journalctl -u jarvis-agent --no-pager -n 10
EOF

chmod +x "$JARVIS_HOME/monitor.sh"
chown $JARVIS_USER:$JARVIS_USER "$JARVIS_HOME/monitor.sh"

# Enable and start the service
echo "Enabling and starting Jarvis agent service..."
systemctl daemon-reload
systemctl enable jarvis-agent
systemctl start jarvis-agent

# Wait for service to start
sleep 5

# Check service status
if systemctl is-active --quiet jarvis-agent; then
    echo "✅ Jarvis agent started successfully"
    systemctl status jarvis-agent --no-pager
else
    echo "❌ Failed to start Jarvis agent"
    journalctl -u jarvis-agent --no-pager -n 20
    exit 1
fi

# Run health check
echo "Running health check..."
if "$JARVIS_HOME/health-check.sh"; then
    echo "✅ Health check passed"
else
    echo "⚠️  Health check failed, but service is running"
fi

echo ""
echo "🎉 Jarvis agent setup completed!"
echo "Agent Type: $AGENT_TYPE"
echo "Capabilities: $CAPABILITIES"
echo "Status: $(systemctl is-active jarvis-agent)"
echo "Config: $JARVIS_HOME/config.toml"
echo "Logs: journalctl -u jarvis-agent -f"
echo "Monitor: $JARVIS_HOME/monitor.sh"