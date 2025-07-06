/*!
 * Configuration management for JARVIS-NV
 *
 * Handles configuration for GPU acceleration, node integration,
 * Web5 networking, and AI agent settings.
 */

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JarvisNvConfig {
    pub general: GeneralConfig,
    pub gpu: GpuConfig,
    pub node: NodeConfig,
    pub web5: Web5Config,
    pub bridge: BridgeConfig,
    pub agent: AgentConfig,
    pub metrics: MetricsConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub name: String,
    pub version: String,
    pub environment: String, // "development", "staging", "production"
    pub log_level: String,
    pub data_dir: String,
    pub pid_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    pub enabled: bool,
    pub device_id: u32,
    pub cuda_version: Option<String>,
    pub memory_limit_gb: Option<f32>,
    pub compute_mode: String, // "default", "exclusive", "prohibited"
    pub power_limit_watts: Option<u32>,
    pub fan_control: bool,
    pub temperature_limit_celsius: u32,
    pub benchmark_on_startup: bool,
    pub models: GpuModelsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuModelsConfig {
    pub inference_model: String,
    pub embedding_model: String,
    pub analysis_model: String,
    pub model_cache_dir: String,
    pub max_context_length: u32,
    pub batch_size: u32,
    pub precision: String, // "fp16", "fp32", "int8"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub ghostchain: GhostChainConfig,
    pub zvm: ZvmConfig,
    pub monitoring: NodeMonitoringConfig,
    pub integration: NodeIntegrationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostChainConfig {
    pub enabled: bool,
    pub node_url: String,
    pub ws_url: Option<String>,
    pub chain_id: u64,
    pub network_id: String,
    pub sync_mode: String, // "full", "fast", "light"
    pub api_key: Option<String>,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub block_confirmations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZvmConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub quic_endpoint: Option<String>,
    pub zns_resolver: String,
    pub web5_gateway: String,
    pub timeout_seconds: u64,
    pub cache_ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMonitoringConfig {
    pub enabled: bool,
    pub check_interval_seconds: u64,
    pub health_check_timeout_seconds: u64,
    pub performance_metrics: bool,
    pub transaction_monitoring: bool,
    pub block_monitoring: bool,
    pub peer_monitoring: bool,
    pub mempool_monitoring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIntegrationConfig {
    pub auto_restart_node: bool,
    pub node_optimization: bool,
    pub resource_management: bool,
    pub backup_management: bool,
    pub log_analysis: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Web5Config {
    pub enabled: bool,
    pub ipv6_preferred: bool,
    pub quic_enabled: bool,
    pub http3_enabled: bool,
    pub bind_address: String,
    pub port: u16,
    pub tls: TlsConfig,
    pub transport: TransportConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub ca_path: Option<String>,
    pub verify_certificates: bool,
    pub alpn_protocols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    pub max_connections: u32,
    pub connection_timeout_ms: u64,
    pub keep_alive_interval_ms: u64,
    pub max_idle_timeout_ms: u64,
    pub max_bi_streams: u32,
    pub max_uni_streams: u32,
    pub congestion_control: String, // "cubic", "bbr", "newreno"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub enabled: bool,
    pub grpc_endpoint: String,
    pub quic_endpoint: Option<String>,
    pub authentication: AuthConfig,
    pub rate_limiting: RateLimitConfig,
    pub load_balancing: LoadBalanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub method: String, // "none", "api_key", "jwt", "sigil"
    pub api_key: Option<String>,
    pub jwt_secret: Option<String>,
    pub sigil_config: Option<SigilConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigilConfig {
    pub identity_provider: String,
    pub public_key: String,
    pub verification_endpoint: String,
    pub trust_level: String, // "low", "medium", "high"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst_size: u32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalanceConfig {
    pub strategy: String, // "round_robin", "least_connections", "weighted"
    pub health_check_interval_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub enabled: bool,
    pub inference_interval_seconds: u64,
    pub analysis_depth: String, // "basic", "standard", "deep"
    pub auto_response: bool,
    pub learning_enabled: bool,
    pub model_updates: bool,

    // AI/LLM Configuration
    pub ollama_endpoint: Option<String>,
    pub default_ai_models: Option<Vec<String>>,
    pub max_context_tokens: Option<u32>,
    pub inference_timeout_seconds: Option<u64>,
    pub chat_session_timeout_minutes: Option<u64>,
    pub ai_temperature: Option<f32>,
    pub ai_max_tokens: Option<u32>,

    pub capabilities: AgentCapabilities,
    pub thresholds: AgentThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    pub anomaly_detection: bool,
    pub performance_optimization: bool,
    pub security_analysis: bool,
    pub predictive_analytics: bool,
    pub transaction_analysis: bool,
    pub network_optimization: bool,
    pub smart_contract_analysis: bool,
    pub zns_optimization: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentThresholds {
    pub anomaly_score_threshold: f32,
    pub performance_degradation_threshold: f32,
    pub security_risk_threshold: f32,
    pub resource_utilization_threshold: f32,
    pub response_time_threshold_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub prometheus_endpoint: String,
    pub collection_interval_seconds: u64,
    pub retention_days: u32,
    pub gpu_metrics: bool,
    pub node_metrics: bool,
    pub network_metrics: bool,
    pub agent_metrics: bool,
    pub export: MetricsExportConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsExportConfig {
    pub enabled: bool,
    pub format: String, // "prometheus", "influxdb", "elasticsearch"
    pub endpoint: Option<String>,
    pub batch_size: u32,
    pub flush_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enabled: bool,
    pub zero_trust: bool,
    pub encryption_at_rest: bool,
    pub encryption_in_transit: bool,
    pub audit_logging: bool,
    pub threat_detection: bool,
    pub guardian_integration: bool,
    pub access_control: AccessControlConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlConfig {
    pub default_policy: String, // "allow", "deny"
    pub admin_users: Vec<String>,
    pub api_permissions: Vec<ApiPermission>,
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPermission {
    pub user: String,
    pub endpoints: Vec<String>,
    pub methods: Vec<String>,
    pub rate_limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_gpu_memory_gb: f32,
    pub max_cpu_cores: u32,
    pub max_memory_gb: f32,
    pub max_disk_gb: f32,
    pub max_network_mbps: u32,
}

impl Default for JarvisNvConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                name: "jarvis-nv".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                environment: "development".to_string(),
                log_level: "info".to_string(),
                data_dir: "/var/lib/jarvis-nv".to_string(),
                pid_file: Some("/var/run/jarvis-nv.pid".to_string()),
            },
            gpu: GpuConfig {
                enabled: true,
                device_id: 0,
                cuda_version: None,
                memory_limit_gb: None,
                compute_mode: "default".to_string(),
                power_limit_watts: None,
                fan_control: false,
                temperature_limit_celsius: 83,
                benchmark_on_startup: true,
                models: GpuModelsConfig {
                    inference_model: "llama3.2:3b".to_string(),
                    embedding_model: "nomic-embed-text".to_string(),
                    analysis_model: "codellama:7b".to_string(),
                    model_cache_dir: "/var/lib/jarvis-nv/models".to_string(),
                    max_context_length: 4096,
                    batch_size: 32,
                    precision: "fp16".to_string(),
                },
            },
            node: NodeConfig {
                ghostchain: GhostChainConfig {
                    enabled: true,
                    node_url: "http://localhost:8545".to_string(),
                    ws_url: Some("ws://localhost:8546".to_string()),
                    chain_id: 1337,
                    network_id: "ghostchain-mainnet".to_string(),
                    sync_mode: "full".to_string(),
                    api_key: None,
                    timeout_seconds: 30,
                    retry_attempts: 3,
                    block_confirmations: 6,
                },
                zvm: ZvmConfig {
                    enabled: true,
                    endpoint: "http://localhost:8547".to_string(),
                    quic_endpoint: Some("quic://localhost:8548".to_string()),
                    zns_resolver: "zns://resolver.ghost".to_string(),
                    web5_gateway: "https://web5.ghost".to_string(),
                    timeout_seconds: 30,
                    cache_ttl_seconds: 300,
                },
                monitoring: NodeMonitoringConfig {
                    enabled: true,
                    check_interval_seconds: 30,
                    health_check_timeout_seconds: 10,
                    performance_metrics: true,
                    transaction_monitoring: true,
                    block_monitoring: true,
                    peer_monitoring: true,
                    mempool_monitoring: true,
                },
                integration: NodeIntegrationConfig {
                    auto_restart_node: false,
                    node_optimization: true,
                    resource_management: true,
                    backup_management: false,
                    log_analysis: true,
                },
            },
            web5: Web5Config {
                enabled: true,
                ipv6_preferred: true,
                quic_enabled: true,
                http3_enabled: true,
                bind_address: "::".to_string(), // IPv6 any
                port: 8080,
                tls: TlsConfig {
                    enabled: true,
                    cert_path: None,
                    key_path: None,
                    ca_path: None,
                    verify_certificates: true,
                    alpn_protocols: vec!["h3".to_string(), "h2".to_string()],
                },
                transport: TransportConfig {
                    max_connections: 1000,
                    connection_timeout_ms: 10000,
                    keep_alive_interval_ms: 30000,
                    max_idle_timeout_ms: 60000,
                    max_bi_streams: 100,
                    max_uni_streams: 100,
                    congestion_control: "bbr".to_string(),
                },
            },
            bridge: BridgeConfig {
                enabled: true,
                grpc_endpoint: "https://[::1]:9090".to_string(),
                quic_endpoint: Some("quic://[::1]:9091".to_string()),
                authentication: AuthConfig {
                    enabled: false,
                    method: "none".to_string(),
                    api_key: None,
                    jwt_secret: None,
                    sigil_config: None,
                },
                rate_limiting: RateLimitConfig {
                    enabled: true,
                    requests_per_second: 100,
                    burst_size: 200,
                    timeout_seconds: 60,
                },
                load_balancing: LoadBalanceConfig {
                    strategy: "round_robin".to_string(),
                    health_check_interval_seconds: 30,
                    max_retries: 3,
                },
            },
            agent: AgentConfig {
                enabled: true,
                inference_interval_seconds: 10,
                analysis_depth: "standard".to_string(),
                auto_response: false,
                learning_enabled: true,
                model_updates: true,

                // AI/LLM Configuration
                ollama_endpoint: Some("http://localhost:11434".to_string()),
                default_ai_models: Some(vec!["llama3.2:3b".to_string(), "qwen2.5:7b".to_string()]),
                max_context_tokens: Some(4096),
                inference_timeout_seconds: Some(30),
                chat_session_timeout_minutes: Some(60),
                ai_temperature: Some(0.7),
                ai_max_tokens: Some(1024),

                capabilities: AgentCapabilities {
                    anomaly_detection: true,
                    performance_optimization: true,
                    security_analysis: true,
                    predictive_analytics: true,
                    transaction_analysis: true,
                    network_optimization: true,
                    smart_contract_analysis: true,
                    zns_optimization: true,
                },
                thresholds: AgentThresholds {
                    anomaly_score_threshold: 0.8,
                    performance_degradation_threshold: 0.7,
                    security_risk_threshold: 0.9,
                    resource_utilization_threshold: 0.85,
                    response_time_threshold_ms: 1000,
                },
            },
            metrics: MetricsConfig {
                enabled: true,
                prometheus_endpoint: "http://[::1]:9090".to_string(),
                collection_interval_seconds: 15,
                retention_days: 7,
                gpu_metrics: true,
                node_metrics: true,
                network_metrics: true,
                agent_metrics: true,
                export: MetricsExportConfig {
                    enabled: false,
                    format: "prometheus".to_string(),
                    endpoint: None,
                    batch_size: 100,
                    flush_interval_seconds: 60,
                },
            },
            security: SecurityConfig {
                enabled: true,
                zero_trust: false,
                encryption_at_rest: true,
                encryption_in_transit: true,
                audit_logging: true,
                threat_detection: true,
                guardian_integration: false,
                access_control: AccessControlConfig {
                    default_policy: "allow".to_string(),
                    admin_users: vec!["jarvis-nv".to_string()],
                    api_permissions: vec![],
                    resource_limits: ResourceLimits {
                        max_gpu_memory_gb: 8.0,
                        max_cpu_cores: 8,
                        max_memory_gb: 32.0,
                        max_disk_gb: 100.0,
                        max_network_mbps: 1000,
                    },
                },
            },
        }
    }
}

impl JarvisNvConfig {
    /// Load configuration from file or create default
    pub async fn load(path: Option<&Path>) -> Result<Self> {
        let config_path = match path {
            Some(p) => p.to_path_buf(),
            None => {
                let config_dir = dirs::config_dir().context("Could not find config directory")?;
                config_dir.join("jarvis-nv").join("config.toml")
            }
        };

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .await
                .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

            let config: Self = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;

            Ok(config)
        } else {
            // Create default config
            let config = Self::default();
            config
                .save(&config_path)
                .await
                .context("Failed to save default config")?;
            Ok(config)
        }
    }

    /// Save configuration to file
    pub async fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(path, content)
            .await
            .with_context(|| format!("Failed to write config file: {:?}", path))?;

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate GPU configuration
        if self.gpu.enabled && self.gpu.device_id > 16 {
            anyhow::bail!("Invalid GPU device ID: {}", self.gpu.device_id);
        }

        // Validate network configuration
        if self.web5.port == 0 || self.web5.port > 65535 {
            anyhow::bail!("Invalid port number: {}", self.web5.port);
        }

        // Validate agent thresholds
        if self.agent.thresholds.anomaly_score_threshold < 0.0
            || self.agent.thresholds.anomaly_score_threshold > 1.0
        {
            anyhow::bail!(
                "Invalid anomaly score threshold: {}",
                self.agent.thresholds.anomaly_score_threshold
            );
        }

        Ok(())
    }
}
