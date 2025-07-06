use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use dirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub llm: LLMConfig,
    pub system: SystemConfig,
    pub blockchain: Option<BlockchainConfig>,
    pub network: NetworkConfig,
    pub agents: AgentConfig,
    pub database_path: String,
    pub plugin_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub primary_provider: String,
    pub ollama_url: String,
    pub openai_api_key: Option<String>,
    pub claude_api_key: Option<String>,
    pub default_model: Option<String>,
    pub context_window: usize,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub arch_package_manager: String, // "pacman", "yay", "paru"
    pub dotfiles_path: Option<String>,
    pub homelab_config: Option<String>,
    pub gpu_enabled: bool,
    pub gpu_devices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    pub ghostchain: Option<GhostChainConfig>,
    pub ethereum: Option<EthereumConfig>,
    pub default_network: String,
    pub gas_optimization: bool,
    pub security_monitoring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostChainConfig {
    pub grpc_url: String,
    pub rpc_url: String, // Fallback JSON-RPC
    pub chain_id: u64,
    pub walletd_url: Option<String>,
    pub ghostbridge_url: Option<String>,
    pub zvm_url: Option<String>,
    pub use_tls: bool,
    pub ipv6_preferred: bool,
    pub connection_timeout_secs: u64,
    pub request_timeout_secs: u64,
    pub max_concurrent_streams: u32,
    pub zns_url: Option<String>,
}

impl Default for GhostChainConfig {
    fn default() -> Self {
        Self {
            grpc_url: "https://[::1]:9090".to_string(),
            rpc_url: "https://[::1]:8545".to_string(),
            chain_id: 1337,
            walletd_url: None,
            ghostbridge_url: None,
            zvm_url: None,
            use_tls: true,
            ipv6_preferred: true,
            connection_timeout_secs: 10,
            request_timeout_secs: 30,
            max_concurrent_streams: 100,
            zns_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub ipv6_enabled: bool,
    pub ipv6_preferred: bool,
    pub dns_over_https: bool,
    pub dns_servers: Vec<String>,
    pub optimization_level: NetworkOptimizationLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkOptimizationLevel {
    Conservative,
    Balanced,
    Aggressive,
    BlockchainOptimized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub transaction_monitor: TransactionMonitorConfig,
    pub contract_auditor: ContractAuditorConfig,
    pub gas_optimizer: GasOptimizerConfig,
    pub network_optimizer: NetworkOptimizerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMonitorConfig {
    pub enabled: bool,
    pub alert_threshold: AlertThreshold,
    pub auto_response: bool,
    pub batch_size: u32,
    pub stream_buffer_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAuditorConfig {
    pub enabled: bool,
    pub audit_frequency: String,
    pub ai_model: String,
    pub deep_analysis: bool,
    pub auto_maintenance: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasOptimizerConfig {
    pub enabled: bool,
    pub strategy: GasOptimizationStrategy,
    pub target_confirmation_time_secs: u32,
    pub max_gas_price_gwei: u64,
    pub auto_optimize: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkOptimizerConfig {
    pub enabled: bool,
    pub ipv6_optimization: bool,
    pub bandwidth_monitoring: bool,
    pub latency_optimization: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertThreshold {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GasOptimizationStrategy {
    Conservative,
    Balanced,
    Aggressive,
    MLBased,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumConfig {
    pub rpc_url: String,
    pub chain_id: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LLMConfig {
                primary_provider: "ollama".to_string(),
                ollama_url: "http://localhost:11434".to_string(),
                openai_api_key: None,
                claude_api_key: None,
                default_model: Some("llama3.1:8b".to_string()),
                context_window: 8192,
                temperature: 0.7,
            },
            system: SystemConfig {
                arch_package_manager: "pacman".to_string(),
                dotfiles_path: None,
                homelab_config: None,
                gpu_enabled: false,
                gpu_devices: vec![],
            },
            blockchain: Some(BlockchainConfig {
                ghostchain: Some(GhostChainConfig {
                    grpc_url: "https://[::1]:9090".to_string(),
                    rpc_url: "http://localhost:8545".to_string(),
                    chain_id: 31337,
                    walletd_url: Some("http://localhost:8080".to_string()),
                    ghostbridge_url: Some("http://localhost:8081".to_string()),
                    zvm_url: Some("http://localhost:8082".to_string()),
                    use_tls: true,
                    ipv6_preferred: true,
                    connection_timeout_secs: 10,
                    request_timeout_secs: 30,
                    max_concurrent_streams: 100,
                    zns_url: Some("http://localhost:8083".to_string()),
                }),
                ethereum: None,
                default_network: "ghostchain".to_string(),
                gas_optimization: true,
                security_monitoring: true,
            }),
            network: NetworkConfig {
                ipv6_enabled: true,
                ipv6_preferred: true,
                dns_over_https: true,
                dns_servers: vec![
                    "[2606:4700:4700::1111]".to_string(), // Cloudflare IPv6
                    "[2606:4700:4700::1001]".to_string(),
                    "1.1.1.1".to_string(), // Fallback IPv4
                    "1.0.0.1".to_string(),
                ],
                optimization_level: NetworkOptimizationLevel::BlockchainOptimized,
            },
            agents: AgentConfig {
                transaction_monitor: TransactionMonitorConfig {
                    enabled: true,
                    alert_threshold: AlertThreshold::Medium,
                    auto_response: true,
                    batch_size: 100,
                    stream_buffer_size: 1000,
                },
                contract_auditor: ContractAuditorConfig {
                    enabled: true,
                    audit_frequency: "hourly".to_string(),
                    ai_model: "llama3.1:8b".to_string(),
                    deep_analysis: true,
                    auto_maintenance: false,
                },
                gas_optimizer: GasOptimizerConfig {
                    enabled: true,
                    strategy: GasOptimizationStrategy::MLBased,
                    target_confirmation_time_secs: 15,
                    max_gas_price_gwei: 50,
                    auto_optimize: true,
                },
                network_optimizer: NetworkOptimizerConfig {
                    enabled: true,
                    ipv6_optimization: true,
                    bandwidth_monitoring: true,
                    latency_optimization: true,
                },
            },
            database_path: "~/.local/share/jarvis/memory.db".to_string(),
            plugin_paths: vec![
                "~/.config/jarvis/plugins".to_string(),
                "/usr/local/share/jarvis/plugins".to_string(),
            ],
        }
    }
}

impl Config {
    pub async fn load(config_path: Option<&str>) -> Result<Self> {
        let path = match config_path {
            Some(p) => PathBuf::from(p),
            None => {
                let config_dir = dirs::config_dir()
                    .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
                config_dir.join("jarvis").join("jarvis.toml")
            }
        };

        if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Create default config
            let config = Config::default();
            config.save(&path).await?;
            Ok(config)
        }
    }

    pub async fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let content = toml::to_string_pretty(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    pub async fn init() -> Result<()> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        let config_path = config_dir.join("jarvis").join("jarvis.toml");
        
        let config = Config::default();
        config.save(&config_path).await?;
        
        // Also create other directories
        let data_dir = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find data directory"))?;
        tokio::fs::create_dir_all(data_dir.join("jarvis")).await?;
        
        Ok(())
    }

    pub async fn set(key: &str, value: &str) -> Result<()> {
        // TODO: Implement dynamic config setting
        println!("Setting {} = {} (not implemented yet)", key, value);
        Ok(())
    }
}
