# Changelog

All notable changes to Jarvis will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### 🎉 Major Update (2025-10-15): AI-Powered Arch Linux System Administration

**Complete Ollama + Omen integration with MCP tools for system management!**

**Highlights:**
- ✅ **Hybrid LLM Architecture**: Direct Ollama + Omen intelligent routing
- ✅ **3 MCP Tools**: SystemStatus, PackageManager, Docker+KVM with AI diagnostics
- ✅ **Natural Language Interface**: Parse commands like "diagnose ollama container"
- ✅ **Enhanced Diagnostics**: Network inspection, volume analysis, performance profiling
- ✅ **Full Test Suite**: 15+ integration tests covering all components
- ✅ **Comprehensive Docs**: 30+ usage examples, complete configuration guide

**Total additions:** ~3,500 lines of production-ready Rust code

### Added (2025-10-15)
- **Ghost Stack Integration**: Added Glyph and Omen as Git dependencies
  - Glyph (MCP Protocol): `https://github.com/ghostkellz/glyph`
  - Omen (AI Gateway): `https://github.com/ghostkellz/omen`
- **OmenClient**: Full LLM client implementation (`jarvis-core/src/llm/omen_client.rs`)
  - Intent-based routing (code, system, devops, reason)
  - OpenAI-compatible chat completions
  - Streaming response support
  - Automatic cost budgeting with `OmenConfig`
  - Specialized methods: `code()`, `system()`, `devops()`, `reason()`
- **Config Enhancement**: Extended configuration module
  - `LLMConfig`: Added `omen_enabled`, `omen_base_url`, `omen_api_key`
  - `McpConfig`: New MCP server configuration (transport, address, tools)
  - Environment variable fallbacks for all settings
  - Helper methods: `omen_url()`, `omen_key()`, `use_omen()`
- **Documentation**: Consolidated into `docs/` directory
  - `docs/GHOST_STACK_INTEGRATION.md`: Comprehensive integration guide
  - `docs/GHOSTAI_STACK.md`: Ghost Stack architecture overview
  - `docs/OMEN_INTEGRATION.md`: Omen-specific integration details

### Changed
- Configuration module now supports both legacy providers and Omen routing
- LLM module reorganized with backward compatibility

### Fixed
- Resolved edition mismatch (now using edition = "2024")
- Updated SQLx from 0.7 to 0.8.1 (security vulnerability fixes)

## [0.2.0] - 2025-10-XX

### Added
- All components integrated into main binary
- Full lifecycle management (start/stop)
- Cross-component data sharing
- CLI interface ready
- CI/CD workflow for dual GPU setup
- Assets and logo

### Fixed
- All 3 SQLx vulnerabilities patched
- Build successful with 0 errors

## [0.1.0] - Initial Release

### Added
- Core workspace structure
- jarvis-core, jarvis-agent, jarvis-shell modules
- jarvis-nvim, jarvis-nv, jarvis-ghostflow modules
- Basic LLM routing (Ollama, OpenAI, Claude)
- Blockchain agent foundation
- Memory store with SQLite
- GhostChain gRPC client
- Basic CLI interface

---

## Integration Status

**Current Version**: 0.2.0
**Next Milestone**: 0.3.0 (MVP with Glyph/Omen)

### Completed ✅
- [x] Glyph & Omen dependencies
- [x] OmenClient implementation
- [x] Configuration enhancement
- [x] Documentation consolidated

### Completed ✅ (2025-10-15 Continued)
- [x] **MCP Server Implementation** (`jarvis-core/src/mcp/server.rs`)
  - Glyph ServerBuilder integration with stdio and WebSocket transports
  - Tool registration system using `server().register_tool()`
  - Proper async runtime with `for_stdio()` and `for_websocket()` wrappers
  - LLMRouter injection for AI-powered tools
- [x] **SystemStatusTool** (`jarvis-core/src/mcp/tools.rs:11-71`)
  - MCP Tool trait implementation for system monitoring
  - CPU usage, memory, swap tracking with sysinfo 0.30
  - Verbose mode for detailed metrics
  - Returns structured text output via `CallToolResult`
- [x] **OllamaClient** (`jarvis-core/src/llm/ollama_client.rs`)
  - Direct Ollama API client with chat completions
  - Specialized methods: `code()`, `system()`, `devops()` with domain-specific prompts
  - Health checking and model listing
  - Streaming support with `chat_stream()`
  - Temperature and option controls
- [x] **LLMRouter Enhancement** (`jarvis-core/src/llm/mod.rs`)
  - **Hybrid routing**: Supports both Omen (intelligent) and Ollama (direct)
  - `generate_with_intent()` method for intent-based routing (Code, System, DevOps, Reason)
  - Automatic fallback: Omen → Ollama → error
  - Health checks: `check_ollama_health()`, `list_ollama_models()`
  - Clone support for multi-threaded agent use
- [x] **PackageManagerTool** (`jarvis-core/src/mcp/tools.rs:73-372`)
  - Full Arch Linux package management via MCP
  - Actions: search, info, install, remove, update, list-installed, list-updates
  - Supports: pacman, yay, paru
  - Safety: `confirm=true` required for destructive operations
  - Integrated with LLMRouter for natural language guidance
- [x] **DockerTool** (`jarvis-core/src/mcp/tools.rs:374-924`)
  - **Docker management**: list, ps, inspect, logs, start, stop, restart, stats
  - **AI diagnostics**: `diagnose` action with LLM-powered troubleshooting
  - **Health monitoring**: `health` action with overview and recommendations
  - **KVM/Libvirt support**: vm-list, vm-status, vm-start, vm-stop, vm-info
  - LLM integration for analyzing container logs and VM performance
  - Uses `Intent::DevOps` for infrastructure-focused analysis
- [x] **Build System**
  - Added sysinfo 0.30 dependency to jarvis-core
  - Fixed Glyph API compatibility (protocol imports, server access, error types)
  - Resolved all compilation errors - build successful ✅

### Completed ✅ (2025-10-15 Final)
- [x] **Natural Language Parser** (`jarvis-core/src/nlp/mod.rs`)
  - Rule-based parsing for common patterns (fast, deterministic)
  - LLM-based parsing for complex queries (context-aware)
  - Intent detection: SystemStatus, PackageManagement, DockerManagement, VMManagement, Troubleshooting
  - Command suggestions based on intent
  - Automatic parameter extraction (package names, container names, etc.)
- [x] **Enhanced Docker Diagnostics** (`jarvis-core/src/mcp/tools.rs:942-1220`)
  - `network-inspect`: Network configuration + connectivity testing + LLM analysis
  - `volume-inspect`: Volume usage, orphaned volumes, storage optimization
  - `profile`: 5-second performance profiling with CPU/memory/I/O statistics + LLM recommendations
  - All new actions integrate with LLMRouter for AI-powered insights
- [x] **Comprehensive Documentation**
  - `docs/USAGE_EXAMPLES.md`: Complete guide with 30+ examples
  - `docs/jarvis.toml.example`: Fully annotated configuration template
  - Usage patterns, troubleshooting, integration examples (Python, Shell)
  - Performance tips and advanced workflows
- [x] **Integration Tests** (`jarvis-core/tests/integration_tests.rs`)
  - Ollama connection and completion tests
  - LLMRouter intent routing tests
  - CommandParser rule-based and LLM-based tests
  - MCP tool interface validation
  - Full integration scenarios (NLP → Tool → LLM)
  - Performance benchmarks (parsing < 100μs)

### In Progress 🚧
- [ ] Interactive shell mode with history
- [ ] CLI commands for MCP server management

### Planned 📋
- [ ] Structured logging and error handling
- [ ] Web UI for MCP server
- [ ] Plugin system for custom tools
- [ ] Prometheus metrics export

---

**Note**: For detailed integration guides, see `docs/` directory.
