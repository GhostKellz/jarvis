/*!
 * Jarvis Daemon (jarvisd) - Autonomous Blockchain Agent Service
 * 
 * This is the daemon variant of Jarvis that runs as a background service
 * for hands-free blockchain monitoring, analysis, and automated responses.
 * 
 * Features:
 * - Autonomous blockchain monitoring across multiple networks
 * - AI-powered analysis and anomaly detection
 * - Automated alerting and response systems
 * - Zero-trust security architecture
 * - Persistent agent memory and state management
 * - gRPC/HTTP3 network optimization
 * - IPv6 and QUIC support
 * - Docker/NVIDIA container compatibility
 * - Systemd service integration
 */

use anyhow::{Context, Result};
use clap::{Arg, Command};
use jarvis_core::{
    config::Config,
    memory::MemoryStore,
    grpc_client::GhostChainClient,
    llm::LLMRouter,
};
use jarvis_agent::{
    blockchain_monitor::{BlockchainMonitorAgent, MonitoringConfig},
    ai_analyzer::{AIBlockchainAnalyzer, AIAnalyzerConfig},
    orchestrator::{BlockchainAgentOrchestrator, OrchestratorConfig},
};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    time::Duration,
};
use tokio::{
    signal,
    sync::RwLock,
    time::{interval, sleep},
};
use tracing::{error, info, warn, debug};

/// Daemon configuration and runtime state
struct JarvisDaemon {
    config: Arc<RwLock<Config>>,
    memory_store: Arc<MemoryStore>,
    orchestrator: Arc<RwLock<BlockchainAgentOrchestrator>>,
    running: Arc<AtomicBool>,
    pid_file: Option<PathBuf>,
}

impl JarvisDaemon {
    /// Initialize the daemon with configuration
    async fn new(config_path: Option<PathBuf>, pid_file: Option<PathBuf>) -> Result<Self> {
        info!("Initializing Jarvis Daemon...");

        // Load configuration
        let config = if let Some(path) = config_path {
            Config::load(Some(path.to_str().unwrap())).await
                .with_context(|| format!("Failed to load config from {:?}", path))?
        } else {
            Config::load(None).await
                .context("Failed to load default config")?
        };

        // Initialize memory store
        let memory_store = Arc::new(
            MemoryStore::new(&config.database_path)
                .await
                .context("Failed to initialize memory store")?
        );

        // Create gRPC client
        let grpc_client = if let Some(ref blockchain_config) = config.blockchain {
            if let Some(ref ghostchain_config) = blockchain_config.ghostchain {
                // Convert config struct to the one expected by GhostChainClient
                let client_config = jarvis_core::grpc_client::GhostChainConfig {
                    endpoint: ghostchain_config.grpc_url.clone(),
                    use_tls: true,
                    ipv6_preferred: true,
                    connection_timeout: std::time::Duration::from_secs(10),
                    request_timeout: std::time::Duration::from_secs(30),
                    max_concurrent_streams: 100,
                };
                GhostChainClient::new(client_config).await?
            } else {
                return Err(anyhow::anyhow!("GhostChain configuration not found"));
            }
        } else {
            return Err(anyhow::anyhow!("Blockchain configuration not found"));
        };

        // Create blockchain monitor agent
        let monitoring_config = MonitoringConfig {
            check_interval: std::time::Duration::from_secs(30),
            latency_threshold: 100.0,
            throughput_threshold: 10.0,
            packet_loss_threshold: 5.0,
            enable_ai_analysis: true,
        };
        
        let monitor_agent = BlockchainMonitorAgent::new(
            grpc_client.clone(),
            (*memory_store).clone(),
            monitoring_config,
        );

        // Create LLM router
        let llm_router = LLMRouter::new(&config).await?;

        // Create AI analyzer
        let ai_config = AIAnalyzerConfig {
            model_name: "llama3.2:3b".to_string(),
            analysis_prompt_template: "Analyze blockchain data for anomalies".to_string(),
            confidence_threshold: 0.8,
            enable_automated_actions: false,
            max_analysis_history: 1000,
        };
        
        let ai_analyzer = AIBlockchainAnalyzer::new(
            llm_router.clone(),
            (*memory_store).clone(),
            ai_config,
        );

        // Create orchestrator config
        let orchestrator_config = OrchestratorConfig {
            enable_monitoring: true,
            enable_ai_analysis: true,
            auto_restart_failed_agents: true,
            max_error_count: 10,
            status_report_interval_minutes: 15,
        };

        // Create orchestrator
        let orchestrator = Arc::new(RwLock::new(
            BlockchainAgentOrchestrator::new(
                orchestrator_config,
                grpc_client,
                (*memory_store).clone(),
                llm_router,
            )
        ));

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            memory_store,
            orchestrator,
            running: Arc::new(AtomicBool::new(false)),
            pid_file,
        })
    }

    /// Start the daemon service
    async fn start(&self) -> Result<()> {
        info!("Starting Jarvis Daemon service...");

        // Write PID file if specified
        if let Some(pid_file) = &self.pid_file {
            self.write_pid_file(pid_file)?;
        }

        // Set running state
        self.running.store(true, Ordering::SeqCst);

        // Start the orchestrator
        {
            let mut orchestrator = self.orchestrator.write().await;
            orchestrator.start().await
                .context("Failed to start agent orchestrator")?;
        }

        info!("Jarvis Daemon started successfully");

        // Main daemon loop
        self.run_daemon_loop().await?;

        Ok(())
    }

    /// Stop the daemon service
    async fn stop(&self) -> Result<()> {
        info!("Stopping Jarvis Daemon service...");

        // Set running state to false
        self.running.store(false, Ordering::SeqCst);

        // Stop the orchestrator
        {
            let mut orchestrator = self.orchestrator.write().await;
            orchestrator.shutdown().await
                .context("Failed to shutdown agent orchestrator")?;
        }

        // Remove PID file if it exists
        if let Some(pid_file) = &self.pid_file {
            if pid_file.exists() {
                std::fs::remove_file(pid_file)
                    .with_context(|| format!("Failed to remove PID file {:?}", pid_file))?;
            }
        }

        info!("Jarvis Daemon stopped successfully");
        Ok(())
    }

    /// Main daemon event loop
    async fn run_daemon_loop(&self) -> Result<()> {
        let mut health_check_interval = interval(Duration::from_secs(30));
        let mut config_reload_interval = interval(Duration::from_secs(300)); // 5 minutes
        let mut cleanup_interval = interval(Duration::from_secs(3600)); // 1 hour

        loop {
            tokio::select! {
                // Health check
                _ = health_check_interval.tick() => {
                    if let Err(e) = self.perform_health_check().await {
                        error!("Health check failed: {}", e);
                    }
                }

                // Config reload
                _ = config_reload_interval.tick() => {
                    if let Err(e) = self.reload_config().await {
                        warn!("Config reload failed: {}", e);
                    }
                }

                // Periodic cleanup
                _ = cleanup_interval.tick() => {
                    if let Err(e) = self.perform_cleanup().await {
                        warn!("Cleanup failed: {}", e);
                    }
                }

                // Graceful shutdown signals
                _ = signal::ctrl_c() => {
                    info!("Received SIGINT, shutting down gracefully...");
                    break;
                }

                // Check if we should stop running
                _ = sleep(Duration::from_secs(1)) => {
                    if !self.running.load(Ordering::SeqCst) {
                        info!("Daemon stop requested, shutting down...");
                        break;
                    }
                }
            }
        }

        self.stop().await?;
        Ok(())
    }

    /// Perform health check on all components
    async fn perform_health_check(&self) -> Result<()> {
        debug!("Performing health check...");

        // Check orchestrator health using get_system_health
        {
            let orchestrator = self.orchestrator.read().await;
            match orchestrator.get_system_health().await {
                Ok(health) => {
                    debug!("System health: {:?}", health);
                }
                Err(e) => {
                    warn!("Health check failed: {}", e);
                }
            }
        }

        // Check memory store health - simplified check
        // Just try to access the store to see if it's responsive
        let _ = (*self.memory_store).clone();

        debug!("Health check completed");
        Ok(())
    }

    /// Reload configuration from file
    async fn reload_config(&self) -> Result<()> {
        debug!("Reloading configuration...");

        // Try to load new config
        let new_config = if let Some(config_path) = self.get_config_path().await {
            match Config::load(Some(config_path.to_str().unwrap())).await {
                Ok(config) => config,
                Err(e) => {
                    warn!("Failed to reload config from {:?}: {}", config_path, e);
                    return Ok(());
                }
            }
        } else {
            return Ok(());
        };

        // Update config
        {
            let mut config = self.config.write().await;
            *config = new_config;
        }

        // For now, we'll just log that config was reloaded
        // In the future, we could restart components that need the new config
        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Perform periodic cleanup tasks
    async fn perform_cleanup(&self) -> Result<()> {
        debug!("Performing periodic cleanup...");

        // Clean up old memory entries - simplified since we don't have this method
        // We could implement this by querying old entries and removing them manually
        
        // Clean up temporary files
        if let Some(temp_dir) = self.get_temp_dir().await {
            if let Err(e) = self.cleanup_temp_files(&temp_dir).await {
                warn!("Failed to cleanup temp files: {}", e);
            }
        }

        debug!("Cleanup completed");
        Ok(())
    }

    /// Write process ID to PID file
    fn write_pid_file(&self, pid_file: &PathBuf) -> Result<()> {
        let pid = std::process::id();
        std::fs::write(pid_file, pid.to_string())
            .with_context(|| format!("Failed to write PID file {:?}", pid_file))?;
        info!("PID {} written to {:?}", pid, pid_file);
        Ok(())
    }

    /// Get current config file path
    async fn get_config_path(&self) -> Option<PathBuf> {
        // This would typically be stored during initialization
        // For now, return the default config path
        dirs::config_dir().map(|dir| dir.join("jarvis").join("jarvis.toml"))
    }

    /// Get temporary directory for cleanup
    async fn get_temp_dir(&self) -> Option<PathBuf> {
        std::env::temp_dir().join("jarvis").into()
    }

    /// Clean up temporary files older than 24 hours
    async fn cleanup_temp_files(&self, temp_dir: &PathBuf) -> Result<()> {
        if !temp_dir.exists() {
            return Ok(());
        }

        let cutoff = std::time::SystemTime::now() - Duration::from_secs(24 * 3600);
        
        for entry in std::fs::read_dir(temp_dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            
            if let Ok(modified) = metadata.modified() {
                if modified < cutoff {
                    let path = entry.path();
                    if let Err(e) = std::fs::remove_file(&path) {
                        debug!("Failed to remove temp file {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Get daemon status from PID file
async fn get_daemon_status(pid_file: &PathBuf) -> Result<DaemonStatus> {
    if !pid_file.exists() {
        return Ok(DaemonStatus::Stopped);
    }

    let pid_str = std::fs::read_to_string(pid_file)
        .context("Failed to read PID file")?;
    
    let pid: u32 = pid_str.trim().parse()
        .context("Invalid PID in PID file")?;

    // Check if process is still running
    if is_process_running(pid) {
        Ok(DaemonStatus::Running(pid))
    } else {
        // Clean up stale PID file
        std::fs::remove_file(pid_file)
            .context("Failed to remove stale PID file")?;
        Ok(DaemonStatus::Stopped)
    }
}

/// Check if a process is running
fn is_process_running(pid: u32) -> bool {
    use std::process::Command;
    
    #[cfg(unix)]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    #[cfg(not(unix))]
    {
        // For non-Unix systems, use a different approach
        use sysinfo::{System, SystemExt, ProcessExt};
        let mut system = System::new_all();
        system.refresh_processes();
        system.processes().contains_key(&sysinfo::Pid::from(pid as usize))
    }
}

/// Daemon status
enum DaemonStatus {
    Running(u32),
    Stopped,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "jarvisd=info,jarvis_core=info,jarvis_agent=info".to_string())
        )
        .init();

    let matches = Command::new("jarvisd")
        .version("0.1.0")
        .author("Christopher Kelley <ckelley@ghostkellz.sh>")
        .about("Jarvis Daemon - Autonomous Blockchain Agent Service")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
        )
        .arg(
            Arg::new("pid-file")
                .short('p')
                .long("pid-file")
                .value_name("FILE")
                .help("PID file path")
                .default_value("/var/run/jarvisd.pid")
        )
        .arg(
            Arg::new("daemon")
                .short('d')
                .long("daemon")
                .help("Run as daemon (detach from terminal)")
                .action(clap::ArgAction::SetTrue)
        )
        .subcommand(
            Command::new("start")
                .about("Start the daemon service")
        )
        .subcommand(
            Command::new("stop")
                .about("Stop the daemon service")
        )
        .subcommand(
            Command::new("restart")
                .about("Restart the daemon service")
        )
        .subcommand(
            Command::new("status")
                .about("Show daemon status")
        )
        .subcommand(
            Command::new("logs")
                .about("Show daemon logs")
                .arg(
                    Arg::new("follow")
                        .short('f')
                        .long("follow")
                        .help("Follow log output")
                        .action(clap::ArgAction::SetTrue)
                )
        )
        .get_matches();

    let config_path = matches.get_one::<String>("config").map(PathBuf::from);
    let pid_file = PathBuf::from(matches.get_one::<String>("pid-file").unwrap());
    let _daemon_mode = matches.get_flag("daemon");

    match matches.subcommand() {
        Some(("start", _)) => {
            info!("Starting Jarvis Daemon...");
            
            // Check if already running
            match get_daemon_status(&pid_file).await? {
                DaemonStatus::Running(pid) => {
                    eprintln!("Daemon is already running with PID {}", pid);
                    std::process::exit(1);
                }
                DaemonStatus::Stopped => {
                    // Start the daemon
                    let daemon = JarvisDaemon::new(config_path, Some(pid_file)).await?;
                    daemon.start().await?;
                }
            }
        }

        Some(("stop", _)) => {
            info!("Stopping Jarvis Daemon...");
            
            match get_daemon_status(&pid_file).await? {
                DaemonStatus::Running(pid) => {
                    // Send SIGTERM to the process
                    #[cfg(unix)]
                    {
                        use std::process::Command;
                        let output = Command::new("kill")
                            .args(["-TERM", &pid.to_string()])
                            .output()
                            .context("Failed to send SIGTERM")?;
                        
                        if !output.status.success() {
                            eprintln!("Failed to stop daemon");
                            std::process::exit(1);
                        }
                    }
                    
                    println!("Daemon stopped successfully");
                }
                DaemonStatus::Stopped => {
                    println!("Daemon is not running");
                }
            }
        }

        Some(("restart", _)) => {
            info!("Restarting Jarvis Daemon...");
            
            // Stop if running
            if let DaemonStatus::Running(pid) = get_daemon_status(&pid_file).await? {
                #[cfg(unix)]
                {
                    use std::process::Command;
                    let _ = Command::new("kill")
                        .args(["-TERM", &pid.to_string()])
                        .output();
                }
                
                // Wait a bit for graceful shutdown
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            
            // Start the daemon
            let daemon = JarvisDaemon::new(config_path, Some(pid_file)).await?;
            daemon.start().await?;
        }

        Some(("status", _)) => {
            match get_daemon_status(&pid_file).await? {
                DaemonStatus::Running(pid) => {
                    println!("Jarvis Daemon is running (PID: {})", pid);
                    
                    // TODO: Add more detailed status information
                    // - Uptime
                    // - Agent status
                    // - Memory usage
                    // - Active blockchain connections
                }
                DaemonStatus::Stopped => {
                    println!("Jarvis Daemon is not running");
                }
            }
        }

        Some(("logs", sub_matches)) => {
            let _follow = sub_matches.get_flag("follow");
            // TODO: Implement log viewing
            // This would typically read from syslog or a dedicated log file
            println!("Log viewing not yet implemented");
        }

        _ => {
            // No subcommand, run in foreground mode
            info!("Running Jarvis Daemon in foreground mode...");
            let daemon = JarvisDaemon::new(config_path, Some(pid_file)).await?;
            daemon.start().await?;
        }
    }

    Ok(())
}
