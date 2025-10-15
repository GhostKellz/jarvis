# Jarvis + Glyph + Omen Integration Guide

Complete guide for integrating Jarvis with Glyph (MCP) and Omen (AI Gateway) as Git crate dependencies from the Ghost Stack ecosystem.

## Overview

**Jarvis** leverages two critical Ghost Stack components:

1. **Glyph** - MCP (Model Context Protocol) implementation
   - Structured tool execution
   - Session management
   - Multi-transport support (stdio, WebSocket, HTTP)
   - Policy gates and audit logging

2. **Omen** - AI Gateway with smart routing
   - OpenAI-compatible API
   - Multi-provider support (Claude, GPT, Ollama, Gemini, etc.)
   - Intent-based routing
   - Cost optimization

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                       Jarvis CLI                         │
│         (System Agent, DevOps, Crypto Monitor)           │
└──────────────┬────────────────────────┬──────────────────┘
               │                        │
               │ MCP Tools              │ LLM Requests
               │                        │
       ┌───────▼────────┐      ┌────────▼─────────┐
       │     Glyph      │      │      Omen         │
       │  (MCP Server)  │      │  (AI Gateway)     │
       │                │      │                   │
       │ - Tool Registry│      │ - Smart Router    │
       │ - Sessions     │      │ - Multi-Provider  │
       │ - Audit Log    │      │ - Cost Tracking   │
       └────────────────┘      └──────┬────────────┘
                                      │
                        ┌─────────────┴──────────────┐
                        │                            │
                   ┌────▼─────┐              ┌──────▼──────┐
                   │  Claude  │              │   Ollama    │
                   │   GPT    │              │  (Local)    │
                   │  Gemini  │              │ 4090/3070   │
                   └──────────┘              └─────────────┘
```

## Installation

### 1. Add Git Crate Dependencies

Update `jarvis-core/Cargo.toml`:

```toml
[dependencies]
# Ghost Stack Integration
glyph = { git = "https://github.com/ghostkellz/glyph", tag = "v0.1.0" }
omen = { git = "https://github.com/ghostkellz/omen" }

# Required by Glyph/Omen
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
reqwest = { version = "0.11", features = ["json", "stream"] }
anyhow = "1.0"
tracing = "0.1"
```

### 2. Verify Dependencies

```bash
cd /data/projects/jarvis
cargo check
```

## Integration Pattern 1: Jarvis as MCP Client (Calling Omen via Glyph)

Jarvis can act as an MCP client to connect to Omen for LLM routing.

### Implementation

Create `jarvis-core/src/llm/omen_client.rs`:

```rust
use anyhow::Result;
use glyph::client::Client as GlyphClient;
use omen::types::{ChatCompletionRequest, ChatMessage, MessageContent};
use serde_json::json;

pub struct OmenClient {
    // Direct HTTP client for OpenAI-compatible API
    http_client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl OmenClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url,
            api_key,
        }
    }

    pub async fn from_config(config: &crate::config::Config) -> Result<Self> {
        let base_url = config.llm.base_url.clone()
            .unwrap_or_else(|| "http://localhost:8080/v1".to_string());
        let api_key = config.llm.api_key.clone()
            .unwrap_or_else(|| std::env::var("OMEN_API_KEY").unwrap_or_default());

        Ok(Self::new(base_url, api_key))
    }

    /// Send a chat completion request to Omen
    pub async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        intent: Option<&str>,
        stream: bool,
    ) -> Result<omen::types::ChatCompletionResponse> {
        let mut tags = std::collections::HashMap::new();
        tags.insert("source".to_string(), "jarvis".to_string());

        if let Some(intent) = intent {
            tags.insert("intent".to_string(), intent.to_string());
        }

        let request = ChatCompletionRequest {
            model: "auto".to_string(), // Let Omen choose
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            tools: None,
            tool_choice: None,
            tags: Some(tags),
            omen: Some(omen::types::OmenConfig {
                strategy: Some("single".to_string()),
                budget_usd: Some(0.10),
                max_latency_ms: Some(5000),
                ..Default::default()
            }),
        };

        let url = format!("{}/chat/completions", self.base_url);
        let response = self.http_client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Omen API error: {}", error_text);
        }

        let result = response.json().await?;
        Ok(result)
    }

    /// Helper: Send a simple text prompt
    pub async fn complete(&self, prompt: &str, intent: Option<&str>) -> Result<String> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text(prompt.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];

        let response = self.chat_completion(messages, intent, false).await?;

        Ok(response.choices
            .first()
            .map(|c| c.message.content.to_string())
            .unwrap_or_default())
    }

    /// Streaming completion (for interactive mode)
    pub async fn complete_stream(
        &self,
        prompt: &str,
        intent: Option<&str>,
    ) -> Result<impl futures::Stream<Item = Result<String>>> {
        use futures::stream::StreamExt;

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text(prompt.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];

        let mut tags = std::collections::HashMap::new();
        tags.insert("source".to_string(), "jarvis".to_string());
        if let Some(intent) = intent {
            tags.insert("intent".to_string(), intent.to_string());
        }

        let request = ChatCompletionRequest {
            model: "auto".to_string(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: true,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            tools: None,
            tool_choice: None,
            tags: Some(tags),
            omen: None,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let response = self.http_client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        let stream = response.bytes_stream()
            .map(|chunk| {
                chunk.map_err(Into::into)
                    .and_then(|bytes| {
                        let text = String::from_utf8(bytes.to_vec())?;
                        Ok(text)
                    })
            });

        Ok(stream)
    }
}
```

### Usage in Jarvis

Update `jarvis-core/src/lib.rs`:

```rust
pub mod llm;

use llm::omen_client::OmenClient;

pub struct Jarvis {
    config: config::Config,
    omen: OmenClient,
}

impl Jarvis {
    pub async fn new(config: config::Config) -> Result<Self> {
        let omen = OmenClient::from_config(&config).await?;

        Ok(Self { config, omen })
    }

    /// System command with AI reasoning
    pub async fn system_command(&self, command: &str) -> Result<String> {
        let prompt = format!(
            "As a Linux system administrator, execute this command safely: {}\n\
             Explain what you will do, then provide the exact command.",
            command
        );

        self.omen.complete(&prompt, Some("system")).await
    }

    /// DevOps task
    pub async fn devops_task(&self, task: &str) -> Result<String> {
        self.omen.complete(task, Some("devops")).await
    }

    /// Code generation
    pub async fn code_generation(&self, request: &str) -> Result<String> {
        self.omen.complete(request, Some("code")).await
    }

    /// Complex reasoning task
    pub async fn reason(&self, question: &str) -> Result<String> {
        self.omen.complete(question, Some("reason")).await
    }
}
```

## Integration Pattern 2: Jarvis as MCP Server (Hosting Tools via Glyph)

Jarvis can host its own MCP tools that other agents (Zeke, GhostFlow) can call.

### Implementation

Create `jarvis-core/src/mcp/server.rs`:

```rust
use anyhow::Result;
use async_trait::async_trait;
use glyph::server::{Server, ServerBuilder, Tool, ToolInputSchema, CallToolResult};
use glyph::protocol::types::{TextContent, CallToolRequest};
use serde_json::{json, Value};

/// Jarvis tool: Check system status
pub struct SystemStatusTool;

#[async_trait]
impl Tool for SystemStatusTool {
    fn name(&self) -> &str {
        "jarvis_system_status"
    }

    fn description(&self) -> &str {
        "Check Linux system status (CPU, memory, disk, services)"
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some(json!({
                "verbose": {
                    "type": "boolean",
                    "description": "Include detailed metrics"
                },
                "services": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Optional list of services to check"
                }
            })),
            required: Some(vec![]),
        }
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let verbose = args.as_ref()
            .and_then(|v| v.get("verbose"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Use sysinfo crate to gather system stats
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();

        let mut output = String::new();
        output.push_str(&format!("CPU Usage: {:.2}%\n", sys.global_cpu_info().cpu_usage()));
        output.push_str(&format!("Memory: {} / {} MB\n",
            sys.used_memory() / 1024 / 1024,
            sys.total_memory() / 1024 / 1024));

        if verbose {
            output.push_str(&format!("Swap: {} / {} MB\n",
                sys.used_swap() / 1024 / 1024,
                sys.total_swap() / 1024 / 1024));
            output.push_str(&format!("Processes: {}\n", sys.processes().len()));
        }

        Ok(CallToolResult {
            content: vec![TextContent::text(output)],
            is_error: Some(false),
        })
    }
}

/// Jarvis tool: Execute pacman/yay command
pub struct PackageManagerTool;

#[async_trait]
impl Tool for PackageManagerTool {
    fn name(&self) -> &str {
        "jarvis_package"
    }

    fn description(&self) -> &str {
        "Manage Arch Linux packages (install, remove, update, search)"
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some(json!({
                "action": {
                    "type": "string",
                    "enum": ["install", "remove", "update", "search", "info"],
                    "description": "Package management action"
                },
                "packages": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Package names"
                },
                "aur": {
                    "type": "boolean",
                    "description": "Use AUR helper (yay)"
                }
            })),
            required: Some(vec!["action".to_string()]),
        }
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let args = args.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action'"))?;

        let packages: Vec<String> = args.get("packages")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect())
            .unwrap_or_default();

        let use_aur = args.get("aur")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let cmd = if use_aur { "yay" } else { "sudo pacman" };

        let command = match action {
            "install" => format!("{} -S {}", cmd, packages.join(" ")),
            "remove" => format!("{} -R {}", cmd, packages.join(" ")),
            "update" => format!("{} -Syu", cmd),
            "search" => format!("{} -Ss {}", cmd, packages.join(" ")),
            "info" => format!("{} -Si {}", cmd, packages.join(" ")),
            _ => return Err(anyhow::anyhow!("Invalid action")),
        };

        Ok(CallToolResult {
            content: vec![TextContent::text(format!(
                "Would execute: {}\n(Dry-run mode - implement actual execution with safety checks)",
                command
            ))],
            is_error: Some(false),
        })
    }
}

/// Jarvis tool: Docker management
pub struct DockerTool;

#[async_trait]
impl Tool for DockerTool {
    fn name(&self) -> &str {
        "jarvis_docker"
    }

    fn description(&self) -> &str {
        "Manage Docker containers and images"
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some(json!({
                "action": {
                    "type": "string",
                    "enum": ["ps", "logs", "start", "stop", "restart", "inspect"],
                    "description": "Docker action"
                },
                "container": {
                    "type": "string",
                    "description": "Container name or ID"
                },
                "follow": {
                    "type": "boolean",
                    "description": "Follow logs (for 'logs' action)"
                }
            })),
            required: Some(vec!["action".to_string()]),
        }
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let args = args.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action'"))?;

        let container = args.get("container")
            .and_then(|v| v.as_str());

        let command = match action {
            "ps" => "docker ps".to_string(),
            "logs" if container.is_some() => {
                let follow = args.get("follow").and_then(|v| v.as_bool()).unwrap_or(false);
                let flag = if follow { "-f" } else { "" };
                format!("docker logs {} {}", flag, container.unwrap())
            },
            "start" | "stop" | "restart" | "inspect" if container.is_some() => {
                format!("docker {} {}", action, container.unwrap())
            },
            _ => return Err(anyhow::anyhow!("Invalid action or missing container")),
        };

        // TODO: Actually execute the command safely
        Ok(CallToolResult {
            content: vec![TextContent::text(format!(
                "Would execute: {}\n(Implement actual execution)",
                command
            ))],
            is_error: Some(false),
        })
    }
}

/// Build and run Jarvis MCP server
pub async fn run_mcp_server(transport: &str, address: Option<&str>) -> Result<()> {
    let mut builder = ServerBuilder::new()
        .name("jarvis")
        .version(env!("CARGO_PKG_VERSION"));

    // Configure transport
    builder = match transport {
        "stdio" => builder.transport_stdio(),
        "ws" | "websocket" => {
            let addr = address.unwrap_or("127.0.0.1:7332");
            builder.transport_ws(addr)
        },
        "http" => {
            let addr = address.unwrap_or("127.0.0.1:7332");
            builder.transport_http(addr)
        },
        _ => return Err(anyhow::anyhow!("Unsupported transport: {}", transport)),
    };

    let mut server = builder.build().await?;

    // Register Jarvis tools
    server.register_tool(Box::new(SystemStatusTool));
    server.register_tool(Box::new(PackageManagerTool));
    server.register_tool(Box::new(DockerTool));

    tracing::info!("Jarvis MCP server starting on {}", transport);
    server.run().await?;

    Ok(())
}
```

### Running MCP Server

Add to `src/bin/jarvisd.rs`:

```rust
use clap::{Parser, Subcommand};
use jarvis_core::mcp::server::run_mcp_server;

#[derive(Parser)]
#[command(name = "jarvisd")]
#[command(about = "Jarvis MCP Server Daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start MCP server
    Serve {
        /// Transport: stdio, ws, http
        #[arg(short, long, default_value = "ws")]
        transport: String,

        /// Address (for ws/http)
        #[arg(short, long)]
        address: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { transport, address } => {
            run_mcp_server(&transport, address.as_deref()).await?;
        }
    }

    Ok(())
}
```

## Integration Pattern 3: Hybrid Mode (Both Client and Server)

Jarvis can simultaneously:
1. **Consume Omen** for LLM reasoning
2. **Host MCP tools** via Glyph for other agents

### Architecture

```rust
// jarvis-core/src/lib.rs
pub struct JarvisRuntime {
    // LLM client (to Omen)
    omen: OmenClient,

    // MCP server (hosting tools)
    mcp_server: Option<tokio::task::JoinHandle<()>>,

    config: Config,
}

impl JarvisRuntime {
    pub async fn new(config: Config) -> Result<Self> {
        let omen = OmenClient::from_config(&config).await?;

        Ok(Self {
            omen,
            mcp_server: None,
            config,
        })
    }

    /// Start MCP server in background
    pub async fn start_mcp_server(&mut self) -> Result<()> {
        let transport = self.config.mcp.transport.clone();
        let address = self.config.mcp.address.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) = run_mcp_server(&transport, address.as_deref()).await {
                tracing::error!("MCP server error: {}", e);
            }
        });

        self.mcp_server = Some(handle);
        Ok(())
    }

    /// Execute a task with Omen LLM + Jarvis tools
    pub async fn execute_task(&self, task: &str) -> Result<String> {
        // Use Omen to reason about the task
        let plan = self.omen.complete(
            &format!("Plan how to execute: {}", task),
            Some("agent")
        ).await?;

        // Execute the plan using local tools
        // (This would call back into Jarvis tools via MCP)

        Ok(plan)
    }
}
```

## Configuration

### jarvis.toml

```toml
[llm]
provider = "omen"
base_url = "http://localhost:8080/v1"
api_key = "env:OMEN_API_KEY"
model = "auto"

[llm.routing]
# Intent hints for Omen
code_intent = "code"
devops_intent = "devops"
system_intent = "system"
reason_intent = "reason"

[llm.tags]
source = "jarvis"
project = "homelab"

[mcp]
# Jarvis MCP server settings
enabled = true
transport = "ws"
address = "127.0.0.1:7332"

[mcp.tools]
# Enable/disable specific tools
system_status = true
package_manager = true
docker = true
proxmox = true
git = true

[features]
code_copilot = true
devops_helper = true
system_companion = true
crypto_monitor = true
```

### Environment Variables

```bash
# ~/.bashrc or ~/.zshrc
export OMEN_API_KEY="your-omen-key"
export JARVIS_CONFIG_PATH="$HOME/.config/jarvis/jarvis.toml"
export JARVIS_MCP_ENABLED="true"
```

## Usage Examples

### Example 1: System Status via Omen

```bash
# Jarvis uses Omen for reasoning
jarvis system status

# Behind the scenes:
# 1. Jarvis sends to Omen with intent="system"
# 2. Omen routes to local Ollama (Mistral)
# 3. Response returned to Jarvis
# 4. Jarvis executes system commands
```

### Example 2: Code Generation via Omen

```bash
jarvis code "Create async HTTP server in Rust"

# Behind the scenes:
# 1. Jarvis sends to Omen with intent="code"
# 2. Omen routes to local Ollama (DeepSeek-Coder)
# 3. Code generated and returned
```

### Example 3: Other Agents Calling Jarvis Tools

```bash
# From Zeke or GhostFlow via MCP
curl -X POST http://localhost:7332/mcp/tools/call \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "jarvis_system_status",
    "arguments": {"verbose": true}
  }'
```

## Deployment with Docker Compose

```yaml
version: '3.8'

services:
  omen:
    image: ghcr.io/ghostkellz/omen:latest
    restart: unless-stopped
    environment:
      OMEN_BIND: "0.0.0.0:8080"
      OMEN_REDIS_URL: "redis://redis:6379"
      OMEN_OLLAMA_ENDPOINTS: "http://ollama:11434"
      OMEN_ANTHROPIC_API_KEY: "${ANTHROPIC_API_KEY}"
      OMEN_ROUTER_PREFER_LOCAL_FOR: "code,system,devops"
    ports:
      - "8080:8080"
    depends_on:
      - redis
      - ollama

  jarvis:
    build: .
    restart: unless-stopped
    environment:
      OMEN_API_KEY: "jarvis-agent-key"
      JARVIS_LLM_BASE_URL: "http://omen:8080/v1"
      JARVIS_MCP_ENABLED: "true"
      JARVIS_MCP_ADDRESS: "0.0.0.0:7332"
    ports:
      - "7332:7332"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - ./config:/app/config
    depends_on:
      - omen

  ollama:
    image: ollama/ollama:latest
    restart: unless-stopped
    volumes:
      - ollama_data:/root/.ollama
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu]

  redis:
    image: redis:7-alpine
    restart: unless-stopped

volumes:
  ollama_data:
```

## Testing Integration

### Test Omen Connection

```rust
// tests/test_omen_integration.rs
use jarvis_core::llm::omen_client::OmenClient;

#[tokio::test]
async fn test_omen_connection() {
    let client = OmenClient::new(
        "http://localhost:8080/v1".to_string(),
        "test-key".to_string()
    );

    let result = client.complete("Hello, Omen!", Some("agent")).await;
    assert!(result.is_ok());
}
```

### Test MCP Server

```bash
# Start Jarvis MCP server
cargo run --bin jarvisd -- serve --transport ws --address 127.0.0.1:7332

# Test with Glyph test client
cd /data/projects/glyph
cargo run --example test_client
```

## Roadmap

### Phase 1: Basic Integration (Week 1-2)
- [x] Add Glyph and Omen as Git dependencies
- [ ] Implement OmenClient for LLM requests
- [ ] Configure intent-based routing
- [ ] Test basic chat completions

### Phase 2: MCP Server (Week 3-4)
- [ ] Implement Jarvis MCP tools (system, package, docker)
- [ ] Add Glyph server runtime
- [ ] Test tool invocation from external clients
- [ ] Add session management

### Phase 3: Advanced Features (Week 5-6)
- [ ] Tool calling support (Omen → Jarvis tools)
- [ ] Streaming responses for interactive mode
- [ ] Policy gates for safe command execution
- [ ] Audit logging integration

### Phase 4: Production (Week 7-8)
- [ ] Docker deployment
- [ ] Monitoring and metrics
- [ ] GhostFlow integration
- [ ] Documentation and examples

## Best Practices

1. **Intent Tagging**: Always tag requests with appropriate intent (code, system, devops, reason)
2. **Cost Optimization**: Use `budget_usd` in OmenConfig to prevent overspending
3. **Local-First**: Prefer local Ollama for simple tasks
4. **Tool Safety**: Implement approval gates for destructive operations
5. **Session Management**: Use Glyph sessions for multi-turn conversations
6. **Error Handling**: Handle provider failures gracefully with fallbacks

## Resources

- [Glyph GitHub](https://github.com/ghostkellz/glyph)
- [Omen GitHub](https://github.com/ghostkellz/omen)
- [MCP Specification](https://modelcontextprotocol.io)
- [Ghost Stack Documentation](../GHOSTAI_STACK.md)

---

**Built with the Ghost Stack**

🦀 **Rust** • 👻 **Ghost Stack** • 🤖 **AI-Powered** • 🏠 **Homelab Native**
