use crate::{BlockchainConfig, LLMProviderConfig, NetworkOptimizationConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Unified configuration for the GhostFlow integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostFlowConfig {
    /// Server configuration
    pub server: ServerConfig,
    
    /// Database configuration
    pub database: DatabaseConfig,
    
    /// LLM provider configurations
    pub llm_providers: Vec<LLMProviderConfig>,
    
    /// Blockchain network configurations
    pub blockchain_networks: Vec<BlockchainConfig>,
    
    /// Network optimization settings
    pub network: NetworkOptimizationConfig,
    
    /// Memory and caching settings
    pub memory: MemoryConfig,
    
    /// Agent orchestration settings
    pub orchestration: OrchestrationConfig,
    
    /// Integration settings for external tools
    pub integrations: IntegrationConfig,
    
    /// Security and encryption settings
    pub security: SecurityConfig,
    
    /// Logging and monitoring settings
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to
    pub host: String,
    
    /// Port to bind to
    pub port: u16,
    
    /// Enable HTTPS/TLS
    pub enable_tls: bool,
    
    /// TLS certificate path
    pub tls_cert_path: Option<PathBuf>,
    
    /// TLS private key path
    pub tls_key_path: Option<PathBuf>,
    
    /// Enable CORS
    pub enable_cors: bool,
    
    /// Allowed CORS origins
    pub cors_origins: Vec<String>,
    
    /// Enable WebSocket support
    pub enable_websocket: bool,
    
    /// Enable metrics endpoint
    pub enable_metrics: bool,
    
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    
    /// Max request body size in bytes
    pub max_request_size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL/connection string
    pub url: String,
    
    /// Enable ZQLite for post-quantum security
    pub enable_zqlite: bool,
    
    /// ZQLite configuration file path
    pub zqlite_config_path: Option<PathBuf>,
    
    /// Maximum number of database connections
    pub max_connections: u32,
    
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    
    /// Enable database migrations
    pub enable_migrations: bool,
    
    /// Enable connection pooling
    pub enable_pooling: bool,
    
    /// Enable read replicas
    pub read_replicas: Vec<String>,
    
    /// Enable encryption at rest
    pub enable_encryption: bool,
    
    /// Backup configuration
    pub backup: BackupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable automated backups
    pub enable_automated_backup: bool,
    
    /// Backup interval in hours
    pub backup_interval_hours: u64,
    
    /// Backup retention days
    pub retention_days: u32,
    
    /// Backup storage path
    pub backup_path: PathBuf,
    
    /// Enable compression
    pub enable_compression: bool,
    
    /// Enable encryption
    pub enable_encryption: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum memory cache size in MB
    pub max_cache_size_mb: u64,
    
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    
    /// Enable persistent memory across restarts
    pub enable_persistent_memory: bool,
    
    /// Memory storage path
    pub memory_storage_path: PathBuf,
    
    /// Enable semantic search
    pub enable_semantic_search: bool,
    
    /// Maximum context entries per workflow
    pub max_context_entries: usize,
    
    /// Memory cleanup interval in seconds
    pub cleanup_interval_seconds: u64,
    
    /// Enable memory compression
    pub enable_compression: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationConfig {
    /// Maximum concurrent agents
    pub max_concurrent_agents: usize,
    
    /// Agent timeout in seconds
    pub agent_timeout_seconds: u64,
    
    /// Enable auto-recovery of failed agents
    pub enable_auto_recovery: bool,
    
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
    
    /// Enable load balancing
    pub enable_load_balancing: bool,
    
    /// Enable priority scheduling
    pub enable_priority_scheduling: bool,
    
    /// Resource limits
    pub resource_limits: ResourceLimitsConfig,
    
    /// Enable agent metrics collection
    pub enable_metrics_collection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimitsConfig {
    /// Maximum memory per agent in MB
    pub max_memory_per_agent_mb: u64,
    
    /// Maximum CPU cores per agent
    pub max_cpu_cores_per_agent: u32,
    
    /// Maximum network connections per agent
    pub max_network_connections_per_agent: u32,
    
    /// Maximum tokens per minute per agent
    pub max_tokens_per_minute_per_agent: u64,
    
    /// Maximum concurrent tasks per agent
    pub max_concurrent_tasks_per_agent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// GhostLLM integration settings
    pub ghostllm: GhostLLMConfig,
    
    /// Zeke integration settings
    pub zeke: ZekeConfig,
    
    /// ZQLite integration settings
    pub zqlite: ZQLiteConfig,
    
    /// External API integrations
    pub external_apis: HashMap<String, ExternalApiConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostLLMConfig {
    /// Enable GhostLLM integration
    pub enabled: bool,
    
    /// GhostLLM server URL
    pub server_url: String,
    
    /// API key for authentication
    pub api_key: Option<String>,
    
    /// Enable GPU acceleration
    pub enable_gpu_acceleration: bool,
    
    /// CUDA device IDs to use
    pub cuda_devices: Vec<u32>,
    
    /// Maximum concurrent requests
    pub max_concurrent_requests: u32,
    
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    
    /// Enable model caching
    pub enable_model_caching: bool,
    
    /// Model cache size in GB
    pub model_cache_size_gb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZekeConfig {
    /// Enable Zeke integration
    pub enabled: bool,
    
    /// Zeke executable path
    pub executable_path: PathBuf,
    
    /// Enable development workflow automation
    pub enable_dev_automation: bool,
    
    /// Supported programming languages
    pub supported_languages: Vec<String>,
    
    /// Enable code completion
    pub enable_code_completion: bool,
    
    /// Enable code analysis
    pub enable_code_analysis: bool,
    
    /// Enable refactoring suggestions
    pub enable_refactoring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZQLiteConfig {
    /// Enable ZQLite integration
    pub enabled: bool,
    
    /// ZQLite library path
    pub library_path: PathBuf,
    
    /// Enable post-quantum cryptography
    pub enable_post_quantum_crypto: bool,
    
    /// Enable zero-knowledge proofs
    pub enable_zero_knowledge: bool,
    
    /// Enable field-level encryption
    pub enable_field_encryption: bool,
    
    /// Cryptographic algorithm selection
    pub crypto_algorithm: String,
    
    /// Key derivation settings
    pub key_derivation: KeyDerivationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationConfig {
    /// PBKDF2 iterations
    pub pbkdf2_iterations: u32,
    
    /// Salt length in bytes
    pub salt_length: usize,
    
    /// Key length in bytes
    pub key_length: usize,
    
    /// Hash algorithm
    pub hash_algorithm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalApiConfig {
    /// Base URL for the API
    pub base_url: String,
    
    /// API key or token
    pub api_key: Option<String>,
    
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    
    /// Maximum retry attempts
    pub max_retries: u32,
    
    /// Rate limiting settings
    pub rate_limit: RateLimitConfig,
    
    /// Custom headers
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per second
    pub requests_per_second: u32,
    
    /// Burst capacity
    pub burst_capacity: u32,
    
    /// Enable rate limiting
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication
    pub enable_authentication: bool,
    
    /// Authentication method
    pub auth_method: AuthMethod,
    
    /// JWT settings
    pub jwt: JwtConfig,
    
    /// API key settings
    pub api_keys: ApiKeyConfig,
    
    /// Enable encryption
    pub enable_encryption: bool,
    
    /// Encryption algorithm
    pub encryption_algorithm: String,
    
    /// Enable request signing
    pub enable_request_signing: bool,
    
    /// Enable audit logging
    pub enable_audit_logging: bool,
    
    /// Audit log path
    pub audit_log_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    None,
    ApiKey,
    JWT,
    OAuth2,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret key
    pub secret_key: String,
    
    /// Token expiration time in seconds
    pub expiration_seconds: u64,
    
    /// Enable refresh tokens
    pub enable_refresh_tokens: bool,
    
    /// Refresh token expiration in seconds
    pub refresh_expiration_seconds: u64,
    
    /// JWT issuer
    pub issuer: String,
    
    /// JWT audience
    pub audience: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// Valid API keys
    pub valid_keys: Vec<String>,
    
    /// API key header name
    pub header_name: String,
    
    /// Enable key rotation
    pub enable_rotation: bool,
    
    /// Key rotation interval in days
    pub rotation_interval_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    pub enable_metrics: bool,
    
    /// Metrics export endpoint
    pub metrics_endpoint: String,
    
    /// Enable distributed tracing
    pub enable_tracing: bool,
    
    /// Tracing endpoint
    pub tracing_endpoint: Option<String>,
    
    /// Log level
    pub log_level: String,
    
    /// Log format
    pub log_format: LogFormat,
    
    /// Log output destination
    pub log_output: LogOutput,
    
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    
    /// Performance metrics interval in seconds
    pub performance_metrics_interval_seconds: u64,
    
    /// Enable alerting
    pub enable_alerting: bool,
    
    /// Alert configurations
    pub alerts: Vec<AlertConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Text,
    Structured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogOutput {
    Console,
    File(PathBuf),
    Syslog,
    CloudWatch,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Alert name
    pub name: String,
    
    /// Metric to monitor
    pub metric: String,
    
    /// Threshold value
    pub threshold: f64,
    
    /// Comparison operator
    pub operator: ComparisonOperator,
    
    /// Alert destination
    pub destination: AlertDestination,
    
    /// Enable alert
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertDestination {
    Email(String),
    Slack(String),
    Webhook(String),
    Console,
}

impl Default for GhostFlowConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            llm_providers: vec![
                LLMProviderConfig {
                    provider: "ollama".to_string(),
                    model: "llama3.1:8b".to_string(),
                    api_key: None,
                    base_url: Some("http://localhost:11434".to_string()),
                    max_tokens: Some(4096),
                    temperature: Some(0.7),
                    context_window: 8192,
                    cost_per_token: 0.0,
                    priority: 1,
                }
            ],
            blockchain_networks: vec![BlockchainConfig::default()],
            network: NetworkOptimizationConfig::default(),
            memory: MemoryConfig::default(),
            orchestration: OrchestrationConfig::default(),
            integrations: IntegrationConfig::default(),
            security: SecurityConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
            enable_websocket: true,
            enable_metrics: true,
            request_timeout_seconds: 30,
            max_request_size_bytes: 10 * 1024 * 1024, // 10MB
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite:ghostflow.db".to_string(),
            enable_zqlite: false,
            zqlite_config_path: None,
            max_connections: 10,
            connection_timeout_seconds: 30,
            enable_migrations: true,
            enable_pooling: true,
            read_replicas: vec![],
            enable_encryption: true,
            backup: BackupConfig::default(),
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enable_automated_backup: false,
            backup_interval_hours: 24,
            retention_days: 30,
            backup_path: PathBuf::from("./backups"),
            enable_compression: true,
            enable_encryption: true,
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_cache_size_mb: 512,
            cache_ttl_seconds: 3600,
            enable_persistent_memory: true,
            memory_storage_path: PathBuf::from("./memory"),
            enable_semantic_search: true,
            max_context_entries: 10000,
            cleanup_interval_seconds: 300,
            enable_compression: true,
        }
    }
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_agents: 10,
            agent_timeout_seconds: 300,
            enable_auto_recovery: true,
            health_check_interval_seconds: 30,
            enable_load_balancing: true,
            enable_priority_scheduling: true,
            resource_limits: ResourceLimitsConfig::default(),
            enable_metrics_collection: true,
        }
    }
}

impl Default for ResourceLimitsConfig {
    fn default() -> Self {
        Self {
            max_memory_per_agent_mb: 1024,
            max_cpu_cores_per_agent: 2,
            max_network_connections_per_agent: 100,
            max_tokens_per_minute_per_agent: 1000,
            max_concurrent_tasks_per_agent: 5,
        }
    }
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            ghostllm: GhostLLMConfig::default(),
            zeke: ZekeConfig::default(),
            zqlite: ZQLiteConfig::default(),
            external_apis: HashMap::new(),
        }
    }
}

impl Default for GhostLLMConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_url: "http://localhost:8081".to_string(),
            api_key: None,
            enable_gpu_acceleration: true,
            cuda_devices: vec![0],
            max_concurrent_requests: 10,
            request_timeout_seconds: 60,
            enable_model_caching: true,
            model_cache_size_gb: 10,
        }
    }
}

impl Default for ZekeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            executable_path: PathBuf::from("/usr/local/bin/zeke"),
            enable_dev_automation: true,
            supported_languages: vec!["rust".to_string(), "python".to_string(), "javascript".to_string()],
            enable_code_completion: true,
            enable_code_analysis: true,
            enable_refactoring: true,
        }
    }
}

impl Default for ZQLiteConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            library_path: PathBuf::from("/usr/local/lib/libzqlite.so"),
            enable_post_quantum_crypto: true,
            enable_zero_knowledge: false,
            enable_field_encryption: true,
            crypto_algorithm: "ML-KEM-768".to_string(),
            key_derivation: KeyDerivationConfig::default(),
        }
    }
}

impl Default for KeyDerivationConfig {
    fn default() -> Self {
        Self {
            pbkdf2_iterations: 100000,
            salt_length: 32,
            key_length: 32,
            hash_algorithm: "SHA-256".to_string(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_authentication: false,
            auth_method: AuthMethod::None,
            jwt: JwtConfig::default(),
            api_keys: ApiKeyConfig::default(),
            enable_encryption: true,
            encryption_algorithm: "AES-256-GCM".to_string(),
            enable_request_signing: false,
            enable_audit_logging: true,
            audit_log_path: PathBuf::from("./logs/audit.log"),
        }
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret_key: "change-me-in-production".to_string(),
            expiration_seconds: 3600,
            enable_refresh_tokens: true,
            refresh_expiration_seconds: 86400,
            issuer: "ghostflow".to_string(),
            audience: "ghostflow-users".to_string(),
        }
    }
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            valid_keys: vec![],
            header_name: "X-API-Key".to_string(),
            enable_rotation: false,
            rotation_interval_days: 90,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            metrics_endpoint: "/metrics".to_string(),
            enable_tracing: false,
            tracing_endpoint: None,
            log_level: "info".to_string(),
            log_format: LogFormat::Json,
            log_output: LogOutput::Console,
            enable_performance_monitoring: true,
            performance_metrics_interval_seconds: 60,
            enable_alerting: false,
            alerts: vec![],
        }
    }
}

/// Load configuration from file or environment variables
impl GhostFlowConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from environment variables with optional file override
    pub fn from_env_with_file(file_path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = if let Some(path) = file_path {
            Self::from_file(path)?
        } else {
            Self::default()
        };

        // Override with environment variables
        config.apply_env_overrides();
        
        Ok(config)
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) {
        if let Ok(host) = std::env::var("GHOSTFLOW_HOST") {
            self.server.host = host;
        }
        
        if let Ok(port) = std::env::var("GHOSTFLOW_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                self.server.port = port_num;
            }
        }
        
        if let Ok(db_url) = std::env::var("GHOSTFLOW_DATABASE_URL") {
            self.database.url = db_url;
        }
        
        if let Ok(log_level) = std::env::var("GHOSTFLOW_LOG_LEVEL") {
            self.monitoring.log_level = log_level;
        }
        
        // Add more environment variable overrides as needed
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate server configuration
        if self.server.port == 0 {
            return Err("Server port must be greater than 0".to_string());
        }
        
        // Validate database configuration
        if self.database.url.is_empty() {
            return Err("Database URL cannot be empty".to_string());
        }
        
        // Validate LLM providers
        if self.llm_providers.is_empty() {
            return Err("At least one LLM provider must be configured".to_string());
        }
        
        // Validate resource limits
        if self.orchestration.max_concurrent_agents == 0 {
            return Err("max_concurrent_agents must be greater than 0".to_string());
        }
        
        Ok(())
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}