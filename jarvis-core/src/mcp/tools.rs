//! Jarvis MCP Tools

use async_trait::async_trait;
use glyph::server::Tool;
use glyph::protocol::{ToolInputSchema, CallToolResult, Content};
use serde_json::{json, Value};
use sysinfo::System;
use std::collections::HashMap;
use tokio::process::Command;

/// System status tool
pub struct SystemStatusTool;

#[async_trait]
impl Tool for SystemStatusTool {
    fn name(&self) -> &str {
        "jarvis_system_status"
    }

    fn description(&self) -> Option<&str> {
        Some("Check Linux system status (CPU, memory, disk, processes)")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "verbose".to_string(),
            json!({
                "type": "boolean",
                "description": "Include detailed metrics",
                "default": false
            })
        );

        ToolInputSchema::object()
            .with_properties(properties)
            .with_required(vec![])
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult, glyph::Error> {
        let verbose = args.as_ref()
            .and_then(|v| v.get("verbose"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut sys = System::new_all();
        sys.refresh_all();

        let mut output = String::new();
        output.push_str("=== Jarvis System Status ===\n\n");

        // CPU
        output.push_str(&format!("CPU Usage: {:.2}%\n", sys.global_cpu_info().cpu_usage()));
        output.push_str(&format!("CPU Cores: {}\n", sys.cpus().len()));

        // Memory
        let used_gb = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
        let total_gb = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
        output.push_str(&format!("\nMemory: {:.2} GB / {:.2} GB ({:.1}%)\n",
            used_gb, total_gb, (used_gb / total_gb) * 100.0));

        if verbose {
            output.push_str(&format!("\nProcesses: {}\n", sys.processes().len()));
            let swap_used_gb = sys.used_swap() as f64 / 1024.0 / 1024.0 / 1024.0;
            let swap_total_gb = sys.total_swap() as f64 / 1024.0 / 1024.0 / 1024.0;
            output.push_str(&format!("Swap: {:.2} GB / {:.2} GB\n", swap_used_gb, swap_total_gb));
        }

        Ok(CallToolResult::success(vec![Content::text(&output)]))
    }
}

/// Package manager tool for Arch Linux (pacman/yay/paru)
pub struct PackageManagerTool;

#[async_trait]
impl Tool for PackageManagerTool {
    fn name(&self) -> &str {
        "jarvis_package_manager"
    }

    fn description(&self) -> Option<&str> {
        Some("Manage Arch Linux packages (search, info, install, remove, update) with pacman/yay/paru")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "action".to_string(),
            json!({
                "type": "string",
                "description": "Action to perform",
                "enum": ["search", "info", "install", "remove", "update", "list-installed", "list-updates"]
            })
        );
        properties.insert(
            "package".to_string(),
            json!({
                "type": "string",
                "description": "Package name (required for search, info, install, remove)"
            })
        );
        properties.insert(
            "manager".to_string(),
            json!({
                "type": "string",
                "description": "Package manager to use",
                "enum": ["pacman", "yay", "paru"],
                "default": "pacman"
            })
        );
        properties.insert(
            "confirm".to_string(),
            json!({
                "type": "boolean",
                "description": "Skip confirmation prompts (use with caution)",
                "default": false
            })
        );

        ToolInputSchema::object()
            .with_properties(properties)
            .with_required(vec!["action".to_string()])
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult, glyph::Error> {
        let args = args.ok_or_else(|| {
            glyph::Error::ToolExecution("Missing arguments".to_string())
        })?;

        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| glyph::Error::ToolExecution("Missing 'action' parameter".to_string()))?;

        let package = args.get("package").and_then(|v| v.as_str());
        let manager = args.get("manager").and_then(|v| v.as_str()).unwrap_or("pacman");
        let confirm = args.get("confirm").and_then(|v| v.as_bool()).unwrap_or(false);

        let output = match action {
            "search" => {
                let pkg = package.ok_or_else(|| {
                    glyph::Error::ToolExecution("Package name required for search".to_string())
                })?;
                search_package(manager, pkg).await?
            }
            "info" => {
                let pkg = package.ok_or_else(|| {
                    glyph::Error::ToolExecution("Package name required for info".to_string())
                })?;
                package_info(manager, pkg).await?
            }
            "install" => {
                let pkg = package.ok_or_else(|| {
                    glyph::Error::ToolExecution("Package name required for install".to_string())
                })?;
                install_package(manager, pkg, confirm).await?
            }
            "remove" => {
                let pkg = package.ok_or_else(|| {
                    glyph::Error::ToolExecution("Package name required for remove".to_string())
                })?;
                remove_package(manager, pkg, confirm).await?
            }
            "update" => {
                update_system(manager, confirm).await?
            }
            "list-installed" => {
                list_installed_packages(manager).await?
            }
            "list-updates" => {
                list_available_updates(manager).await?
            }
            _ => {
                return Err(glyph::Error::ToolExecution(format!("Unknown action: {}", action)));
            }
        };

        Ok(CallToolResult::success(vec![Content::text(&output)]))
    }
}

// Helper functions for package management

async fn search_package(manager: &str, package: &str) -> Result<String, glyph::Error> {
    let (cmd, args) = match manager {
        "pacman" => ("pacman", vec!["-Ss", package]),
        "yay" | "paru" => (manager, vec!["-Ss", package]),
        _ => return Err(glyph::Error::ToolExecution(format!("Unknown package manager: {}", manager))),
    };

    let output = Command::new(cmd)
        .args(&args)
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to run {}: {}", cmd, e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("Search failed:\n{}", stderr));
    }

    let lines: Vec<&str> = stdout.lines().take(20).collect();
    Ok(format!("=== Package Search: {} ===\n\n{}\n\n(Showing first 20 results)", package, lines.join("\n")))
}

async fn package_info(manager: &str, package: &str) -> Result<String, glyph::Error> {
    let (cmd, args) = match manager {
        "pacman" => ("pacman", vec!["-Si", package]),
        "yay" | "paru" => (manager, vec!["-Si", package]),
        _ => return Err(glyph::Error::ToolExecution(format!("Unknown package manager: {}", manager))),
    };

    let output = Command::new(cmd)
        .args(&args)
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to run {}: {}", cmd, e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("Package info failed:\n{}", stderr));
    }

    Ok(format!("=== Package Info: {} ===\n\n{}", package, stdout))
}

async fn install_package(manager: &str, package: &str, confirm: bool) -> Result<String, glyph::Error> {
    if !confirm {
        return Ok(format!(
            "🚨 Package installation requires confirmation.\n\n\
            To install '{}', run manually:\n\
            $ sudo {} -S {}\n\n\
            Or use confirm=true parameter (use with caution)",
            package, manager, package
        ));
    }

    let (cmd, args) = match manager {
        "pacman" => ("sudo", vec!["pacman", "-S", "--noconfirm", package]),
        "yay" => ("yay", vec!["-S", "--noconfirm", package]),
        "paru" => ("paru", vec!["-S", "--noconfirm", package]),
        _ => return Err(glyph::Error::ToolExecution(format!("Unknown package manager: {}", manager))),
    };

    let output = Command::new(cmd)
        .args(&args)
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to run {}: {}", cmd, e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("Installation failed:\n{}\n{}", stdout, stderr));
    }

    Ok(format!("✅ Successfully installed: {}\n\n{}", package, stdout))
}

async fn remove_package(manager: &str, package: &str, confirm: bool) -> Result<String, glyph::Error> {
    if !confirm {
        return Ok(format!(
            "🚨 Package removal requires confirmation.\n\n\
            To remove '{}', run manually:\n\
            $ sudo {} -R {}\n\n\
            Or use confirm=true parameter (use with caution)",
            package, manager, package
        ));
    }

    let (cmd, args) = match manager {
        "pacman" => ("sudo", vec!["pacman", "-R", "--noconfirm", package]),
        "yay" | "paru" => (manager, vec!["-R", "--noconfirm", package]),
        _ => return Err(glyph::Error::ToolExecution(format!("Unknown package manager: {}", manager))),
    };

    let output = Command::new(cmd)
        .args(&args)
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to run {}: {}", cmd, e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("Removal failed:\n{}\n{}", stdout, stderr));
    }

    Ok(format!("✅ Successfully removed: {}\n\n{}", package, stdout))
}

async fn update_system(manager: &str, confirm: bool) -> Result<String, glyph::Error> {
    if !confirm {
        return Ok(format!(
            "🚨 System update requires confirmation.\n\n\
            To update system, run manually:\n\
            $ sudo {} -Syu\n\n\
            Or use confirm=true parameter (use with caution)",
            manager
        ));
    }

    let (cmd, args) = match manager {
        "pacman" => ("sudo", vec!["pacman", "-Syu", "--noconfirm"]),
        "yay" => ("yay", vec!["-Syu", "--noconfirm"]),
        "paru" => ("paru", vec!["-Syu", "--noconfirm"]),
        _ => return Err(glyph::Error::ToolExecution(format!("Unknown package manager: {}", manager))),
    };

    let output = Command::new(cmd)
        .args(&args)
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to run {}: {}", cmd, e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("System update failed:\n{}\n{}", stdout, stderr));
    }

    Ok(format!("✅ System update complete:\n\n{}", stdout))
}

async fn list_installed_packages(manager: &str) -> Result<String, glyph::Error> {
    let (cmd, args) = match manager {
        "pacman" | "yay" | "paru" => ("pacman", vec!["-Q"]),
        _ => return Err(glyph::Error::ToolExecution(format!("Unknown package manager: {}", manager))),
    };

    let output = Command::new(cmd)
        .args(&args)
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to run {}: {}", cmd, e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.lines().count();

    Ok(format!("=== Installed Packages ===\n\nTotal: {} packages\n\n{}", count, stdout))
}

async fn list_available_updates(manager: &str) -> Result<String, glyph::Error> {
    let (cmd, args) = match manager {
        "pacman" => ("sh", vec!["-c", "checkupdates"]),
        "yay" | "paru" => (manager, vec!["-Qu"]),
        _ => return Err(glyph::Error::ToolExecution(format!("Unknown package manager: {}", manager))),
    };

    let output = Command::new(cmd)
        .args(&args)
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to run {}: {}", cmd, e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if stdout.is_empty() && !output.status.success() {
        return Ok(format!("✅ System is up to date!\n\n{}", stderr));
    }

    let count = stdout.lines().count();
    Ok(format!("=== Available Updates ===\n\n{} packages can be updated:\n\n{}", count, stdout))
}

/// Docker and KVM/Libvirt management tool with LLM diagnostics
pub struct DockerTool {
    llm_router: Option<crate::llm::LLMRouter>,
}

impl DockerTool {
    pub fn new(llm_router: Option<crate::llm::LLMRouter>) -> Self {
        Self { llm_router }
    }

    pub fn without_llm() -> Self {
        Self { llm_router: None }
    }
}

#[async_trait]
impl Tool for DockerTool {
    fn name(&self) -> &str {
        "jarvis_docker"
    }

    fn description(&self) -> Option<&str> {
        Some("Manage and diagnose Docker containers and KVM/libvirt VMs with AI-powered troubleshooting")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "action".to_string(),
            json!({
                "type": "string",
                "description": "Action to perform",
                "enum": [
                    "list", "ps", "inspect", "logs", "start", "stop", "restart", "stats",
                    "diagnose", "health", "network-inspect", "volume-inspect", "profile",
                    "vm-list", "vm-status", "vm-start", "vm-stop", "vm-info"
                ]
            })
        );
        properties.insert(
            "target".to_string(),
            json!({
                "type": "string",
                "description": "Container ID/name or VM name (required for most actions)"
            })
        );
        properties.insert(
            "tail".to_string(),
            json!({
                "type": "integer",
                "description": "Number of log lines to show (for logs action)",
                "default": 50
            })
        );
        properties.insert(
            "llm_assist".to_string(),
            json!({
                "type": "boolean",
                "description": "Use LLM to analyze and provide recommendations",
                "default": true
            })
        );

        ToolInputSchema::object()
            .with_properties(properties)
            .with_required(vec!["action".to_string()])
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult, glyph::Error> {
        let args = args.ok_or_else(|| {
            glyph::Error::ToolExecution("Missing arguments".to_string())
        })?;

        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| glyph::Error::ToolExecution("Missing 'action' parameter".to_string()))?;

        let target = args.get("target").and_then(|v| v.as_str());
        let tail = args.get("tail").and_then(|v| v.as_i64()).unwrap_or(50);
        let llm_assist = args.get("llm_assist").and_then(|v| v.as_bool()).unwrap_or(true);

        let output = match action {
            // Docker commands
            "list" | "ps" => docker_list().await?,
            "inspect" => {
                let container = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("Container name required for inspect".to_string())
                })?;
                docker_inspect(container).await?
            }
            "logs" => {
                let container = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("Container name required for logs".to_string())
                })?;
                docker_logs(container, tail as usize).await?
            }
            "start" => {
                let container = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("Container name required for start".to_string())
                })?;
                docker_start(container).await?
            }
            "stop" => {
                let container = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("Container name required for stop".to_string())
                })?;
                docker_stop(container).await?
            }
            "restart" => {
                let container = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("Container name required for restart".to_string())
                })?;
                docker_restart(container).await?
            }
            "stats" => {
                let container = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("Container name required for stats".to_string())
                })?;
                docker_stats(container).await?
            }
            "diagnose" => {
                let container = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("Container name required for diagnose".to_string())
                })?;
                docker_diagnose(container, &self.llm_router, llm_assist).await?
            }
            "health" => {
                docker_health_overview(&self.llm_router, llm_assist).await?
            }
            "network-inspect" => {
                let container = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("Container name required for network-inspect".to_string())
                })?;
                docker_network_inspect(container, &self.llm_router, llm_assist).await?
            }
            "volume-inspect" => {
                docker_volume_inspect(&self.llm_router, llm_assist).await?
            }
            "profile" => {
                let container = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("Container name required for profile".to_string())
                })?;
                docker_performance_profile(container, &self.llm_router, llm_assist).await?
            }

            // KVM/Libvirt commands
            "vm-list" => vm_list().await?,
            "vm-status" => {
                let vm = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("VM name required for vm-status".to_string())
                })?;
                vm_status(vm).await?
            }
            "vm-start" => {
                let vm = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("VM name required for vm-start".to_string())
                })?;
                vm_start(vm).await?
            }
            "vm-stop" => {
                let vm = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("VM name required for vm-stop".to_string())
                })?;
                vm_stop(vm).await?
            }
            "vm-info" => {
                let vm = target.ok_or_else(|| {
                    glyph::Error::ToolExecution("VM name required for vm-info".to_string())
                })?;
                vm_info(vm, &self.llm_router, llm_assist).await?
            }

            _ => {
                return Err(glyph::Error::ToolExecution(format!("Unknown action: {}", action)));
            }
        };

        Ok(CallToolResult::success(vec![Content::text(&output)]))
    }
}

// Docker helper functions

async fn docker_list() -> Result<String, glyph::Error> {
    let output = Command::new("docker")
        .args(&["ps", "-a", "--format", "table {{.ID}}\\t{{.Names}}\\t{{.Status}}\\t{{.Image}}"])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to run docker ps: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("❌ Docker command failed:\n{}", stderr));
    }

    Ok(format!("=== Docker Containers ===\n\n{}", stdout))
}

async fn docker_inspect(container: &str) -> Result<String, glyph::Error> {
    let output = Command::new("docker")
        .args(&["inspect", container])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to inspect container: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("❌ Inspect failed:\n{}", stderr));
    }

    Ok(format!("=== Container Inspect: {} ===\n\n{}", container, stdout))
}

async fn docker_logs(container: &str, tail: usize) -> Result<String, glyph::Error> {
    let output = Command::new("docker")
        .args(&["logs", "--tail", &tail.to_string(), container])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to get logs: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Docker logs can write to stderr even on success
    let combined = format!("{}{}", stdout, stderr);

    Ok(format!("=== Container Logs: {} (last {} lines) ===\n\n{}", container, tail, combined))
}

async fn docker_start(container: &str) -> Result<String, glyph::Error> {
    let output = Command::new("docker")
        .args(&["start", container])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to start container: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("❌ Start failed:\n{}", stderr));
    }

    Ok(format!("✅ Started container: {}\n\n{}", container, stdout))
}

async fn docker_stop(container: &str) -> Result<String, glyph::Error> {
    let output = Command::new("docker")
        .args(&["stop", container])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to stop container: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("❌ Stop failed:\n{}", stderr));
    }

    Ok(format!("✅ Stopped container: {}\n\n{}", container, stdout))
}

async fn docker_restart(container: &str) -> Result<String, glyph::Error> {
    let output = Command::new("docker")
        .args(&["restart", container])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to restart container: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("❌ Restart failed:\n{}", stderr));
    }

    Ok(format!("✅ Restarted container: {}\n\n{}", container, stdout))
}

async fn docker_stats(container: &str) -> Result<String, glyph::Error> {
    let output = Command::new("docker")
        .args(&["stats", "--no-stream", "--format", "table {{.Container}}\\t{{.CPUPerc}}\\t{{.MemUsage}}\\t{{.MemPerc}}\\t{{.NetIO}}\\t{{.BlockIO}}", container])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to get stats: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("❌ Stats failed:\n{}", stderr));
    }

    Ok(format!("=== Container Stats: {} ===\n\n{}", container, stdout))
}

async fn docker_diagnose(
    container: &str,
    llm_router: &Option<crate::llm::LLMRouter>,
    llm_assist: bool,
) -> Result<String, glyph::Error> {
    // Gather diagnostic information
    let mut diagnostics = String::new();
    diagnostics.push_str(&format!("=== Diagnostic Report: {} ===\n\n", container));

    // Get container status
    let status_output = Command::new("docker")
        .args(&["inspect", "--format", "{{.State.Status}} | {{.State.ExitCode}} | {{.State.Error}}", container])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to get status: {}", e)))?;

    let status = String::from_utf8_lossy(&status_output.stdout);
    diagnostics.push_str(&format!("Status: {}\n", status.trim()));

    // Get recent logs
    let logs_output = Command::new("docker")
        .args(&["logs", "--tail", "20", container])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to get logs: {}", e)))?;

    let logs = String::from_utf8_lossy(&logs_output.stdout);
    let logs_err = String::from_utf8_lossy(&logs_output.stderr);
    let combined_logs = format!("{}{}", logs, logs_err);

    diagnostics.push_str(&format!("\nRecent Logs (last 20 lines):\n{}\n", combined_logs));

    // Get resource usage
    let stats_output = Command::new("docker")
        .args(&["stats", "--no-stream", "--format", "CPU: {{.CPUPerc}} | Memory: {{.MemUsage}} ({{.MemPerc}})", container])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to get stats: {}", e)))?;

    let stats = String::from_utf8_lossy(&stats_output.stdout);
    diagnostics.push_str(&format!("\nResource Usage:\n{}\n", stats.trim()));

    // Use LLM to analyze if available
    if llm_assist {
        if let Some(router) = llm_router {
            diagnostics.push_str("\n=== AI Analysis ===\n\n");

            let prompt = format!(
                "Analyze this Docker container diagnostic information and provide troubleshooting recommendations:\n\n{}",
                diagnostics
            );

            match router.generate_with_intent(&prompt, crate::llm::Intent::DevOps).await {
                Ok(analysis) => {
                    diagnostics.push_str(&analysis);
                    diagnostics.push_str("\n");
                }
                Err(e) => {
                    diagnostics.push_str(&format!("⚠️ LLM analysis unavailable: {}\n", e));
                }
            }
        } else {
            diagnostics.push_str("\n⚠️ LLM not configured. Enable Ollama or Omen for AI-powered diagnostics.\n");
        }
    }

    Ok(diagnostics)
}

async fn docker_health_overview(
    llm_router: &Option<crate::llm::LLMRouter>,
    llm_assist: bool,
) -> Result<String, glyph::Error> {
    let mut report = String::new();
    report.push_str("=== Docker Health Overview ===\n\n");

    // Get all containers
    let ps_output = Command::new("docker")
        .args(&["ps", "-a", "--format", "{{.Names}}|{{.Status}}|{{.Image}}"])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to list containers: {}", e)))?;

    let containers = String::from_utf8_lossy(&ps_output.stdout);

    let mut running = 0;
    let mut stopped = 0;
    let mut unhealthy = 0;

    for line in containers.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            if parts[1].contains("Up") {
                running += 1;
            } else {
                stopped += 1;
            }
            if parts[1].contains("unhealthy") {
                unhealthy += 1;
            }
        }
    }

    report.push_str(&format!("Total Containers: {}\n", running + stopped));
    report.push_str(&format!("Running: {} ✅\n", running));
    report.push_str(&format!("Stopped: {} ⏸️\n", stopped));
    report.push_str(&format!("Unhealthy: {} ❌\n\n", unhealthy));

    // Docker system info
    let info_output = Command::new("docker")
        .args(&["system", "df"])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to get system info: {}", e)))?;

    let disk_usage = String::from_utf8_lossy(&info_output.stdout);
    report.push_str(&format!("Disk Usage:\n{}\n", disk_usage));

    // LLM recommendations
    if llm_assist && unhealthy > 0 {
        if let Some(router) = llm_router {
            report.push_str("\n=== AI Recommendations ===\n\n");

            let prompt = format!(
                "There are {} unhealthy Docker containers. Provide recommendations for troubleshooting and maintaining Docker health.\n\nCurrent state:\n{}",
                unhealthy, report
            );

            match router.generate_with_intent(&prompt, crate::llm::Intent::DevOps).await {
                Ok(recommendations) => {
                    report.push_str(&recommendations);
                    report.push_str("\n");
                }
                Err(e) => {
                    report.push_str(&format!("⚠️ LLM recommendations unavailable: {}\n", e));
                }
            }
        }
    }

    Ok(report)
}

// KVM/Libvirt helper functions

async fn vm_list() -> Result<String, glyph::Error> {
    let output = Command::new("virsh")
        .args(&["list", "--all"])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to run virsh: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("❌ Virsh command failed:\n{}\n\nMake sure libvirt is installed and you have permissions.", stderr));
    }

    Ok(format!("=== KVM Virtual Machines ===\n\n{}", stdout))
}

async fn vm_status(vm: &str) -> Result<String, glyph::Error> {
    let output = Command::new("virsh")
        .args(&["domstate", vm])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to get VM status: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("❌ VM status failed:\n{}", stderr));
    }

    Ok(format!("=== VM Status: {} ===\n\n{}", vm, stdout))
}

async fn vm_start(vm: &str) -> Result<String, glyph::Error> {
    let output = Command::new("virsh")
        .args(&["start", vm])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to start VM: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("❌ VM start failed:\n{}", stderr));
    }

    Ok(format!("✅ Started VM: {}\n\n{}", vm, stdout))
}

async fn vm_stop(vm: &str) -> Result<String, glyph::Error> {
    let output = Command::new("virsh")
        .args(&["shutdown", vm])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to stop VM: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(format!("❌ VM shutdown failed:\n{}", stderr));
    }

    Ok(format!("✅ Shutting down VM: {}\n\n{}", vm, stdout))
}

async fn vm_info(
    vm: &str,
    llm_router: &Option<crate::llm::LLMRouter>,
    llm_assist: bool,
) -> Result<String, glyph::Error> {
    let mut info = String::new();
    info.push_str(&format!("=== VM Information: {} ===\n\n", vm));

    // Get VM info
    let info_output = Command::new("virsh")
        .args(&["dominfo", vm])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to get VM info: {}", e)))?;

    let dominfo = String::from_utf8_lossy(&info_output.stdout);
    info.push_str(&format!("{}\n", dominfo));

    // Get CPU stats
    let cpu_output = Command::new("virsh")
        .args(&["cpu-stats", vm])
        .output()
        .await;

    if let Ok(cpu_output) = cpu_output {
        let cpu_stats = String::from_utf8_lossy(&cpu_output.stdout);
        info.push_str(&format!("\nCPU Stats:\n{}\n", cpu_stats));
    }

    // LLM analysis if requested
    if llm_assist {
        if let Some(router) = llm_router {
            info.push_str("\n=== AI Analysis ===\n\n");

            let prompt = format!(
                "Analyze this KVM virtual machine information and provide optimization recommendations:\n\n{}",
                info
            );

            match router.generate_with_intent(&prompt, crate::llm::Intent::DevOps).await {
                Ok(analysis) => {
                    info.push_str(&analysis);
                    info.push_str("\n");
                }
                Err(e) => {
                    info.push_str(&format!("⚠️ LLM analysis unavailable: {}\n", e));
                }
            }
        }
    }

    Ok(info)
}

// Enhanced diagnostic functions

async fn docker_network_inspect(
    container: &str,
    llm_router: &Option<crate::llm::LLMRouter>,
    llm_assist: bool,
) -> Result<String, glyph::Error> {
    let mut report = String::new();
    report.push_str(&format!("=== Network Diagnostics: {} ===\n\n", container));

    // Get network settings
    let net_output = Command::new("docker")
        .args(&["inspect", "--format", "{{json .NetworkSettings}}", container])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to inspect network: {}", e)))?;

    let network_json = String::from_utf8_lossy(&net_output.stdout);

    // Parse and display key network info
    if let Ok(net_data) = serde_json::from_str::<serde_json::Value>(&network_json) {
        // IP Address
        if let Some(ip) = net_data.get("IPAddress").and_then(|v| v.as_str()) {
            report.push_str(&format!("IP Address: {}\n", ip));
        }

        // Ports
        if let Some(ports) = net_data.get("Ports").and_then(|v| v.as_object()) {
            report.push_str("\nPort Mappings:\n");
            for (port, bindings) in ports {
                if let Some(bindings_array) = bindings.as_array() {
                    for binding in bindings_array {
                        if let Some(host_port) = binding.get("HostPort").and_then(|v| v.as_str()) {
                            report.push_str(&format!("  {} → 0.0.0.0:{}\n", port, host_port));
                        }
                    }
                }
            }
        }

        // Networks
        if let Some(networks) = net_data.get("Networks").and_then(|v| v.as_object()) {
            report.push_str("\nConnected Networks:\n");
            for (name, network) in networks {
                report.push_str(&format!("  {}\n", name));
                if let Some(ip) = network.get("IPAddress").and_then(|v| v.as_str()) {
                    report.push_str(&format!("    IP: {}\n", ip));
                }
                if let Some(gateway) = network.get("Gateway").and_then(|v| v.as_str()) {
                    report.push_str(&format!("    Gateway: {}\n", gateway));
                }
            }
        }
    }

    // Test connectivity
    report.push_str("\nConnectivity Test:\n");
    let ping_output = Command::new("docker")
        .args(&["exec", container, "sh", "-c", "ping -c 1 8.8.8.8 || echo 'Ping failed'"])
        .output()
        .await;

    if let Ok(ping_output) = ping_output {
        let ping_result = String::from_utf8_lossy(&ping_output.stdout);
        if ping_result.contains("1 packets transmitted, 1 received") {
            report.push_str("  ✅ Internet connectivity: OK\n");
        } else {
            report.push_str("  ❌ Internet connectivity: FAILED\n");
        }
    }

    // LLM analysis
    if llm_assist {
        if let Some(router) = llm_router {
            report.push_str("\n=== Network Analysis ===\n\n");

            let prompt = format!(
                "Analyze this Docker container network configuration and identify potential issues:\n\n{}",
                report
            );

            match router.generate_with_intent(&prompt, crate::llm::Intent::DevOps).await {
                Ok(analysis) => {
                    report.push_str(&analysis);
                    report.push_str("\n");
                }
                Err(e) => {
                    report.push_str(&format!("⚠️ Network analysis unavailable: {}\n", e));
                }
            }
        }
    }

    Ok(report)
}

async fn docker_volume_inspect(
    llm_router: &Option<crate::llm::LLMRouter>,
    llm_assist: bool,
) -> Result<String, glyph::Error> {
    let mut report = String::new();
    report.push_str("=== Docker Volume Analysis ===\n\n");

    // List volumes
    let volumes_output = Command::new("docker")
        .args(&["volume", "ls", "--format", "{{.Name}}|{{.Driver}}|{{.Mountpoint}}"])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to list volumes: {}", e)))?;

    let volumes = String::from_utf8_lossy(&volumes_output.stdout);
    let volume_lines: Vec<&str> = volumes.lines().collect();

    report.push_str(&format!("Total Volumes: {}\n\n", volume_lines.len()));

    // Get disk usage
    let df_output = Command::new("docker")
        .args(&["system", "df", "-v"])
        .output()
        .await
        .map_err(|e| glyph::Error::ToolExecution(format!("Failed to get disk usage: {}", e)))?;

    let disk_usage = String::from_utf8_lossy(&df_output.stdout);
    report.push_str(&format!("Disk Usage:\n{}\n", disk_usage));

    // Identify orphaned volumes
    let orphans_output = Command::new("docker")
        .args(&["volume", "ls", "-f", "dangling=true", "--format", "{{.Name}}"])
        .output()
        .await;

    if let Ok(orphans_output) = orphans_output {
        let orphans = String::from_utf8_lossy(&orphans_output.stdout);
        let orphan_count = orphans.lines().count();

        if orphan_count > 0 {
            report.push_str(&format!("\n⚠️ Orphaned Volumes: {}\n", orphan_count));
            report.push_str("Run 'docker volume prune' to clean up\n");
        } else {
            report.push_str("\n✅ No orphaned volumes\n");
        }
    }

    // LLM recommendations
    if llm_assist && volume_lines.len() > 10 {
        if let Some(router) = llm_router {
            report.push_str("\n=== Storage Optimization ===\n\n");

            let prompt = format!(
                "Analyze this Docker volume configuration and provide optimization recommendations:\n\n{}",
                report
            );

            match router.generate_with_intent(&prompt, crate::llm::Intent::DevOps).await {
                Ok(recommendations) => {
                    report.push_str(&recommendations);
                    report.push_str("\n");
                }
                Err(e) => {
                    report.push_str(&format!("⚠️ Storage analysis unavailable: {}\n", e));
                }
            }
        }
    }

    Ok(report)
}

async fn docker_performance_profile(
    container: &str,
    llm_router: &Option<crate::llm::LLMRouter>,
    llm_assist: bool,
) -> Result<String, glyph::Error> {
    let mut report = String::new();
    report.push_str(&format!("=== Performance Profile: {} ===\n\n", container));

    // Collect metrics over 5 seconds
    report.push_str("Collecting metrics (5 second profile)...\n\n");

    let mut cpu_samples = Vec::new();
    let mut mem_samples = Vec::new();

    for i in 0..5 {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let stats_output = Command::new("docker")
            .args(&["stats", "--no-stream", "--format", "{{.CPUPerc}}|{{.MemUsage}}", container])
            .output()
            .await
            .map_err(|e| glyph::Error::ToolExecution(format!("Failed to get stats: {}", e)))?;

        let stats = String::from_utf8_lossy(&stats_output.stdout);
        let parts: Vec<&str> = stats.trim().split('|').collect();

        if parts.len() >= 2 {
            // Parse CPU percentage
            if let Some(cpu_str) = parts[0].strip_suffix('%') {
                if let Ok(cpu) = cpu_str.parse::<f64>() {
                    cpu_samples.push(cpu);
                }
            }

            // Store memory string
            mem_samples.push(parts[1].to_string());
        }

        tracing::debug!("Sample {}: {:?}", i + 1, stats.trim());
    }

    // Calculate statistics
    if !cpu_samples.is_empty() {
        let avg_cpu: f64 = cpu_samples.iter().sum::<f64>() / cpu_samples.len() as f64;
        let max_cpu = cpu_samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_cpu = cpu_samples.iter().cloned().fold(f64::INFINITY, f64::min);

        report.push_str("CPU Usage:\n");
        report.push_str(&format!("  Average: {:.2}%\n", avg_cpu));
        report.push_str(&format!("  Min: {:.2}%\n", min_cpu));
        report.push_str(&format!("  Max: {:.2}%\n", max_cpu));
        report.push_str(&format!("  Variance: {:.2}%\n\n", max_cpu - min_cpu));
    }

    if !mem_samples.is_empty() {
        report.push_str(&format!("Memory Usage:\n  {}\n\n", mem_samples.last().unwrap()));
    }

    // Get process list
    let top_output = Command::new("docker")
        .args(&["top", container])
        .output()
        .await;

    if let Ok(top_output) = top_output {
        let processes = String::from_utf8_lossy(&top_output.stdout);
        report.push_str("Running Processes:\n");
        report.push_str(&processes);
        report.push_str("\n");
    }

    // Get I/O stats
    let io_output = Command::new("docker")
        .args(&["stats", "--no-stream", "--format", "{{.BlockIO}}|{{.NetIO}}", container])
        .output()
        .await;

    if let Ok(io_output) = io_output {
        let io_stats = String::from_utf8_lossy(&io_output.stdout);
        let parts: Vec<&str> = io_stats.trim().split('|').collect();
        if parts.len() >= 2 {
            report.push_str(&format!("I/O Statistics:\n"));
            report.push_str(&format!("  Block I/O: {}\n", parts[0]));
            report.push_str(&format!("  Network I/O: {}\n\n", parts[1]));
        }
    }

    // LLM performance analysis
    if llm_assist {
        if let Some(router) = llm_router {
            report.push_str("=== Performance Analysis ===\n\n");

            let prompt = format!(
                "Analyze this Docker container performance profile and provide optimization recommendations:\n\n{}",
                report
            );

            match router.generate_with_intent(&prompt, crate::llm::Intent::DevOps).await {
                Ok(analysis) => {
                    report.push_str(&analysis);
                    report.push_str("\n");
                }
                Err(e) => {
                    report.push_str(&format!("⚠️ Performance analysis unavailable: {}\n", e));
                }
            }
        }
    }

    Ok(report)
}
