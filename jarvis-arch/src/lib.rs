pub mod package_manager;
pub mod aur_monitor;
pub mod system_health;
pub mod security_scanner;
pub mod maintenance_scheduler;
pub mod config;
pub mod vulnerability_scanner;
pub mod service_manager;
pub mod wazuh;
pub mod zqlite_integration;

// Re-export main types
pub use package_manager::{PackageManager, PackageInfo, PackageOperation, PackageStatus};
pub use aur_monitor::{AURMonitor, AURPackage, AURSecurityIssue};
pub use system_health::{SystemHealth, HealthMetric, HealthStatus};
pub use security_scanner::{SecurityScanner, SecurityIssue, SecuritySeverity};
pub use maintenance_scheduler::{MaintenanceScheduler, MaintenanceTask, MaintenanceResult};
pub use config::{Config, AgentConfig, PacmanConfig, SystemConfig, WazuhConfig};
pub use vulnerability_scanner::{VulnerabilityScanner, Vulnerability, CVEInfo};
pub use service_manager::{ServiceManager, ServiceInfo, ServiceOperation};
pub use wazuh::{WazuhIntegration, SecurityEvent, RiskLevel};
pub use zqlite_integration::{ZQLiteDatabase, DatabaseConfig};

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Main Arch Linux agent interface
#[async_trait]
pub trait ArchAgent: Send + Sync {
    /// Initialize the agent with configuration
    async fn initialize(&mut self, config: ArchConfig) -> Result<()>;
    
    /// Perform health check
    async fn health_check(&self) -> Result<AgentHealth>;
    
    /// Get agent capabilities
    fn capabilities(&self) -> Vec<AgentCapability>;
    
    /// Execute an operation
    async fn execute_operation(&self, operation: ArchOperation) -> Result<OperationResult>;
    
    /// Get current status
    async fn get_status(&self) -> Result<AgentStatus>;
    
    /// Shutdown the agent
    async fn shutdown(&mut self) -> Result<()>;
}

/// Arch-specific operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchOperation {
    // Package management
    UpdatePackages { packages: Option<Vec<String>> },
    InstallPackage { package: String, from_aur: bool },
    RemovePackage { package: String, remove_deps: bool },
    SearchPackages { query: String, include_aur: bool },
    
    // System maintenance
    SystemCleanup { clean_cache: bool, clean_logs: bool },
    UpdateMirrorlist { country: Option<String> },
    CheckDiskUsage { path: Option<String> },
    
    // Security operations
    SecurityScan { full_scan: bool },
    VulnerabilityScan { packages: Option<Vec<String>> },
    AURSecurityCheck { packages: Option<Vec<String>> },
    
    // Service management
    ServiceOperation { service: String, operation: ServiceOperation },
    ListServices { filter: Option<String> },
    
    // System monitoring
    HealthCheck { include_services: bool },
    PerformanceAnalysis { duration_minutes: u32 },
    LogAnalysis { service: Option<String>, hours: u32 },
    
    // Configuration management
    BackupConfigs { destination: String },
    RestoreConfigs { source: String },
    ValidateConfigs,
    
    // Custom operations
    CustomCommand { command: String, args: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub operation: ArchOperation,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub executed_at: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHealth {
    pub status: HealthStatus,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub uptime_seconds: u64,
    pub error_count: u32,
    pub success_rate: f64,
    pub system_load: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub active_operations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentCapability {
    PackageManagement,
    AURSupport,
    SecurityScanning,
    VulnerabilityAssessment,
    SystemMonitoring,
    ServiceManagement,
    ConfigurationManagement,
    LogAnalysis,
    PerformanceAnalysis,
    AutomatedMaintenance,
    WazuhIntegration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub agent_id: Uuid,
    pub version: String,
    pub status: AgentState,
    pub capabilities: Vec<AgentCapability>,
    pub active_operations: Vec<String>,
    pub last_maintenance: Option<chrono::DateTime<chrono::Utc>>,
    pub next_scheduled_maintenance: Option<chrono::DateTime<chrono::Utc>>,
    pub statistics: AgentStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentState {
    Initializing,
    Ready,
    Busy,
    Maintenance,
    Error,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatistics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub packages_managed: u64,
    pub security_issues_found: u64,
    pub security_issues_resolved: u64,
    pub uptime_hours: f64,
    pub average_operation_time_ms: f64,
}

/// Main Arch Linux system agent
pub struct ArchLinuxAgent {
    config: Option<Config>,
    package_manager: Option<PackageManager>,
    aur_monitor: Option<AURMonitor>,
    system_health: Option<SystemHealth>,
    security_scanner: Option<SecurityScanner>,
    maintenance_scheduler: Option<MaintenanceScheduler>,
    vulnerability_scanner: Option<VulnerabilityScanner>,
    service_manager: Option<ServiceManager>,
    wazuh_integration: Option<WazuhIntegration>,
    database: Option<ZQLiteDatabase>,
    agent_id: Uuid,
    statistics: AgentStatistics,
    state: AgentState,
    start_time: chrono::DateTime<chrono::Utc>,
}

impl ArchLinuxAgent {
    pub fn new() -> Self {
        Self {
            config: None,
            package_manager: None,
            aur_monitor: None,
            system_health: None,
            security_scanner: None,
            maintenance_scheduler: None,
            vulnerability_scanner: None,
            service_manager: None,
            wazuh_integration: None,
            database: None,
            agent_id: Uuid::new_v4(),
            statistics: AgentStatistics::default(),
            state: AgentState::Initializing,
            start_time: chrono::Utc::now(),
        }
    }
    
    /// Get package manager instance
    pub fn package_manager(&self) -> Option<&PackageManager> {
        self.package_manager.as_ref()
    }
    
    /// Get AUR monitor instance
    pub fn aur_monitor(&self) -> Option<&AURMonitor> {
        self.aur_monitor.as_ref()
    }
    
    /// Get system health monitor instance
    pub fn system_health(&self) -> Option<&SystemHealth> {
        self.system_health.as_ref()
    }
    
    /// Get security scanner instance
    pub fn security_scanner(&self) -> Option<&SecurityScanner> {
        self.security_scanner.as_ref()
    }
    
    /// Get maintenance scheduler instance
    pub fn maintenance_scheduler(&self) -> Option<&MaintenanceScheduler> {
        self.maintenance_scheduler.as_ref()
    }
    
    /// Get vulnerability scanner instance
    pub fn vulnerability_scanner(&self) -> Option<&VulnerabilityScanner> {
        self.vulnerability_scanner.as_ref()
    }
    
    /// Get service manager instance
    pub fn service_manager(&self) -> Option<&ServiceManager> {
        self.service_manager.as_ref()
    }
    
    /// Get Wazuh integration instance
    pub fn wazuh_integration(&self) -> Option<&WazuhIntegration> {
        self.wazuh_integration.as_ref()
    }
    
    /// Get database instance
    pub fn database(&self) -> Option<&ZQLiteDatabase> {
        self.database.as_ref()
    }
}

#[async_trait]
impl ArchAgent for ArchLinuxAgent {
    async fn initialize(&mut self, config: Config) -> Result<()> {
        self.state = AgentState::Initializing;
        
        // Initialize ZQLite database first
        let mut database = ZQLiteDatabase::new();
        database.initialize(&config.database).await?;
        self.database = Some(database);
        
        // Initialize package manager
        let mut package_manager = PackageManager::new();
        package_manager.initialize(&config.agent.pacman).await?;
        self.package_manager = Some(package_manager);
        
        // Initialize AUR monitor if enabled
        if config.agent.aur.enabled {
            let mut aur_monitor = AURMonitor::new();
            aur_monitor.initialize(&config.agent.aur).await?;
            self.aur_monitor = Some(aur_monitor);
        }
        
        // Initialize system health monitor
        let mut system_health = SystemHealth::new();
        system_health.initialize(&config.agent.system).await?;
        self.system_health = Some(system_health);
        
        // Initialize security scanner
        let mut security_scanner = SecurityScanner::new();
        security_scanner.initialize(&config.agent.security).await?;
        self.security_scanner = Some(security_scanner);
        
        // Initialize maintenance scheduler
        let mut maintenance_scheduler = MaintenanceScheduler::new();
        maintenance_scheduler.initialize(&config.agent.maintenance).await?;
        self.maintenance_scheduler = Some(maintenance_scheduler);
        
        // Initialize vulnerability scanner
        let mut vulnerability_scanner = VulnerabilityScanner::new();
        vulnerability_scanner.initialize(&config.agent.vulnerability).await?;
        self.vulnerability_scanner = Some(vulnerability_scanner);
        
        // Initialize service manager
        let mut service_manager = ServiceManager::new();
        service_manager.initialize(&config.agent.services).await?;
        self.service_manager = Some(service_manager);
        
        // Initialize Wazuh integration if enabled
        if config.wazuh.enabled {
            if let Some(ref package_manager) = self.package_manager {
                let wazuh_integration = WazuhIntegration::new(
                    config.wazuh.clone(),
                    package_manager.clone(),
                );
                wazuh_integration.initialize().await?;
                self.wazuh_integration = Some(wazuh_integration);
                
                tracing::info!("Wazuh integration initialized for AUR package monitoring");
            }
        }
        
        self.config = Some(config);
        self.state = AgentState::Ready;
        
        tracing::info!("Arch Linux agent initialized successfully with Wazuh integration");
        Ok(())
    }
    
    async fn health_check(&self) -> Result<AgentHealth> {
        let system_info = sysinfo::System::new_all();
        let uptime = chrono::Utc::now()
            .signed_duration_since(self.start_time)
            .num_seconds() as u64;
        
        let success_rate = if self.statistics.total_operations > 0 {
            self.statistics.successful_operations as f64 / self.statistics.total_operations as f64
        } else {
            1.0
        };
        
        Ok(AgentHealth {
            status: self.determine_health_status(),
            last_check: chrono::Utc::now(),
            uptime_seconds: uptime,
            error_count: self.statistics.failed_operations as u32,
            success_rate,
            system_load: system_info.load_average().one,
            memory_usage_percent: (system_info.used_memory() as f64 / system_info.total_memory() as f64) * 100.0,
            disk_usage_percent: 0.0, // Would implement actual disk usage check
            active_operations: 0, // Would track active operations
        })
    }
    
    fn capabilities(&self) -> Vec<AgentCapability> {
        let mut caps = vec![
            AgentCapability::PackageManagement,
            AgentCapability::SystemMonitoring,
            AgentCapability::ServiceManagement,
            AgentCapability::ConfigurationManagement,
            AgentCapability::LogAnalysis,
            AgentCapability::PerformanceAnalysis,
            AgentCapability::AutomatedMaintenance,
        ];
        
        if self.aur_monitor.is_some() {
            caps.push(AgentCapability::AURSupport);
        }
        
        if self.security_scanner.is_some() {
            caps.push(AgentCapability::SecurityScanning);
        }
        
        if self.vulnerability_scanner.is_some() {
            caps.push(AgentCapability::VulnerabilityAssessment);
        }
        
        caps
    }
    
    async fn execute_operation(&self, operation: ArchOperation) -> Result<OperationResult> {
        let start_time = std::time::Instant::now();
        let executed_at = chrono::Utc::now();
        
        let result = match operation.clone() {
            ArchOperation::UpdatePackages { packages } => {
                if let Some(pm) = &self.package_manager {
                    pm.update_packages(packages).await
                } else {
                    Err(anyhow::anyhow!("Package manager not initialized"))
                }
            }
            
            ArchOperation::SecurityScan { full_scan } => {
                if let Some(scanner) = &self.security_scanner {
                    scanner.scan_system(full_scan).await
                } else {
                    Err(anyhow::anyhow!("Security scanner not initialized"))
                }
            }
            
            ArchOperation::HealthCheck { include_services } => {
                if let Some(health) = &self.system_health {
                    health.check_system_health(include_services).await
                } else {
                    Err(anyhow::anyhow!("System health monitor not initialized"))
                }
            }
            
            // Add more operation implementations...
            _ => {
                Err(anyhow::anyhow!("Operation not implemented: {:?}", operation))
            }
        };
        
        let duration = start_time.elapsed();
        let success = result.is_ok();
        
        Ok(OperationResult {
            operation,
            success,
            output: match result {
                Ok(data) => data,
                Err(e) => serde_json::json!({"error": e.to_string()}),
            },
            error: if success { None } else { Some("Operation failed".to_string()) },
            duration_ms: duration.as_millis() as u64,
            executed_at,
            metadata: HashMap::new(),
        })
    }
    
    async fn get_status(&self) -> Result<AgentStatus> {
        Ok(AgentStatus {
            agent_id: self.agent_id,
            version: env!("CARGO_PKG_VERSION").to_string(),
            status: self.state.clone(),
            capabilities: self.capabilities(),
            active_operations: vec![], // Would track active operations
            last_maintenance: None, // Would track from scheduler
            next_scheduled_maintenance: None, // Would get from scheduler
            statistics: self.statistics.clone(),
        })
    }
    
    async fn shutdown(&mut self) -> Result<()> {
        self.state = AgentState::Shutdown;
        
        // Shutdown all components
        if let Some(scheduler) = &mut self.maintenance_scheduler {
            scheduler.shutdown().await?;
        }
        
        tracing::info!("Arch Linux agent shutdown completed");
        Ok(())
    }
}

impl ArchLinuxAgent {
    fn determine_health_status(&self) -> HealthStatus {
        match self.state {
            AgentState::Ready => HealthStatus::Healthy,
            AgentState::Busy => HealthStatus::Healthy,
            AgentState::Maintenance => HealthStatus::Warning,
            AgentState::Error => HealthStatus::Critical,
            AgentState::Shutdown => HealthStatus::Critical,
            AgentState::Initializing => HealthStatus::Unknown,
        }
    }
}

impl Default for AgentStatistics {
    fn default() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            packages_managed: 0,
            security_issues_found: 0,
            security_issues_resolved: 0,
            uptime_hours: 0.0,
            average_operation_time_ms: 0.0,
        }
    }
}