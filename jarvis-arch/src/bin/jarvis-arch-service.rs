/// Jarvis Arch Linux System Agent Service
/// This is the main service binary that runs as a systemd daemon
use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use jarvis_arch::{
    ArchLinuxAgent, ArchAgent, ArchOperation, ArchConfig,
    PackageManager, SystemHealth, SecurityScanner,
    zqlite_integration::{JarvisDatabase, DatabaseConfig}
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "jarvis-arch")]
#[command(about = "Jarvis Arch Linux System Maintenance Agent")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Configuration file path
    #[arg(short, long, default_value = "/etc/jarvis/jarvis-arch.toml")]
    config: PathBuf,
    
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
    
    /// Run as daemon (systemd service mode)
    #[arg(short = 'D', long)]
    daemon: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Jarvis service daemon
    Service {
        /// Systemd socket file descriptor
        #[arg(long)]
        systemd_fd: Option<i32>,
    },
    
    /// Package management operations
    Package {
        #[command(subcommand)]
        operation: PackageCommands,
    },
    
    /// System health and monitoring
    Health {
        #[command(subcommand)]
        operation: HealthCommands,
    },
    
    /// Security operations
    Security {
        #[command(subcommand)]
        operation: SecurityCommands,
    },
    
    /// Agent management
    Agent {
        #[command(subcommand)]
        operation: AgentCommands,
    },
}

#[derive(Subcommand)]
enum PackageCommands {
    /// Update system packages
    Update {
        /// Specific packages to update
        packages: Vec<String>,
        /// Include AUR packages
        #[arg(long)]
        aur: bool,
    },
    
    /// Install package
    Install {
        /// Package name
        package: String,
        /// Install from AUR
        #[arg(long)]
        aur: bool,
    },
    
    /// Remove package
    Remove {
        /// Package name
        package: String,
        /// Remove dependencies
        #[arg(long)]
        deps: bool,
    },
    
    /// Search packages
    Search {
        /// Search query
        query: String,
        /// Include AUR
        #[arg(long)]
        aur: bool,
    },
    
    /// List installed packages
    List {
        /// Filter by repository
        #[arg(long)]
        repo: Option<String>,
    },
    
    /// Check for updates
    Check,
    
    /// Clean package cache
    Clean {
        /// Aggressive cleaning
        #[arg(long)]
        aggressive: bool,
    },
}

#[derive(Subcommand)]
enum HealthCommands {
    /// Check system health
    Check {
        /// Include service status
        #[arg(long)]
        services: bool,
    },
    
    /// Monitor system metrics
    Monitor {
        /// Duration in minutes
        #[arg(short, long, default_value = "5")]
        duration: u32,
    },
    
    /// Analyze system logs
    Logs {
        /// Service name
        #[arg(long)]
        service: Option<String>,
        /// Hours to analyze
        #[arg(long, default_value = "24")]
        hours: u32,
    },
}

#[derive(Subcommand)]
enum SecurityCommands {
    /// Run security scan
    Scan {
        /// Full system scan
        #[arg(long)]
        full: bool,
    },
    
    /// Check for vulnerabilities
    Vulnerabilities {
        /// Specific packages
        packages: Vec<String>,
    },
    
    /// AUR security check
    AurCheck {
        /// Specific AUR packages
        packages: Vec<String>,
    },
}

#[derive(Subcommand)]
enum AgentCommands {
    /// Get agent status
    Status,
    
    /// Start the agent
    Start,
    
    /// Stop the agent
    Stop,
    
    /// Restart the agent
    Restart,
    
    /// Get agent configuration
    Config,
    
    /// Run health check
    Health,
}

/// Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceConfig {
    pub agent: ArchConfig,
    pub database: DatabaseConfig,
    pub service: ServiceSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceSettings {
    pub listen_address: String,
    pub listen_port: u16,
    pub enable_metrics: bool,
    pub metrics_port: u16,
    pub log_level: String,
    pub enable_systemd_notifications: bool,
    pub health_check_interval_seconds: u64,
    pub maintenance_schedule: MaintenanceSchedule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MaintenanceSchedule {
    pub auto_update: bool,
    pub update_schedule: String, // Cron expression
    pub auto_clean: bool,
    pub clean_schedule: String,
    pub security_scan_schedule: String,
}

/// Main service manager
struct JarvisService {
    agent: Arc<RwLock<ArchLinuxAgent>>,
    database: Arc<RwLock<JarvisDatabase>>,
    config: ServiceConfig,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(format!("jarvis_arch={}", log_level).parse()?)
        )
        .init();
    
    // Load configuration
    let config = load_config(&cli.config).await
        .with_context(|| format!("Failed to load config from {:?}", cli.config))?;
    
    match cli.command {
        Commands::Service { systemd_fd } => {
            run_service(config, systemd_fd).await
        }
        Commands::Package { operation } => {
            run_package_command(config, operation).await
        }
        Commands::Health { operation } => {
            run_health_command(config, operation).await
        }
        Commands::Security { operation } => {
            run_security_command(config, operation).await
        }
        Commands::Agent { operation } => {
            run_agent_command(config, operation).await
        }
    }
}

async fn load_config(path: &PathBuf) -> Result<ServiceConfig> {
    if path.exists() {
        let content = tokio::fs::read_to_string(path).await?;
        let config: ServiceConfig = toml::from_str(&content)?;
        Ok(config)
    } else {
        // Create default config
        let config = ServiceConfig::default();
        let content = toml::to_string_pretty(&config)?;
        
        // Create config directory if it doesn't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::write(path, content).await?;
        info!("Created default configuration at {:?}", path);
        Ok(config)
    }
}

async fn run_service(config: ServiceConfig, _systemd_fd: Option<i32>) -> Result<()> {
    info!("Starting Jarvis Arch Linux System Agent Service");
    
    // Initialize database
    let database = JarvisDatabase::new(config.database.clone()).await?;
    let database = Arc::new(RwLock::new(database));
    
    // Initialize agent
    let mut agent = ArchLinuxAgent::new();
    agent.initialize(config.agent.clone()).await?;
    let agent = Arc::new(RwLock::new(agent));
    
    let service = JarvisService {
        agent: agent.clone(),
        database: database.clone(),
        config: config.clone(),
        shutdown_tx: None,
    };
    
    // Set up signal handlers
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    
    // Start service components
    let health_check_task = start_health_monitor(agent.clone(), config.service.health_check_interval_seconds);
    let maintenance_task = start_maintenance_scheduler(agent.clone(), config.service.maintenance_schedule.clone());
    let metrics_task = if config.service.enable_metrics {
        Some(start_metrics_server(config.service.metrics_port))
    } else {
        None
    };
    
    // Notify systemd that we're ready
    if config.service.enable_systemd_notifications {
        #[cfg(target_os = "linux")]
        {
            systemd_notify_ready().await?;
        }
    }
    
    info!("Jarvis service is running and ready");
    
    // Wait for shutdown signal
    tokio::select! {
        _ = shutdown_rx => {
            info!("Received shutdown signal");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down");
        }
    }
    
    // Graceful shutdown
    info!("Shutting down Jarvis service...");
    
    // Cancel all tasks
    health_check_task.abort();
    maintenance_task.abort();
    if let Some(task) = metrics_task {
        task.abort();
    }
    
    // Shutdown agent
    agent.write().await.shutdown().await?;
    
    // Close database
    database.write().await.close().await?;
    
    info!("Jarvis service shutdown complete");
    Ok(())
}

async fn run_package_command(config: ServiceConfig, operation: PackageCommands) -> Result<()> {
    let mut agent = ArchLinuxAgent::new();
    agent.initialize(config.agent).await?;
    
    let arch_operation = match operation {
        PackageCommands::Update { packages, aur: _ } => {
            let packages = if packages.is_empty() { None } else { Some(packages) };
            ArchOperation::UpdatePackages { packages }
        }
        PackageCommands::Install { package, aur } => {
            ArchOperation::InstallPackage { package, from_aur: aur }
        }
        PackageCommands::Remove { package, deps } => {
            ArchOperation::RemovePackage { package, remove_deps: deps }
        }
        PackageCommands::Search { query, aur } => {
            ArchOperation::SearchPackages { query, include_aur: aur }
        }
        PackageCommands::List { repo: _ } => {
            // Convert to appropriate operation
            ArchOperation::CustomCommand { 
                command: "pacman".to_string(), 
                args: vec!["-Q".to_string()] 
            }
        }
        PackageCommands::Check => {
            ArchOperation::CustomCommand { 
                command: "pacman".to_string(), 
                args: vec!["-Qu".to_string()] 
            }
        }
        PackageCommands::Clean { aggressive } => {
            ArchOperation::SystemCleanup { 
                clean_cache: true, 
                clean_logs: aggressive 
            }
        }
    };
    
    let result = agent.execute_operation(arch_operation).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    
    Ok(())
}

async fn run_health_command(config: ServiceConfig, operation: HealthCommands) -> Result<()> {
    let mut agent = ArchLinuxAgent::new();
    agent.initialize(config.agent).await?;
    
    let arch_operation = match operation {
        HealthCommands::Check { services } => {
            ArchOperation::HealthCheck { include_services: services }
        }
        HealthCommands::Monitor { duration } => {
            ArchOperation::PerformanceAnalysis { duration_minutes: duration }
        }
        HealthCommands::Logs { service, hours } => {
            ArchOperation::LogAnalysis { service, hours }
        }
    };
    
    let result = agent.execute_operation(arch_operation).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    
    Ok(())
}

async fn run_security_command(config: ServiceConfig, operation: SecurityCommands) -> Result<()> {
    let mut agent = ArchLinuxAgent::new();
    agent.initialize(config.agent).await?;
    
    let arch_operation = match operation {
        SecurityCommands::Scan { full } => {
            ArchOperation::SecurityScan { full_scan: full }
        }
        SecurityCommands::Vulnerabilities { packages } => {
            let packages = if packages.is_empty() { None } else { Some(packages) };
            ArchOperation::VulnerabilityScan { packages }
        }
        SecurityCommands::AurCheck { packages } => {
            let packages = if packages.is_empty() { None } else { Some(packages) };
            ArchOperation::AURSecurityCheck { packages }
        }
    };
    
    let result = agent.execute_operation(arch_operation).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    
    Ok(())
}

async fn run_agent_command(config: ServiceConfig, operation: AgentCommands) -> Result<()> {
    match operation {
        AgentCommands::Status => {
            // Check if service is running
            let status = check_service_status().await?;
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        AgentCommands::Start => {
            start_service().await?;
        }
        AgentCommands::Stop => {
            stop_service().await?;
        }
        AgentCommands::Restart => {
            restart_service().await?;
        }
        AgentCommands::Config => {
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
        AgentCommands::Health => {
            let mut agent = ArchLinuxAgent::new();
            agent.initialize(config.agent).await?;
            let health = agent.health_check().await?;
            println!("{}", serde_json::to_string_pretty(&health)?);
        }
    }
    
    Ok(())
}

async fn start_health_monitor(
    agent: Arc<RwLock<ArchLinuxAgent>>, 
    interval_seconds: u64
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_seconds));
        
        loop {
            interval.tick().await;
            
            match agent.read().await.health_check().await {
                Ok(health) => {
                    if matches!(health.status, jarvis_arch::HealthStatus::Critical) {
                        error!("Critical health issue detected: {:?}", health);
                        // Could trigger alerts or auto-recovery here
                    }
                }
                Err(e) => {
                    error!("Health check failed: {}", e);
                }
            }
        }
    })
}

async fn start_maintenance_scheduler(
    agent: Arc<RwLock<ArchLinuxAgent>>, 
    schedule: MaintenanceSchedule
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // This would implement cron-like scheduling
        // For now, just a placeholder
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // 1 hour
        
        loop {
            interval.tick().await;
            
            if schedule.auto_update {
                info!("Running scheduled package update");
                let operation = ArchOperation::UpdatePackages { packages: None };
                match agent.read().await.execute_operation(operation).await {
                    Ok(result) => {
                        if result.success {
                            info!("Scheduled update completed successfully");
                        } else {
                            warn!("Scheduled update failed: {:?}", result.error);
                        }
                    }
                    Err(e) => {
                        error!("Scheduled update error: {}", e);
                    }
                }
            }
        }
    })
}

async fn start_metrics_server(port: u16) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Placeholder for metrics server (Prometheus, etc.)
        info!("Metrics server would start on port {}", port);
        // Implementation would use a web framework like axum
    })
}

#[cfg(target_os = "linux")]
async fn systemd_notify_ready() -> Result<()> {
    use std::os::unix::net::UnixDatagram;
    
    if let Ok(socket_path) = std::env::var("NOTIFY_SOCKET") {
        let socket = UnixDatagram::unbound()?;
        socket.send_to(b"READY=1", &socket_path)?;
        info!("Notified systemd that service is ready");
    }
    
    Ok(())
}

async fn check_service_status() -> Result<serde_json::Value> {
    // Check systemd service status
    let output = tokio::process::Command::new("systemctl")
        .args(&["is-active", "jarvis-arch"])
        .output()
        .await?;
    
    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    Ok(serde_json::json!({
        "service": "jarvis-arch",
        "status": status,
        "active": status == "active",
        "pid": std::process::id()
    }))
}

async fn start_service() -> Result<()> {
    tokio::process::Command::new("systemctl")
        .args(&["start", "jarvis-arch"])
        .status()
        .await?;
    
    println!("Jarvis service started");
    Ok(())
}

async fn stop_service() -> Result<()> {
    tokio::process::Command::new("systemctl")
        .args(&["stop", "jarvis-arch"])
        .status()
        .await?;
    
    println!("Jarvis service stopped");
    Ok(())
}

async fn restart_service() -> Result<()> {
    tokio::process::Command::new("systemctl")
        .args(&["restart", "jarvis-arch"])
        .status()
        .await?;
    
    println!("Jarvis service restarted");
    Ok(())
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            agent: ArchConfig::default(),
            database: DatabaseConfig::default(),
            service: ServiceSettings::default(),
        }
    }
}

impl Default for ServiceSettings {
    fn default() -> Self {
        Self {
            listen_address: "127.0.0.1".to_string(),
            listen_port: 7419, // JARV on phone keypad
            enable_metrics: true,
            metrics_port: 9090,
            log_level: "info".to_string(),
            enable_systemd_notifications: true,
            health_check_interval_seconds: 300, // 5 minutes
            maintenance_schedule: MaintenanceSchedule::default(),
        }
    }
}

impl Default for MaintenanceSchedule {
    fn default() -> Self {
        Self {
            auto_update: false, // Disabled by default for safety
            update_schedule: "0 2 * * 0".to_string(), // Sunday 2 AM
            auto_clean: true,
            clean_schedule: "0 3 * * 0".to_string(), // Sunday 3 AM
            security_scan_schedule: "0 1 * * *".to_string(), // Daily 1 AM
        }
    }
}