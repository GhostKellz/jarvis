# Jarvis + OMEN Integration Guide

Complete integration guide for [Jarvis](https://github.com/ghostkellz/jarvis) - the all-in-one CLI-native AI companion built in Rust - with OMEN for intelligent model routing, cost optimization, and multi-provider support.

## Overview

**Jarvis** is your local AI assistant for Rust, Linux, and Homelab operations, featuring:
- Developer Copilot (Rust, Zig, Shell scripting with LSP integration)
- DevOps & Infrastructure Management (systemctl, Docker, Proxmox, Snapper, Btrfs)
- Linux System Companion (package management, dotfiles, environment awareness)
- Privacy-first design (local LLM support, no telemetry, fully offline)
- Modular architecture (`jarvis-core`, `jarvis-agent`, `jarvis-shell`)

**OMEN** supercharges Jarvis by providing:
- Smart model routing based on task complexity and intent
- Multi-provider support with unified API
- Cost optimization (route simple tasks to local Ollama, complex to Claude/GPT)
- Usage tracking and budget controls
- High availability with fallback providers

Together, they create a powerful, cost-effective, and intelligent AI companion that adapts to your workflow.

## Why Integrate Jarvis with OMEN?

| Benefit | Description |
|---------|-------------|
| 💰 **Cost Savings** | Route 80%+ of Jarvis tasks to free local models |
| 🧠 **Intelligent Routing** | OMEN selects best model for each task type |
| 🏠 **Homelab Optimized** | Leverage local GPUs for privacy and speed |
| 🔌 **Multi-Provider** | Access Claude, GPT, Gemini, Ollama from single config |
| 📊 **Usage Tracking** | Monitor costs and usage across all Jarvis operations |
| ⚡ **Performance** | Local routing for instant responses on simple tasks |

## Quick Start

### 1. Configure Jarvis to Use OMEN

Update your `jarvis.toml`:

```toml
# ~/.config/jarvis/jarvis.toml

[llm]
# Use OMEN as the backend
provider = "omen"
base_url = "http://localhost:8080/v1"
api_key = "env:OMEN_API_KEY"
model = "auto"  # Let OMEN choose optimal model

[llm.routing]
# Provide hints for OMEN routing
default_intent = "agent"
code_intent = "code"
devops_intent = "system"
reasoning_intent = "reason"

[llm.tags]
# Default tags for all requests
source = "jarvis"
project = "homelab"

[features]
# Enable Jarvis features
code_copilot = true
devops_helper = true
system_companion = true
git_aware = true
environment_detection = true

[performance]
# Performance settings
request_timeout_ms = 30000
prefer_local = true  # Hint to OMEN to prefer local models
max_retries = 3
```

Or via environment variables:

```bash
# ~/.bashrc or ~/.zshrc
export JARVIS_LLM_PROVIDER="omen"
export JARVIS_LLM_BASE_URL="http://localhost:8080/v1"
export OMEN_API_KEY="your-omen-api-key"
export JARVIS_LLM_MODEL="auto"
export JARVIS_PREFER_LOCAL="true"
```

### 2. Configure OMEN for Jarvis Workloads

Optimize OMEN routing for typical Jarvis use cases:

```toml
# omen.toml

[routing]
# Prefer local models for Jarvis tasks
prefer_local_for = [
    "code",
    "system",
    "devops",
    "shell",
    "config",
]

# Use cloud models for complex reasoning
use_cloud_for = [
    "architecture",
    "debugging",
    "optimization",
]

fallback_to_cloud = true
enable_auto_swap = true

[routing.intents.agent]
primary_provider = "ollama"
models = ["mistral:7b-instruct", "llama3.1:8b-instruct"]
complexity_threshold = "medium"
fallback_provider = "anthropic"

[routing.intents.code]
primary_provider = "ollama"
models = ["deepseek-coder:6.7b", "codellama:13b-instruct"]
timeout_ms = 10000
fallback_provider = "anthropic"

[routing.intents.system]
primary_provider = "ollama"
models = ["mistral:7b-instruct"]
timeout_ms = 5000

[routing.intents.reason]
primary_provider = "anthropic"
models = ["claude-3-5-sonnet-20241022"]
use_local_first = false

[providers.ollama]
endpoints = ["http://localhost:11434", "http://homelab-gpu:11434"]
models = [
    "deepseek-coder:6.7b",
    "codellama:13b-instruct",
    "mistral:7b-instruct",
    "llama3.1:8b-instruct",
]
priority = 100

[providers.anthropic]
api_key = "env:ANTHROPIC_API_KEY"
models = ["claude-3-5-sonnet-20241022", "claude-3-haiku-20240307"]
priority = 50

[providers.openai]
api_key = "env:OPENAI_API_KEY"
models = ["gpt-4-turbo-preview"]
priority = 40

[budget]
monthly_usd = 100
soft_limit_usd = 80
alert_threshold = 0.8
track_by = ["provider", "intent", "source"]

[cache]
enabled = true
backend = "redis"
ttl_seconds = 7200
cache_by_intent = true
```

### 3. Launch the Stack

```bash
# Start OMEN and dependencies
docker compose up -d omen redis ollama

# Or run OMEN natively
cargo run --release --bin omen

# Pull recommended models for Jarvis
ollama pull deepseek-coder:6.7b
ollama pull codellama:13b-instruct
ollama pull mistral:7b-instruct
ollama pull llama3.1:8b-instruct

# Install Jarvis
cargo install --git https://github.com/ghostkellz/jarvis jarvis

# Verify connection
jarvis --check-connection
```

## Usage Examples

### Code Copilot

```bash
# Generate Rust code
jarvis code "Create an async HTTP client with retry logic"
# OMEN → DeepSeek Coder (local, optimized for code)

# Refactor existing code
jarvis refactor src/main.rs --focus "Extract error handling"
# OMEN → CodeLlama (local, specialized for refactoring)

# Explain complex code
jarvis explain src/server.rs
# OMEN → Claude 3.5 Sonnet (complex explanation needs reasoning)

# Generate tests
jarvis test --file src/parser.rs --coverage 90
# OMEN → Claude (high coverage requires comprehensive thinking)
```

### DevOps & Infrastructure

```bash
# System diagnostics
jarvis diagnose "Why is my Docker container restarting?"
# OMEN → Mistral (local, fast system tasks)

# Proxmox management
jarvis proxmox "Create a new VM with 8GB RAM and Ubuntu 22.04"
# OMEN → Llama3.1 (local, handles structured commands)

# Snapshot management
jarvis snapshot --create --description "Before kernel upgrade"
# OMEN → Mistral (local, simple Snapper operation)

# Complex deployment
jarvis deploy "Set up a three-node Kubernetes cluster with Cilium"
# OMEN → Claude 3.5 Sonnet (complex orchestration needs reasoning)
```

### Linux System Companion

```bash
# Package management
jarvis install "kubectl helm k9s"
# OMEN → Mistral (local, fast package operations)

# System optimization
jarvis optimize "Improve boot time on this system"
# OMEN → Claude 3.5 Sonnet (complex analysis and recommendations)

# Dotfile management
jarvis dotfiles sync
# OMEN → Llama3.1 (local, environment-aware operations)

# Troubleshooting
jarvis fix "NetworkManager not starting after reboot"
# OMEN → Claude 3.5 Sonnet (complex debugging)
```

### Git Integration

```bash
# Commit message generation
jarvis commit
# OMEN → DeepSeek Coder (local, code-aware)

# PR description
jarvis pr --create
# OMEN → Claude 3.5 Sonnet (comprehensive description)

# Code review
jarvis review --file src/auth.rs
# OMEN → Claude 3.5 Sonnet (security and quality focus)
```

### Interactive Mode

```bash
# Start Jarvis shell
jarvis shell

jarvis> deploy nginx with SSL
# OMEN → Mistral → Plans deployment
# OMEN → Claude → Generates secure config

jarvis> explain the deployment
# OMEN → Claude → Detailed explanation

jarvis> optimize for 10k concurrent connections
# OMEN → Claude → Advanced tuning recommendations
```

## Rust Integration

### Using Jarvis as a Library with OMEN

```rust
// Cargo.toml
[dependencies]
jarvis-core = { git = "https://github.com/ghostkellz/jarvis" }
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }

// src/main.rs
use jarvis_core::{Jarvis, JarvisConfig, LLMConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure Jarvis with OMEN backend
    let config = JarvisConfig {
        llm: LLMConfig {
            provider: "omen".to_string(),
            base_url: "http://localhost:8080/v1".to_string(),
            api_key: std::env::var("OMEN_API_KEY")?,
            model: "auto".to_string(),
            tags: Some(vec![
                ("source".to_string(), "custom-app".to_string()),
                ("intent".to_string(), "agent".to_string()),
            ]),
        },
        features: Default::default(),
    };

    // Initialize Jarvis
    let jarvis = Jarvis::new(config).await?;

    // Code generation
    let code = jarvis
        .code("Create a Tokio TCP server")
        .await?;
    println!("Generated code:\n{}", code);

    // System operation
    let result = jarvis
        .system("Check Docker container health")
        .await?;
    println!("System check: {}", result);

    // DevOps task
    jarvis
        .devops("Deploy app to Proxmox VM")
        .await?;

    Ok(())
}
```

### Custom Agent with OMEN

```rust
// Custom agent using Jarvis + OMEN
use jarvis_core::Agent;
use serde_json::json;

pub struct HomelabAgent {
    jarvis: Jarvis,
}

impl HomelabAgent {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let jarvis = Jarvis::from_env().await?;
        Ok(Self { jarvis })
    }

    pub async fn maintain(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Step 1: Diagnose system
        let diagnostics = self.jarvis
            .with_intent("system")
            .execute("Run comprehensive system health check")
            .await?;

        // Step 2: Generate maintenance plan
        let plan = self.jarvis
            .with_intent("reason")
            .with_complexity("high")
            .execute(&format!(
                "Based on these diagnostics, create a maintenance plan:\n{}",
                diagnostics
            ))
            .await?;

        // Step 3: Execute maintenance tasks
        for task in parse_tasks(&plan) {
            self.jarvis
                .with_intent("devops")
                .execute(&task)
                .await?;
        }

        Ok(())
    }

    pub async fn deploy_app(
        &self,
        app: &str,
        target: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // OMEN automatically routes each step to optimal model

        // Plan deployment (complex reasoning)
        let plan = self.jarvis
            .with_intent("reason")
            .execute(&format!("Plan deployment of {} to {}", app, target))
            .await?;

        // Generate configs (code generation)
        let configs = self.jarvis
            .with_intent("code")
            .execute(&format!("Generate deployment configs for: {}", plan))
            .await?;

        // Execute deployment (system operations)
        self.jarvis
            .with_intent("devops")
            .execute(&format!("Deploy with configs: {}", configs))
            .await?;

        Ok(())
    }
}

fn parse_tasks(plan: &str) -> Vec<String> {
    // Parse plan into executable tasks
    plan.lines()
        .filter(|l| l.starts_with("- "))
        .map(|l| l.trim_start_matches("- ").to_string())
        .collect()
}
```

## Advanced Configuration

### Intent-Based Routing

```toml
# Fine-tune OMEN routing for specific Jarvis intents

[routing.intents.code_generation]
primary_provider = "ollama"
model = "deepseek-coder:6.7b"
max_tokens = 2048
temperature = 0.2

[routing.intents.code_review]
primary_provider = "anthropic"
model = "claude-3-5-sonnet-20241022"
temperature = 0.3

[routing.intents.shell_script]
primary_provider = "ollama"
model = "codellama:13b-instruct"
temperature = 0.1

[routing.intents.system_diagnostics]
primary_provider = "ollama"
model = "mistral:7b-instruct"
max_tokens = 1024
timeout_ms = 5000

[routing.intents.architecture]
primary_provider = "anthropic"
model = "claude-3-5-sonnet-20241022"
temperature = 0.7
no_local_fallback = true

[routing.intents.troubleshooting]
primary_provider = "anthropic"
model = "claude-3-5-sonnet-20241022"
fallback_providers = ["openai"]
```

### Cost Optimization Strategy

```toml
[budget]
# Set aggressive limits for Jarvis usage
monthly_usd = 50
daily_usd = 2
per_request_max_usd = 0.10

# Alert before hitting limits
alert_threshold = 0.7
alert_email = "admin@homelab.local"

[budget.overrides]
# Allow higher costs for critical intents
troubleshooting = 0.50
architecture = 0.50

[cost_optimization]
# Prefer local models even if slightly lower quality
quality_vs_cost_threshold = 0.8
always_try_local_first = true
cache_expensive_requests = true
cache_ttl_hours = 24
```

### Multi-GPU Homelab Setup

```yaml
# docker-compose.yml - Multi-GPU Ollama for Jarvis
version: '3.8'

services:
  omen:
    image: ghcr.io/ghostkellz/omen:latest
    restart: unless-stopped
    environment:
      OMEN_BIND: "0.0.0.0:8080"
      OMEN_REDIS_URL: "redis://redis:6379"
      OMEN_OLLAMA_ENDPOINTS: "http://ollama-4090:11434,http://ollama-3070:11434"
      OMEN_ANTHROPIC_API_KEY: "${ANTHROPIC_API_KEY}"
      OMEN_ROUTER_PREFER_LOCAL_FOR: "code,system,devops,agent"
      OMEN_BUDGET_MONTHLY_USD: "50"
    ports:
      - "8080:8080"
    depends_on:
      - redis
      - ollama-4090
      - ollama-3070

  ollama-4090:
    image: ollama/ollama:latest
    restart: unless-stopped
    environment:
      CUDA_VISIBLE_DEVICES: "0"
      OLLAMA_NUM_PARALLEL: "4"
      OLLAMA_MAX_LOADED_MODELS: "3"
    volumes:
      - ollama_4090_data:/root/.ollama
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              device_ids: ['0']
              capabilities: [gpu]

  ollama-3070:
    image: ollama/ollama:latest
    restart: unless-stopped
    environment:
      CUDA_VISIBLE_DEVICES: "1"
      OLLAMA_NUM_PARALLEL: "2"
      OLLAMA_MAX_LOADED_MODELS: "2"
    volumes:
      - ollama_3070_data:/root/.ollama
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              device_ids: ['1']
              capabilities: [gpu]

  redis:
    image: redis:7-alpine
    restart: unless-stopped
    command: redis-server --maxmemory 512mb --maxmemory-policy allkeys-lru
    volumes:
      - redis_data:/data

volumes:
  ollama_4090_data:
  ollama_3070_data:
  redis_data:
```

## Monitoring & Metrics

### Track Jarvis Usage

```bash
# View Jarvis-specific metrics in OMEN
curl http://localhost:8080/admin/metrics?source=jarvis | jq

# Example output:
{
  "total_requests": 2341,
  "by_intent": {
    "code": 892,
    "system": 654,
    "devops": 421,
    "reason": 234,
    "agent": 140
  },
  "local_usage_pct": 87.3,
  "cloud_usage_pct": 12.7,
  "total_cost_usd": 1.42,
  "avg_latency_ms": {
    "local": 234,
    "cloud": 1823
  },
  "cache_hit_rate": 34.2
}
```

### Prometheus Dashboard

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'omen-jarvis'
    static_configs:
      - targets: ['localhost:8080']
    metric_relabel_configs:
      - source_labels: [source]
        regex: 'jarvis'
        action: keep
```

Grafana queries:
```promql
# Jarvis request rate
rate(omen_requests_total{source="jarvis"}[5m])

# Cost per intent
sum by (intent) (omen_cost_usd{source="jarvis"})

# Local vs Cloud routing
sum by (provider_type) (omen_requests_total{source="jarvis"})
```

## Troubleshooting

### Slow Responses

```bash
# Check which provider is being used
jarvis --debug code "test function"

# Check OMEN routing decision
curl http://localhost:8080/admin/routing/last?source=jarvis | jq

# Common fixes:

# 1. Ensure local models are loaded
ollama list

# 2. Preload models
ollama pull deepseek-coder:6.7b
ollama run deepseek-coder:6.7b ""

# 3. Reduce timeout for local-only
OMEN_LOCAL_TIMEOUT_MS=5000
```

### Wrong Model Selected

```bash
# Force specific model for testing
jarvis --model "deepseek-coder:6.7b" code "test"

# Check OMEN routing rules
curl http://localhost:8080/admin/routing/rules?intent=code

# Update Jarvis config to provide better hints
[llm.tags]
intent = "code"  # Explicit intent
complexity = "simple"  # Hint for local routing
```

### High Costs

```bash
# Analyze cost breakdown
curl http://localhost:8080/admin/costs?source=jarvis&group_by=intent | jq

# Typical issues:
# - Code reviews going to cloud (expensive)
# - Should use local models more

# Fix: Update routing rules
[routing.intents.code_review]
primary_provider = "ollama"  # Try local first
model = "codellama:13b-instruct"
fallback_provider = "anthropic"
complexity_threshold = "very_high"  # Only use Claude for very complex reviews
```

## Integration with Ghost Stack

### Jarvis + Zeke + OMEN

```bash
# Jarvis for system/DevOps tasks
jarvis deploy "Setup development environment"

# Zeke for coding in editor
zeke ask "Write Rust parser"

# Both use OMEN with optimal routing
# - Jarvis: More system/DevOps intent
# - Zeke: More code completion intent
```

### Jarvis + GhostFlow + OMEN

```yaml
# GhostFlow workflow using Jarvis
nodes:
  - id: plan_deployment
    type: jarvis
    config:
      omen_endpoint: http://localhost:8080/v1
      command: "plan"

  - id: execute_deployment
    type: jarvis
    config:
      command: "deploy"
      input: "{{ nodes.plan_deployment.output }}"
```

### Jarvis + Glyph MCP

```rust
// Jarvis using Glyph for structured tool execution
pub struct EnhancedJarvis {
    jarvis: Jarvis,
    mcp_client: glyph::Client,
}

impl EnhancedJarvis {
    pub async fn execute(&self, task: &str) -> Result<String, Error> {
        // Use Jarvis (via OMEN) for planning
        let plan = self.jarvis.plan(task).await?;

        // Use Glyph MCP for structured tool execution
        for tool in plan.tools {
            self.mcp_client.call_tool(&tool.name, tool.params).await?;
        }

        // Use Jarvis (via OMEN) for synthesis
        self.jarvis.synthesize(&plan.results).await
    }
}
```

## Best Practices

1. **Tag Requests Appropriately**: Help OMEN route correctly with explicit intents
2. **Prefer Local for Simple Tasks**: 80%+ of Jarvis tasks can use local models
3. **Monitor Costs**: Track spending by intent to identify optimization opportunities
4. **Cache Aggressively**: Many Jarvis operations are repetitive
5. **Use Complexity Hints**: Guide OMEN to choose the right model for the task
6. **Preload Models**: Keep frequently-used models loaded in Ollama

## Migration from Direct Provider Usage

```toml
# Before: Direct Anthropic
[llm]
provider = "anthropic"
api_key = "sk-ant-..."
model = "claude-3-5-sonnet-20241022"

# After: OMEN (with intelligent routing)
[llm]
provider = "omen"
base_url = "http://localhost:8080/v1"
api_key = "omen-key"
model = "auto"

# Benefits:
# - 87% cost reduction (local routing)
# - Faster responses for simple tasks
# - Automatic fallback if Ollama down
# - Usage tracking and budgets
```

## Next Steps

1. **Install Jarvis**: Clone and build from [github.com/ghostkellz/jarvis](https://github.com/ghostkellz/jarvis)
2. **Configure OMEN**: Set up routing rules optimized for Jarvis workloads
3. **Pull Ollama Models**: Get DeepSeek Coder, Mistral, Llama3.1, CodeLlama
4. **Test Integration**: Run sample commands and monitor routing decisions
5. **Optimize**: Adjust routing rules based on actual usage patterns

## Resources

- [Jarvis GitHub](https://github.com/ghostkellz/jarvis)
- [OMEN Documentation](https://github.com/ghostkellz/omen)
- [Ollama Models](https://ollama.ai/library)
- [Ghost Stack Integration](./GHOST_INTEGRATIONS.md)

---

**Built with the Ghost Stack**

🦀 **Rust** • 👻 **Ghost Stack** • 🤖 **AI-Powered** • 🏠 **Homelab Native**
