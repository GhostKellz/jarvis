# Jarvis Usage Examples

Complete guide to using Jarvis with Ollama, Omen, and MCP tools for Arch Linux system administration.

## Table of Contents

1. [Configuration](#configuration)
2. [LLM Backends](#llm-backends)
3. [MCP Tools Usage](#mcp-tools-usage)
4. [Docker & KVM Management](#docker--kvm-management)
5. [Natural Language Commands](#natural-language-commands)
6. [Troubleshooting](#troubleshooting)

---

## Configuration

### Basic Setup

Create `~/.config/jarvis/jarvis.toml`:

```toml
[llm]
# Use Ollama directly (simplest)
primary_provider = "ollama"
ollama_url = "http://localhost:11434"
default_model = "llama3.1:8b"
context_window = 8192
temperature = 0.7

[system]
arch_package_manager = "pacman"  # or "yay" or "paru"

[mcp]
enabled = true
transport = "ws"
address = "127.0.0.1:7332"
```

### Hybrid Omen Configuration (Recommended)

For intelligent routing and cost optimization:

```toml
[llm]
# Primary provider
primary_provider = "omen"

# Ollama config (for local models)
ollama_url = "http://localhost:11434"
default_model = "llama3.1:8b"

# Omen config (for intelligent routing)
omen_enabled = true
omen_base_url = "http://localhost:8080/v1"
# omen_api_key = "optional-api-key"

context_window = 8192
temperature = 0.7

[system]
arch_package_manager = "yay"
gpu_enabled = true
gpu_devices = ["nvidia0", "nvidia1"]

[mcp]
enabled = true
transport = "ws"
address = "127.0.0.1:7332"

[mcp.tools]
system_status = true
package_manager = true
docker = true
proxmox = true
git = true
```

---

## LLM Backends

### Direct Ollama Usage

Jarvis uses Ollama for all AI-powered features:

```rust
// Automatic intent-based routing
LLMRouter::generate_with_intent(
    "How do I install Docker on Arch?",
    Intent::System
)
// → Routes to Ollama with system admin prompt
```

**Available Intents:**
- `Code` - Programming tasks (uses specialized code prompt)
- `System` - System administration (Arch Linux focused)
- `DevOps` - Infrastructure/Docker/Kubernetes
- `Reason` - Complex reasoning tasks

### Hybrid Omen + Ollama

When Omen is enabled, requests are intelligently routed:

```
User Request → Omen Gateway
    ↓
Intent Detection (automatic)
    ↓
├─ "code" → Ollama (deepseek-coder:6.7b, local, fast)
├─ "system" → Ollama (llama3.1:8b, local, safe)
├─ "devops" → Ollama (qwen2.5:7b, local)
└─ "reason" → Claude/GPT (cloud, high quality)
```

**Benefits:**
- ✅ Free inference for 90% of tasks
- ✅ Automatic fallback to cloud when needed
- ✅ Cost tracking and budget management
- ✅ Health checks and failover

---

## MCP Tools Usage

### System Status Tool

**Check system resources:**
```bash
# Via MCP protocol
{
  "tool": "jarvis_system_status",
  "arguments": {
    "verbose": false
  }
}
```

**Output:**
```
=== Jarvis System Status ===

CPU Usage: 45.32%
CPU Cores: 16

Memory: 24.50 GB / 64.00 GB (38.3%)
```

**Verbose mode:**
```json
{
  "tool": "jarvis_system_status",
  "arguments": {
    "verbose": true
  }
}
```

Includes process count and swap usage.

---

### Package Manager Tool

**Search for packages:**
```json
{
  "tool": "jarvis_package_manager",
  "arguments": {
    "action": "search",
    "package": "docker",
    "manager": "yay"
  }
}
```

**Get package info:**
```json
{
  "tool": "jarvis_package_manager",
  "arguments": {
    "action": "info",
    "package": "docker"
  }
}
```

**Install package (requires confirmation):**
```json
{
  "tool": "jarvis_package_manager",
  "arguments": {
    "action": "install",
    "package": "docker",
    "manager": "pacman",
    "confirm": false
  }
}
```

Returns installation command for manual execution (safety first!).

**Install with auto-confirm (use cautiously):**
```json
{
  "tool": "jarvis_package_manager",
  "arguments": {
    "action": "install",
    "package": "neovim",
    "confirm": true
  }
}
```

**Check for updates:**
```json
{
  "tool": "jarvis_package_manager",
  "arguments": {
    "action": "list-updates"
  }
}
```

**Update system:**
```json
{
  "tool": "jarvis_package_manager",
  "arguments": {
    "action": "update",
    "manager": "yay",
    "confirm": true
  }
}
```

---

## Docker & KVM Management

### Docker Operations

**List all containers:**
```json
{
  "tool": "jarvis_docker",
  "arguments": {
    "action": "list"
  }
}
```

**View container logs:**
```json
{
  "tool": "jarvis_docker",
  "arguments": {
    "action": "logs",
    "target": "ollama",
    "tail": 100
  }
}
```

**Container stats:**
```json
{
  "tool": "jarvis_docker",
  "arguments": {
    "action": "stats",
    "target": "ollama"
  }
}
```

**Start/Stop containers:**
```json
{
  "tool": "jarvis_docker",
  "arguments": {
    "action": "start",
    "target": "ollama"
  }
}
```

### AI-Powered Diagnostics

**Diagnose container issues:**
```json
{
  "tool": "jarvis_docker",
  "arguments": {
    "action": "diagnose",
    "target": "ollama",
    "llm_assist": true
  }
}
```

**What happens:**
1. Gathers container status, exit code, errors
2. Extracts last 20 log lines
3. Checks CPU/memory usage
4. Sends to Ollama with DevOps intent
5. Returns AI analysis with recommendations

**Example output:**
```
=== Diagnostic Report: ollama ===

Status: running | 0 |

Recent Logs (last 20 lines):
[INFO] Loading model llama3.1:8b
[INFO] Model loaded successfully
[INFO] Listening on 0.0.0.0:11434

Resource Usage:
CPU: 12.5% | Memory: 4.2GB (6.56%)

=== AI Analysis ===

The Ollama container is running healthy with normal resource usage.
Based on the diagnostics:

✅ Container Status: Running without errors
✅ Resource Usage: Well within limits (12.5% CPU, 4.2GB RAM)
✅ Logs: Clean startup, no errors detected

Recommendations:
- No immediate action needed
- Monitor memory if loading larger models
- Consider resource limits if running multiple models
```

**System-wide health check:**
```json
{
  "tool": "jarvis_docker",
  "arguments": {
    "action": "health",
    "llm_assist": true
  }
}
```

Provides overview of all containers with AI recommendations if issues detected.

### KVM/Libvirt Management

**List VMs:**
```json
{
  "tool": "jarvis_docker",
  "arguments": {
    "action": "vm-list"
  }
}
```

**VM status:**
```json
{
  "tool": "jarvis_docker",
  "arguments": {
    "action": "vm-status",
    "target": "windows11"
  }
}
```

**Start/Stop VM:**
```json
{
  "tool": "jarvis_docker",
  "arguments": {
    "action": "vm-start",
    "target": "ubuntu-server"
  }
}
```

**VM info with AI analysis:**
```json
{
  "tool": "jarvis_docker",
  "arguments": {
    "action": "vm-info",
    "target": "windows11",
    "llm_assist": true
  }
}
```

Gets VM details and asks Ollama for optimization recommendations.

---

## Natural Language Commands

### Coming Soon: NLP Parser

Natural language interface for intuitive system management:

```bash
jarvis "show me system status"
# → Calls SystemStatusTool

jarvis "install docker"
# → Calls PackageManagerTool with action=install, package=docker

jarvis "why is my ollama container using so much memory?"
# → Calls DockerTool diagnose with llm_assist=true

jarvis "list all running VMs"
# → Calls DockerTool vm-list
```

**Intent Detection:**
- Package queries → PackageManagerTool
- Docker/container queries → DockerTool (docker actions)
- VM queries → DockerTool (KVM actions)
- System queries → SystemStatusTool
- Complex troubleshooting → LLM with tool suggestions

---

## Troubleshooting

### Ollama Connection Issues

**Problem:** "Failed to connect to Ollama"

**Solution:**
```bash
# Check if Ollama is running
docker ps | grep ollama

# Check Ollama health
curl http://localhost:11434/api/tags

# Verify Ollama URL in config
cat ~/.config/jarvis/jarvis.toml | grep ollama_url
```

### MCP Server Issues

**Problem:** MCP tools not responding

**Solution:**
```bash
# Check MCP server status
jarvis mcp status  # (if implemented)

# Manually test MCP server
# Start in stdio mode
jarvis mcp server --transport stdio

# Or WebSocket mode
jarvis mcp server --transport ws --address 127.0.0.1:7332
```

### Permission Issues

**Problem:** "Permission denied" when managing packages/Docker/VMs

**Solution:**
```bash
# Docker permissions
sudo usermod -aG docker $USER
newgrp docker

# Libvirt permissions
sudo usermod -aG libvirt $USER

# Package manager (yay doesn't need sudo)
# pacman requires sudo (handled by tool)
```

### LLM Not Providing Analysis

**Problem:** DockerTool diagnose shows "⚠️ LLM analysis unavailable"

**Checklist:**
1. Is Ollama running? `docker ps | grep ollama`
2. Is model available? `ollama list`
3. Is config correct? Check `jarvis.toml`
4. Try direct test:
   ```bash
   curl http://localhost:11434/api/chat -d '{
     "model": "llama3.1:8b",
     "messages": [{"role": "user", "content": "test"}]
   }'
   ```

### Omen Routing Issues

**Problem:** Always falling back to Ollama, never using cloud

**Solution:**
```bash
# Check Omen is running
curl http://localhost:8080/health

# Verify configuration
cat ~/.config/jarvis/jarvis.toml | grep omen

# Check Omen logs
docker logs omen  # if running in Docker
```

**Problem:** Getting charged for local tasks

**Solution:**
Configure Omen's local preference:
```toml
# In Omen config (separate from jarvis.toml)
[routing]
prefer_local_for = ["code", "system", "devops", "tests", "regex"]
```

---

## Advanced Usage

### Custom System Prompts

Modify Ollama prompts for your environment:

```rust
// In jarvis-core/src/llm/ollama_client.rs
pub async fn system(&self, model: &str, request: &str, temperature: Option<f32>) -> Result<String> {
    let system = "You are an expert Arch Linux system administrator with 10+ years experience.
                  Specialize in: Docker, KVM/libvirt, pacman/AUR, Nvidia GPU management.
                  Always provide tested commands with explanations.";
    self.complete_with_system(model, system, request, temperature).await
}
```

### Model Selection

Use different models for different tasks:

```toml
[llm.intents]
system = "llama3.1:8b"           # General Arch admin
code = "deepseek-coder:6.7b"     # Code generation
devops = "qwen2.5:7b-instruct"   # Docker/K8s
reason = "claude-3-5-sonnet"     # Complex troubleshooting (via Omen)
```

### Batch Operations

Process multiple containers:

```bash
# Get all container names
docker ps --format '{{.Names}}'

# Diagnose each with LLM
for container in $(docker ps --format '{{.Names}}'); do
    jarvis mcp call jarvis_docker --action diagnose --target $container
done
```

---

## Performance Tips

**Ollama Optimization:**
```bash
# Use GPU acceleration (if available)
docker run -d --gpus all -p 11434:11434 ollama/ollama

# Preload models for faster response
ollama pull llama3.1:8b
ollama pull deepseek-coder:6.7b
```

**MCP Server:**
```bash
# Use WebSocket for persistent connections (faster than stdio)
[mcp]
transport = "ws"
address = "127.0.0.1:7332"
```

**Omen Routing:**
```bash
# Enable speculate_k strategy (starts local + cloud in parallel)
# Omen chooses fastest response
# See Omen docs for configuration
```

---

## Example Workflows

### 1. Debug Failing Container

```bash
# List containers
jarvis mcp call jarvis_docker --action list

# Get logs
jarvis mcp call jarvis_docker --action logs --target my-app --tail 100

# AI diagnosis
jarvis mcp call jarvis_docker --action diagnose --target my-app --llm_assist true

# Based on recommendations, check stats
jarvis mcp call jarvis_docker --action stats --target my-app
```

### 2. System Maintenance

```bash
# Check system status
jarvis mcp call jarvis_system_status --verbose true

# Check for updates
jarvis mcp call jarvis_package_manager --action list-updates

# Update system (with confirmation)
jarvis mcp call jarvis_package_manager --action update --manager yay --confirm true

# Check Docker health
jarvis mcp call jarvis_docker --action health --llm_assist true
```

### 3. VM Management with AI

```bash
# List all VMs
jarvis mcp call jarvis_docker --action vm-list

# Check specific VM
jarvis mcp call jarvis_docker --action vm-status --target windows11

# Get AI optimization recommendations
jarvis mcp call jarvis_docker --action vm-info --target windows11 --llm_assist true
```

---

## Integration Examples

### Using from Python

```python
import requests
import json

def call_mcp_tool(tool, arguments):
    response = requests.post(
        'http://localhost:7332/v1/tools/call',
        json={
            'tool': tool,
            'arguments': arguments
        }
    )
    return response.json()

# Diagnose Docker container
result = call_mcp_tool('jarvis_docker', {
    'action': 'diagnose',
    'target': 'ollama',
    'llm_assist': True
})

print(result['content'])
```

### Using from Shell Scripts

```bash
#!/bin/bash

# Check if container is healthy
diagnose_container() {
    local container=$1

    jarvis mcp call jarvis_docker \
        --action diagnose \
        --target "$container" \
        --llm_assist true
}

# Monitor all containers
for container in $(docker ps --format '{{.Names}}'); do
    echo "Checking $container..."
    diagnose_container "$container"
    echo "---"
done
```

---

## Next Steps

1. **Explore Natural Language Interface** (coming soon)
   - Conversational system management
   - Intent detection and routing
   - Context-aware responses

2. **Add Custom Tools**
   - Implement MCP Tool trait
   - Register in server.rs
   - Use LLMRouter for AI features

3. **Optimize Performance**
   - Tune model selection
   - Configure Omen routing
   - Enable GPU acceleration

4. **Contribute**
   - Report issues on GitHub
   - Submit tool implementations
   - Share configuration examples

---

**Questions or Issues?**
- GitHub: https://github.com/ghostkellz/jarvis
- Documentation: `docs/`
- Examples: This file!
