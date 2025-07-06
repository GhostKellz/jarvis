/*!
 * JARVIS-NV: NVIDIA-Accelerated AI Agent for GhostChain Nodes
 * 
 * A GPU-accelerated daemon designed to assist and monitor ghostchain and ghostplane nodes.
 * Runs alongside or inside blockchain nodes, providing real-time insight, telemetry, 
 * and AI-enhanced operations leveraging NVIDIA CUDA and container runtime.
 * 
 * This is separate from jarvisd (the main AI daemon) and is specialized for:
 * - Blockchain node integration
 * - GPU-accelerated tasks
 * - High-performance environments
 * - Web5 stack integration (QUIC/IPv6/HTTP3)
 * - ZVM and GhostChain native operations
 */

use anyhow::{Context, Result};
use clap::{Arg, Command};
use std::{path::PathBuf, sync::Arc};
use tokio::signal;
use tracing::{info, warn, error, debug};

mod config;
mod gpu;
mod metrics;
mod node;
mod bridge;
mod agent;
mod nvcore;
mod web5;

use config::JarvisNvConfig;
use gpu::GpuManager;
use metrics::MetricsCollector;
use node::NodeManager;
use bridge::GhostBridge;
use agent::NvAgent;

/// Main JARVIS-NV application state
pub struct JarvisNv {
    config: Arc<JarvisNvConfig>,
    gpu_manager: Arc<GpuManager>,
    metrics_collector: Arc<MetricsCollector>,
    node_manager: Arc<NodeManager>,
    ghost_bridge: Arc<GhostBridge>,
    agent: Arc<NvAgent>,
}

impl JarvisNv {
    /// Initialize JARVIS-NV with configuration
    pub async fn new(config_path: Option<PathBuf>) -> Result<Self> {
        info!("🚀 Initializing JARVIS-NV (NVIDIA-Accelerated AI Agent)");

        // Load configuration
        let config = Arc::new(
            JarvisNvConfig::load(config_path.as_deref()).await
                .context("Failed to load configuration")?
        );

        // Initialize GPU manager
        let gpu_manager = Arc::new(
            GpuManager::new(&config.gpu).await
                .context("Failed to initialize GPU manager")?
        );

        // Initialize metrics collector
        let metrics_collector = Arc::new(
            MetricsCollector::new(&config.metrics, gpu_manager.clone()).await?
        );

        // Initialize node manager (GhostChain/ZVM integration)
        let node_manager = Arc::new(
            NodeManager::new(&config.node, &config.web5).await
                .context("Failed to initialize node manager")?
        );

        // Initialize GhostBridge (gRPC/QUIC communication)
        let ghost_bridge = Arc::new(
            GhostBridge::new(&config.bridge, &config.web5).await
                .context("Failed to initialize GhostBridge")?
        );

        // Initialize AI agent
        let agent = Arc::new(
            NvAgent::new(
                &config.agent,
                gpu_manager.clone(),
                metrics_collector.clone(),
                node_manager.clone(),
                ghost_bridge.clone(),
            ).await.context("Failed to initialize NV Agent")?
        );

        Ok(Self {
            config,
            gpu_manager,
            metrics_collector,
            node_manager,
            ghost_bridge,
            agent,
        })
    }

    /// Start JARVIS-NV services
    pub async fn start(&self) -> Result<()> {
        info!("🧠 Starting JARVIS-NV services...");

        // Start GPU manager
        self.gpu_manager.start().await
            .context("Failed to start GPU manager")?;

        // Start metrics collection
        self.metrics_collector.start().await
            .context("Failed to start metrics collector")?;

        // Start node monitoring
        self.node_manager.start().await
            .context("Failed to start node manager")?;

        // Start GhostBridge communication
        self.ghost_bridge.start().await
            .context("Failed to start GhostBridge")?;

        // Start AI agent
        self.agent.start().await
            .context("Failed to start NV Agent")?;

        info!("✅ JARVIS-NV services started successfully");

        // Wait for shutdown signal
        self.wait_for_shutdown().await;

        // Graceful shutdown
        self.shutdown().await
    }

    /// Wait for shutdown signal
    async fn wait_for_shutdown(&self) {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Received SIGINT, shutting down gracefully...");
            }
            _ = async {
                let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
                sigterm.recv().await;
            } => {
                info!("Received SIGTERM, shutting down gracefully...");
            }
        }
    }

    /// Graceful shutdown
    async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down JARVIS-NV services...");

        // Stop AI agent
        if let Err(e) = self.agent.stop().await {
            error!("Error stopping NV Agent: {}", e);
        }

        // Stop GhostBridge
        if let Err(e) = self.ghost_bridge.stop().await {
            error!("Error stopping GhostBridge: {}", e);
        }

        // Stop node manager
        if let Err(e) = self.node_manager.stop().await {
            error!("Error stopping node manager: {}", e);
        }

        // Stop metrics collector
        if let Err(e) = self.metrics_collector.stop().await {
            error!("Error stopping metrics collector: {}", e);
        }

        // Stop GPU manager
        if let Err(e) = self.gpu_manager.stop().await {
            error!("Error stopping GPU manager: {}", e);
        }

        info!("✅ JARVIS-NV shutdown complete");
        Ok(())
    }

    /// Get system status
    pub async fn get_status(&self) -> Result<serde_json::Value> {
        let gpu_status = self.gpu_manager.get_status().await?;
        let node_status = self.node_manager.get_status().await?;
        let metrics_status = self.metrics_collector.get_status().await?;
        let bridge_status = self.ghost_bridge.get_status().await?;
        let agent_status = self.agent.get_status().await?;

        Ok(serde_json::json!({
            "jarvis_nv": {
                "version": env!("CARGO_PKG_VERSION"),
                "status": "running",
                "uptime": metrics_status["uptime"],
                "gpu": gpu_status,
                "node": node_status,
                "metrics": metrics_status,
                "bridge": bridge_status,
                "agent": agent_status
            }
        }))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "jarvis_nv=info,jarvis_core=info".to_string())
        )
        .init();

    let matches = Command::new("jarvis-nv")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Christopher Kelley <ckelley@ghostkellz.sh>")
        .about("NVIDIA-Accelerated AI Agent for GhostChain Nodes")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
        )
        .arg(
            Arg::new("gpu")
                .short('g')
                .long("gpu")
                .value_name("ID")
                .help("GPU device ID to use (default: 0)")
                .default_value("0")
        )
        .arg(
            Arg::new("node-url")
                .short('n')
                .long("node-url")
                .value_name("URL")
                .help("GhostChain node URL")
                .default_value("http://localhost:8545")
        )
        .subcommand(
            Command::new("start")
                .about("Start JARVIS-NV daemon")
        )
        .subcommand(
            Command::new("status")
                .about("Show system status")
        )
        .subcommand(
            Command::new("gpu-info")
                .about("Show GPU information")
        )
        .subcommand(
            Command::new("node-info")
                .about("Show node information")
        )
        .subcommand(
            Command::new("benchmark")
                .about("Run GPU benchmark")
        )
        .get_matches();

    let config_path = matches.get_one::<String>("config").map(PathBuf::from);

    match matches.subcommand() {
        Some(("start", _)) => {
            info!("🚀 Starting JARVIS-NV daemon...");
            let jarvis_nv = JarvisNv::new(config_path).await?;
            jarvis_nv.start().await?;
        }

        Some(("status", _)) => {
            info!("📊 Getting JARVIS-NV status...");
            let jarvis_nv = JarvisNv::new(config_path).await?;
            let status = jarvis_nv.get_status().await?;
            println!("{}", serde_json::to_string_pretty(&status)?);
        }

        Some(("gpu-info", _)) => {
            info!("🖥️ Getting GPU information...");
            let jarvis_nv = JarvisNv::new(config_path).await?;
            let gpu_info = jarvis_nv.gpu_manager.get_detailed_info().await?;
            println!("{}", serde_json::to_string_pretty(&gpu_info)?);
        }

        Some(("node-info", _)) => {
            info!("🔗 Getting node information...");
            let jarvis_nv = JarvisNv::new(config_path).await?;
            let node_info = jarvis_nv.node_manager.get_detailed_info().await?;
            println!("{}", serde_json::to_string_pretty(&node_info)?);
        }

        Some(("benchmark", _)) => {
            info!("🏃 Running GPU benchmark...");
            let jarvis_nv = JarvisNv::new(config_path).await?;
            let benchmark_results = jarvis_nv.gpu_manager.run_benchmark().await?;
            println!("{}", serde_json::to_string_pretty(&benchmark_results)?);
        }

        _ => {
            // Default: start daemon
            info!("🚀 Starting JARVIS-NV daemon (default)...");
            let jarvis_nv = JarvisNv::new(config_path).await?;
            jarvis_nv.start().await?;
        }
    }

    Ok(())
}
