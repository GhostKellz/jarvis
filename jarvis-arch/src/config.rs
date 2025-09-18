use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure for Jarvis Arch agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub agent: AgentConfig,
    pub database: DatabaseConfig,
    pub service: ServiceConfig,
    pub wazuh: WazuhConfig,
    pub logging: LoggingConfig,
    pub notifications: NotificationsConfig,
    pub performance: PerformanceConfig,
    pub network: NetworkConfig,
    pub hooks: HooksConfig,
}

/// Agent-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub pacman: PacmanConfig,
    pub aur: AurConfig,
    pub system: SystemConfig,
    pub security: SecurityConfig,
    pub maintenance: MaintenanceConfig,
    pub services: ServicesConfig,
    pub vulnerability: VulnerabilityConfig,
}

/// Pacman configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacmanConfig {
    pub no_confirm: bool,
    pub verbose: bool,
    pub check_space: bool,
    pub download_timeout: u32,
    pub parallel_downloads: u32,
}

/// AUR configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AurConfig {
    pub enabled: bool,
    pub helper: String,
    pub check_updates: bool,
    pub build_timeout: u32,
    pub pgp_verify: bool,
}

/// System monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub check_interval: u32,
    pub disk_usage_threshold: u32,
    pub memory_threshold: u32,
    pub load_threshold: f64,
    pub temp_threshold: u32,
}

/// Security scanning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enabled: bool,
    pub vulnerability_feeds: Vec<String>,
    pub check_interval: u32,
    pub auto_patch: bool,
    pub severity_threshold: String,
}

/// Maintenance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceConfig {
    pub enabled: bool,
    pub cleanup_cache: bool,
    pub cleanup_logs: bool,
    pub update_mirrorlist: bool,
    pub vacuum_database: bool,
}

/// Services monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    pub monitor_critical: bool,
    pub restart_failed: bool,
    pub services_to_monitor: Vec<String>,
}

/// Vulnerability scanning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityConfig {
    pub database_url: String,
    pub update_interval: u32,
    pub cache_duration: u32,
    pub check_aur_packages: bool,
}

/// ZQLite database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub db_path: PathBuf,
    pub encryption_key: String,
    pub max_connections: u32,
    pub enable_wal_mode: bool,
    pub enable_foreign_keys: bool,
    pub cache_size_kb: u32,
    pub page_size: u32,
    pub vacuum_on_startup: bool,
}

/// Service daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub listen_address: String,
    pub listen_port: u16,
    pub enable_metrics: bool,
    pub metrics_port: u16,
    pub log_level: String,
    pub enable_systemd_notifications: bool,
    pub health_check_interval_seconds: u32,
    pub maintenance_schedule: MaintenanceScheduleConfig,
}

/// Maintenance scheduling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceScheduleConfig {
    pub auto_update: bool,
    pub update_schedule: String,
    pub auto_clean: bool,
    pub clean_schedule: String,
    pub security_scan_schedule: String,
}

/// Wazuh SIEM integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WazuhConfig {
    pub enabled: bool,
    pub server: String,
    pub port: u16,
    pub protocol: String,
    pub encryption: bool,
    pub certificate_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: PathBuf,
    pub max_size_mb: u32,
    pub max_files: u32,
    pub json_format: bool,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
    pub enabled: bool,
    pub email_enabled: bool,
    pub email_to: Option<String>,
    pub email_from: Option<String>,
    pub webhook_enabled: bool,
    pub webhook_url: Option<String>,
    pub desktop_enabled: bool,
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub max_memory_mb: u32,
    pub max_cpu_percent: u32,
    pub max_disk_io_mb_s: u32,
    pub nice_level: i32,
    pub enable_package_cache: bool,
    pub cache_ttl_hours: u32,
    pub max_cache_size_mb: u32,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub timeout_seconds: u32,
    pub retry_attempts: u32,
    pub user_agent: String,
}

/// Hooks configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksConfig {
    pub pre_update: Vec<String>,
    pub post_update: Vec<String>,
    pub pre_install: Vec<String>,
    pub post_install: Vec<String>,
}

impl Config {
    /// Load configuration from TOML file
    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration with default values
    pub fn load_with_defaults() -> Self {
        Self::default()
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate database path
        if let Some(parent) = self.database.db_path.parent() {
            if !parent.exists() {
                return Err(anyhow::anyhow!("Database directory does not exist: {:?}", parent));
            }
        }

        // Validate log file path
        if let Some(parent) = self.logging.file.parent() {
            if !parent.exists() {
                return Err(anyhow::anyhow!("Log directory does not exist: {:?}", parent));
            }
        }

        // Validate Wazuh configuration if enabled
        if self.wazuh.enabled {
            if self.wazuh.server.is_empty() {
                return Err(anyhow::anyhow!("Wazuh server address cannot be empty"));
            }
            if self.wazuh.port == 0 {
                return Err(anyhow::anyhow!("Wazuh port must be valid"));
            }
            if !["tcp", "udp"].contains(&self.wazuh.protocol.as_str()) {
                return Err(anyhow::anyhow!("Wazuh protocol must be 'tcp' or 'udp'"));
            }
        }

        // Validate AUR helper if AUR is enabled
        if self.agent.aur.enabled {
            let valid_helpers = ["yay", "paru", "trizen", "aurman"];
            if !valid_helpers.contains(&self.agent.aur.helper.as_str()) {
                return Err(anyhow::anyhow!("Invalid AUR helper: {}", self.agent.aur.helper));
            }
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent: AgentConfig::default(),
            database: DatabaseConfig::default(),
            service: ServiceConfig::default(),
            wazuh: WazuhConfig::default(),
            logging: LoggingConfig::default(),
            notifications: NotificationsConfig::default(),
            performance: PerformanceConfig::default(),
            network: NetworkConfig::default(),
            hooks: HooksConfig::default(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            pacman: PacmanConfig::default(),
            aur: AurConfig::default(),
            system: SystemConfig::default(),
            security: SecurityConfig::default(),
            maintenance: MaintenanceConfig::default(),
            services: ServicesConfig::default(),
            vulnerability: VulnerabilityConfig::default(),
        }
    }
}

impl Default for PacmanConfig {
    fn default() -> Self {
        Self {
            no_confirm: false,
            verbose: true,
            check_space: true,
            download_timeout: 30,
            parallel_downloads: 5,
        }
    }
}

impl Default for AurConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            helper: "yay".to_string(),
            check_updates: true,
            build_timeout: 1800,
            pgp_verify: true,
        }
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            check_interval: 300,
            disk_usage_threshold: 85,
            memory_threshold: 90,
            load_threshold: 5.0,
            temp_threshold: 80,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            vulnerability_feeds: vec![
                "https://security.archlinux.org/json".to_string(),
                "https://nvd.nist.gov/feeds/json/cve/1.1/nvdcve-1.1-recent.json.gz".to_string(),
            ],
            check_interval: 3600,
            auto_patch: false,
            severity_threshold: "medium".to_string(),
        }
    }
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cleanup_cache: true,
            cleanup_logs: true,
            update_mirrorlist: true,
            vacuum_database: true,
        }
    }
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            monitor_critical: true,
            restart_failed: false,
            services_to_monitor: vec![
                "systemd-resolved".to_string(),
                "systemd-networkd".to_string(),
                "sshd".to_string(),
                "chronyd".to_string(),
            ],
        }
    }
}

impl Default for VulnerabilityConfig {
    fn default() -> Self {
        Self {
            database_url: "https://security.archlinux.org/issues.json".to_string(),
            update_interval: 21600,
            cache_duration: 86400,
            check_aur_packages: true,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("/var/lib/jarvis/jarvis.db"),
            encryption_key: "jarvis-default-key-change-me".to_string(),
            max_connections: 10,
            enable_wal_mode: true,
            enable_foreign_keys: true,
            cache_size_kb: 10240,
            page_size: 4096,
            vacuum_on_startup: false,
        }
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            listen_address: "127.0.0.1".to_string(),
            listen_port: 7419,
            enable_metrics: true,
            metrics_port: 9090,
            log_level: "info".to_string(),
            enable_systemd_notifications: true,
            health_check_interval_seconds: 300,
            maintenance_schedule: MaintenanceScheduleConfig::default(),
        }
    }
}

impl Default for MaintenanceScheduleConfig {
    fn default() -> Self {
        Self {
            auto_update: false,
            update_schedule: "0 2 * * 0".to_string(),
            auto_clean: true,
            clean_schedule: "0 3 * * 0".to_string(),
            security_scan_schedule: "0 1 * * *".to_string(),
        }
    }
}

impl Default for WazuhConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server: "127.0.0.1".to_string(),
            port: 1514,
            protocol: "tcp".to_string(),
            encryption: true,
            certificate_path: Some(PathBuf::from("/etc/jarvis/wazuh.crt")),
            key_path: Some(PathBuf::from("/etc/jarvis/wazuh.key")),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file: PathBuf::from("/var/log/jarvis/jarvis-arch.log"),
            max_size_mb: 100,
            max_files: 5,
            json_format: false,
        }
    }
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            email_enabled: false,
            email_to: Some("admin@localhost".to_string()),
            email_from: Some("jarvis@localhost".to_string()),
            webhook_enabled: false,
            webhook_url: Some("https://hooks.slack.com/your/webhook/url".to_string()),
            desktop_enabled: false,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_cpu_percent: 50,
            max_disk_io_mb_s: 100,
            nice_level: 10,
            enable_package_cache: true,
            cache_ttl_hours: 24,
            max_cache_size_mb: 100,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            retry_attempts: 3,
            user_agent: "Jarvis-Arch/0.1.0".to_string(),
        }
    }
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            pre_update: vec![],
            post_update: vec![],
            pre_install: vec![],
            post_install: vec![],
        }
    }
}