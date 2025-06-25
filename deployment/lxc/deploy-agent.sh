#!/bin/bash

# Jarvis Agent LXC Deployment Script for Proxmox VE
# Usage: ./deploy-agent.sh <agent-type> <node-name> [options]

set -e

# Configuration
PROXMOX_API_URL="${PROXMOX_API_URL:-https://proxmox.local:8006/api2/json}"
PROXMOX_NODE="${PROXMOX_NODE:-pve}"
TEMPLATE_DIR="$(dirname "$0")"
JARVIS_IMAGE_REPO="${JARVIS_IMAGE_REPO:-registry.local/jarvis}"

# Default values
AGENT_TYPE="$1"
NODE_NAME="${2:-$PROXMOX_NODE}"
VMID="${3:-$(get_next_vmid)}"
AGENT_NAME="jarvis-${AGENT_TYPE}-$(date +%s)"

# Agent type configurations
declare -A AGENT_CONFIGS
AGENT_CONFIGS[network-monitor]="NetworkMonitor,DataCollector"
AGENT_CONFIGS[blockchain-auditor]="BlockchainAuditor,SecurityScanner"
AGENT_CONFIGS[gas-optimizer]="GasFeeOptimizer,DataCollector"
AGENT_CONFIGS[contract-maintainer]="ContractMaintainer,SecurityScanner,GasOptimizer"
AGENT_CONFIGS[infra-controller]="InfraController,ContainerOrchestrator"

# Function to get next available VMID
get_next_vmid() {
    local next_id=200
    while pvesh get /nodes/${NODE_NAME}/lxc/${next_id}/status >/dev/null 2>&1; do
        ((next_id++))
    done
    echo $next_id
}

# Function to generate random MAC address
generate_mac() {
    printf '02:00:00:%02x:%02x:%02x\n' $((RANDOM%256)) $((RANDOM%256)) $((RANDOM%256))
}

# Function to get available IP from DHCP range
get_next_ip() {
    # This would integrate with your DHCP server or IP management system
    echo "dhcp"
}

# Function to create LXC container
create_container() {
    local vmid=$1
    local config_file=$2
    
    echo "Creating LXC container ${vmid} on node ${NODE_NAME}..."
    
    # Create container using Proxmox API
    pvesh create /nodes/${NODE_NAME}/lxc \
        --vmid=${vmid} \
        --ostemplate=local:vztmpl/ubuntu-22.04-standard_22.04-1_amd64.tar.zst \
        --hostname="${AGENT_NAME}" \
        --cores=2 \
        --memory=2048 \
        --swap=512 \
        --rootfs=local-lvm:8 \
        --net0="name=eth0,bridge=vmbr0,firewall=1,hwaddr=$(generate_mac),ip=dhcp,type=veth" \
        --features="nesting=1,mount=nfs" \
        --startup="order=3,up=30,down=30" \
        --onboot=1 \
        --unprivileged=1 \
        --mp0="/var/lib/jarvis,mp=/data/jarvis,backup=1,size=10G" \
        --mp1="/var/log/jarvis,mp=/var/log/jarvis,backup=0,size=2G" \
        --tags="jarvis,agent,${AGENT_TYPE}" \
        --description="Jarvis AI Agent - ${AGENT_TYPE}"

    echo "Container ${vmid} created successfully"
}

# Function to configure agent-specific settings
configure_agent() {
    local vmid=$1
    local agent_type=$2
    
    case $agent_type in
        "network-monitor")
            echo "Configuring network monitoring agent..."
            # Add additional network interface for monitoring
            pvesh set /nodes/${NODE_NAME}/lxc/${vmid}/config \
                --net1="name=monitor,bridge=vmbr1,firewall=0,hwaddr=$(generate_mac),ip=dhcp,type=veth"
            ;;
        "blockchain-auditor")
            echo "Configuring blockchain auditor agent..."
            # Increase memory and add blockchain storage
            pvesh set /nodes/${NODE_NAME}/lxc/${vmid}/config \
                --memory=4096 \
                --mp2="/var/lib/blockchain,mp=/data/blockchain,backup=1,size=50G"
            ;;
        "infra-controller")
            echo "Configuring infrastructure controller agent..."
            # Make privileged and add Docker socket access
            pvesh set /nodes/${NODE_NAME}/lxc/${vmid}/config \
                --unprivileged=0 \
                --features="nesting=1,mount=nfs,keyctl=1" \
                --mp3="/var/run/docker.sock,mp=/var/run/docker.sock,backup=0"
            ;;
    esac
}

# Function to start and configure container
start_and_configure() {
    local vmid=$1
    local agent_type=$2
    
    echo "Starting container ${vmid}..."
    pvesh create /nodes/${NODE_NAME}/lxc/${vmid}/status/start
    
    # Wait for container to start
    echo "Waiting for container to start..."
    sleep 10
    
    # Copy setup script to container
    pct push ${vmid} "${TEMPLATE_DIR}/setup-agent.sh" /tmp/setup-agent.sh
    
    # Execute setup script inside container
    echo "Setting up Jarvis agent inside container..."
    pct exec ${vmid} -- bash /tmp/setup-agent.sh ${agent_type} ${AGENT_CONFIGS[$agent_type]}
    
    echo "Agent setup completed"
}

# Function to register agent with coordinator
register_agent() {
    local vmid=$1
    local agent_type=$2
    local container_ip
    
    # Get container IP
    container_ip=$(pct exec ${vmid} -- hostname -I | awk '{print $1}')
    
    echo "Registering agent with coordinator..."
    
    # Create agent registration payload
    cat > /tmp/agent_registration.json << EOF
{
    "agent_id": "$(uuidgen)",
    "name": "${AGENT_NAME}",
    "agent_type": "${agent_type}",
    "capabilities": ["${AGENT_CONFIGS[$agent_type]//,/\",\"}"],
    "endpoint": "http://${container_ip}:7777",
    "deployment_info": {
        "platform": "lxc",
        "node": "${NODE_NAME}",
        "vmid": ${vmid},
        "container_ip": "${container_ip}"
    }
}
EOF

    # Register with Jarvis coordinator
    curl -X POST \
        -H "Content-Type: application/json" \
        -d @/tmp/agent_registration.json \
        "${JARVIS_COORDINATOR_URL:-http://jarvis-core:8080}/api/v1/agents/register"
    
    rm /tmp/agent_registration.json
    echo "Agent registered successfully"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 <agent-type> [node-name] [vmid]"
    echo ""
    echo "Available agent types:"
    echo "  network-monitor      - Network monitoring and bandwidth analysis"
    echo "  blockchain-auditor   - Blockchain security auditing"
    echo "  gas-optimizer       - Gas fee optimization"
    echo "  contract-maintainer  - Smart contract maintenance"
    echo "  infra-controller     - Infrastructure orchestration"
    echo ""
    echo "Examples:"
    echo "  $0 network-monitor pve1 201"
    echo "  $0 blockchain-auditor pve2"
    echo "  $0 gas-optimizer"
    echo ""
    echo "Environment variables:"
    echo "  PROXMOX_API_URL      - Proxmox API URL (default: https://proxmox.local:8006/api2/json)"
    echo "  PROXMOX_NODE         - Default Proxmox node (default: pve)"
    echo "  JARVIS_COORDINATOR_URL - Jarvis coordinator URL for agent registration"
}

# Function to validate prerequisites
validate_prerequisites() {
    # Check if pvesh is available
    if ! command -v pvesh &> /dev/null; then
        echo "Error: pvesh command not found. Please install Proxmox VE tools."
        exit 1
    fi
    
    # Check if curl is available
    if ! command -v curl &> /dev/null; then
        echo "Error: curl command not found."
        exit 1
    fi
    
    # Check if uuidgen is available
    if ! command -v uuidgen &> /dev/null; then
        echo "Error: uuidgen command not found."
        exit 1
    fi
    
    # Check if agent type is valid
    if [[ -z "${AGENT_CONFIGS[$AGENT_TYPE]}" ]]; then
        echo "Error: Invalid agent type '${AGENT_TYPE}'"
        show_usage
        exit 1
    fi
}

# Function to create deployment manifest
create_deployment_manifest() {
    local vmid=$1
    local agent_type=$2
    local container_ip=$3
    
    cat > "/tmp/jarvis-${agent_type}-${vmid}-manifest.yaml" << EOF
apiVersion: jarvis.ai/v1
kind: AgentDeployment
metadata:
  name: ${AGENT_NAME}
  namespace: jarvis-agents
  labels:
    agent-type: ${agent_type}
    platform: lxc
    node: ${NODE_NAME}
spec:
  agent:
    type: ${agent_type}
    capabilities: [${AGENT_CONFIGS[$agent_type]}]
    image: ${JARVIS_IMAGE_REPO}/${agent_type}:latest
  deployment:
    platform: lxc
    vmid: ${vmid}
    node: ${NODE_NAME}
    ip: ${container_ip}
  resources:
    cpu: 2
    memory: 2Gi
    storage: 10Gi
  network:
    endpoints:
      - name: api
        port: 8080
        protocol: HTTP
      - name: p2p
        port: 7777
        protocol: QUIC
status:
  phase: Running
  startTime: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
EOF

    echo "Deployment manifest created: /tmp/jarvis-${agent_type}-${vmid}-manifest.yaml"
}

# Main execution
main() {
    echo "🤖 Jarvis Agent LXC Deployment"
    echo "================================"
    
    # Validate inputs
    if [[ -z "$AGENT_TYPE" ]]; then
        show_usage
        exit 1
    fi
    
    validate_prerequisites
    
    echo "Agent Type: $AGENT_TYPE"
    echo "Node: $NODE_NAME"
    echo "VMID: $VMID"
    echo "Capabilities: ${AGENT_CONFIGS[$AGENT_TYPE]}"
    echo ""
    
    # Confirm deployment
    read -p "Proceed with deployment? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Deployment cancelled"
        exit 0
    fi
    
    # Create and configure container
    create_container $VMID "${TEMPLATE_DIR}/jarvis-agent-template.conf"
    configure_agent $VMID $AGENT_TYPE
    start_and_configure $VMID $AGENT_TYPE
    
    # Get container IP for registration
    sleep 5
    CONTAINER_IP=$(pct exec ${VMID} -- hostname -I | awk '{print $1}')
    
    # Register agent and create manifest
    register_agent $VMID $AGENT_TYPE
    create_deployment_manifest $VMID $AGENT_TYPE $CONTAINER_IP
    
    echo ""
    echo "✅ Deployment completed successfully!"
    echo "Agent Name: $AGENT_NAME"
    echo "VMID: $VMID"
    echo "IP Address: $CONTAINER_IP"
    echo "Endpoint: http://${CONTAINER_IP}:8080"
    echo ""
    echo "To monitor the agent:"
    echo "  pct exec ${VMID} -- systemctl status jarvis-agent"
    echo "  pct exec ${VMID} -- journalctl -f -u jarvis-agent"
}

# Execute main function
main "$@"