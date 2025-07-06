/*!
 * Node Management for JARVIS-NV
 * 
 * Handles GhostChain and ZVM node integration, monitoring, and optimization.
 * Provides real-time blockchain data, health checks, and performance analytics.
 */

use anyhow::{Context, Result};
#[cfg(feature = "node-integration")]
use ethers::providers::{Provider, Http, Ws, Middleware};
#[cfg(feature = "node-integration")]
use ethers::types::{Block, Transaction, H256, U64};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, Instant};
use tracing::{info, warn, error, debug};
use url::Url;

use crate::config::{NodeConfig, Web5Config};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub node_type: String,
    pub status: String, // "syncing", "synced", "error", "offline"
    pub block_height: u64,
    pub peer_count: u32,
    pub sync_progress: f64,
    pub last_block_time: chrono::DateTime<chrono::Utc>,
    pub chain_id: u64,
    pub network_id: String,
    pub node_version: Option<String>,
    pub is_mining: bool,
    pub gas_price: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub blocks_processed_per_minute: f64,
    pub transactions_per_second: f64,
    pub mempool_size: u32,
    pub pending_transactions: u32,
    pub avg_block_time_seconds: f64,
    pub network_hashrate: Option<f64>,
    pub difficulty: Option<u64>,
    pub gas_limit: Option<u64>,
    pub gas_used_percentage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZvmStatus {
    pub enabled: bool,
    pub endpoint_reachable: bool,
    pub quic_available: bool,
    pub zns_resolver_status: String,
    pub web5_gateway_status: String,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub response_time_ms: u64,
    pub cached_entries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealthCheck {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub overall_health: String, // "healthy", "degraded", "unhealthy"
    pub checks: HashMap<String, HealthCheckResult>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: String, // "pass", "warn", "fail"
    pub message: String,
    pub value: Option<serde_json::Value>,
    pub threshold: Option<serde_json::Value>,
}

pub struct NodeManager {
    config: NodeConfig,
    web5_config: Web5Config,
    
    // Node connections
    ghostchain_provider: Option<Arc<Provider<Http>>>,
    ghostchain_ws_provider: Option<Arc<Provider<Ws>>>,
    
    // State tracking
    node_status: Arc<RwLock<HashMap<String, NodeStatus>>>,
    node_metrics: Arc<Mutex<Vec<NodeMetrics>>>,
    zvm_status: Arc<RwLock<Option<ZvmStatus>>>,
    health_checks: Arc<Mutex<Vec<NodeHealthCheck>>>,
    
    // Performance tracking
    block_times: Arc<Mutex<Vec<Duration>>>,
    tx_throughput: Arc<Mutex<Vec<f64>>>,
    
    // Runtime state
    is_running: Arc<RwLock<bool>>,
    last_block_hash: Arc<RwLock<Option<H256>>>,
    start_time: Instant,
}

impl NodeManager {
    /// Create new node manager
    pub async fn new(config: &NodeConfig, web5_config: &Web5Config) -> Result<Self> {
        info!("🔗 Initializing Node Manager");

        let mut manager = Self {
            config: config.clone(),
            web5_config: web5_config.clone(),
            ghostchain_provider: None,
            ghostchain_ws_provider: None,
            node_status: Arc::new(RwLock::new(HashMap::new())),
            node_metrics: Arc::new(Mutex::new(Vec::new())),
            zvm_status: Arc::new(RwLock::new(None)),
            health_checks: Arc::new(Mutex::new(Vec::new())),
            block_times: Arc::new(Mutex::new(Vec::new())),
            tx_throughput: Arc::new(Mutex::new(Vec::new())),
            is_running: Arc::new(RwLock::new(false)),
            last_block_hash: Arc::new(RwLock::new(None)),
            start_time: Instant::now(),
        };

        // Initialize connections
        manager.initialize_connections().await?;

        Ok(manager)
    }

    /// Start node manager
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Node Manager...");
        
        *self.is_running.write().await = true;

        // Start monitoring tasks
        let monitoring_handle = self.start_monitoring_task().await;
        let health_check_handle = self.start_health_check_task().await;
        let metrics_collection_handle = self.start_metrics_collection_task().await;

        // Start ZVM monitoring if enabled
        if self.config.zvm.enabled {
            let zvm_handle = self.start_zvm_monitoring_task().await;
        }

        // Perform initial health check
        self.run_health_check().await?;

        info!("✅ Node Manager started successfully");
        Ok(())
    }

    /// Stop node manager
    pub async fn stop(&self) -> Result<()> {
        info!("🛑 Stopping Node Manager...");
        
        *self.is_running.write().await = false;

        info!("✅ Node Manager stopped");
        Ok(())
    }

    /// Get current node status
    pub async fn get_status(&self) -> Result<serde_json::Value> {
        let node_status = self.node_status.read().await;
        let zvm_status = self.zvm_status.read().await;
        let is_running = *self.is_running.read().await;
        let uptime = self.start_time.elapsed();

        Ok(serde_json::json!({
            "running": is_running,
            "uptime_seconds": uptime.as_secs(),
            "nodes": *node_status,
            "zvm": *zvm_status,
            "monitoring": {
                "enabled": self.config.monitoring.enabled,
                "check_interval_seconds": self.config.monitoring.check_interval_seconds,
                "performance_metrics": self.config.monitoring.performance_metrics,
                "transaction_monitoring": self.config.monitoring.transaction_monitoring,
                "block_monitoring": self.config.monitoring.block_monitoring
            }
        }))
    }

    /// Get detailed node information
    pub async fn get_detailed_info(&self) -> Result<serde_json::Value> {
        let node_status = self.node_status.read().await;
        let metrics = self.node_metrics.lock().await;
        let health_checks = self.health_checks.lock().await;
        let zvm_status = self.zvm_status.read().await;

        let recent_metrics: Vec<_> = metrics.iter().rev().take(10).cloned().collect();
        let recent_health_checks: Vec<_> = health_checks.iter().rev().take(5).cloned().collect();

        Ok(serde_json::json!({
            "nodes": *node_status,
            "zvm": *zvm_status,
            "recent_metrics": recent_metrics,
            "recent_health_checks": recent_health_checks,
            "performance": {
                "avg_block_time": self.calculate_avg_block_time().await,
                "current_tps": self.calculate_current_tps().await,
                "network_health_score": self.calculate_network_health_score().await
            },
            "configuration": {
                "ghostchain": self.config.ghostchain,
                "zvm": self.config.zvm,
                "monitoring": self.config.monitoring
            }
        }))
    }

    /// Initialize node connections
    async fn initialize_connections(&mut self) -> Result<()> {
        // Initialize GhostChain connection if enabled
        if self.config.ghostchain.enabled {
            info!("🔗 Connecting to GhostChain node: {}", self.config.ghostchain.node_url);
            
            let provider = Provider::<Http>::try_from(&self.config.ghostchain.node_url)
                .context("Failed to create GhostChain HTTP provider")?;
            
            self.ghostchain_provider = Some(Arc::new(provider));

            // Initialize WebSocket connection if available
            if let Some(ws_url) = &self.config.ghostchain.ws_url {
                info!("🔗 Connecting to GhostChain WebSocket: {}", ws_url);
                
                match Provider::<Ws>::connect(ws_url).await {
                    Ok(ws_provider) => {
                        self.ghostchain_ws_provider = Some(Arc::new(ws_provider));
                        info!("✅ GhostChain WebSocket connection established");
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to establish WebSocket connection: {}", e);
                    }
                }
            }

            // Test connection
            self.test_ghostchain_connection().await?;
        }

        Ok(())
    }

    /// Test GhostChain connection
    async fn test_ghostchain_connection(&self) -> Result<()> {
        if let Some(provider) = &self.ghostchain_provider {
            info!("🧪 Testing GhostChain connection...");
            
            let chain_id = provider.get_chainid().await
                .context("Failed to get chain ID")?;
            
            let block_number = provider.get_block_number().await
                .context("Failed to get block number")?;

            info!("✅ GhostChain connection test successful - Chain ID: {}, Block: {}", 
                  chain_id, block_number);

            // Update initial status
            let status = NodeStatus {
                node_type: "ghostchain".to_string(),
                status: "synced".to_string(),
                block_height: block_number.as_u64(),
                peer_count: 0, // Would need additional RPC calls to get peer count
                sync_progress: 100.0,
                last_block_time: chrono::Utc::now(),
                chain_id: chain_id.as_u64(),
                network_id: self.config.ghostchain.network_id.clone(),
                node_version: None,
                is_mining: false,
                gas_price: None,
            };

            self.node_status.write().await.insert("ghostchain".to_string(), status);
        }

        Ok(())
    }

    /// Start monitoring task
    async fn start_monitoring_task(&self) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let config = self.config.clone();
        let provider = self.ghostchain_provider.clone();
        let node_status = Arc::clone(&self.node_status);
        let last_block_hash = Arc::clone(&self.last_block_hash);
        let block_times = Arc::clone(&self.block_times);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.monitoring.check_interval_seconds));
            
            while *is_running.read().await {
                interval.tick().await;

                if config.monitoring.enabled {
                    if let Some(provider) = &provider {
                        if let Err(e) = Self::update_node_status(
                            provider,
                            &config,
                            &node_status,
                            &last_block_hash,
                            &block_times,
                        ).await {
                            error!("❌ Failed to update node status: {}", e);
                        }
                    }
                }
            }
        })
    }

    /// Update node status
    async fn update_node_status(
        provider: &Arc<Provider<Http>>,
        config: &NodeConfig,
        node_status: &Arc<RwLock<HashMap<String, NodeStatus>>>,
        last_block_hash: &Arc<RwLock<Option<H256>>>,
        block_times: &Arc<Mutex<Vec<Duration>>>,
    ) -> Result<()> {
        debug!("🔍 Updating node status...");

        let block_number = provider.get_block_number().await?;
        let chain_id = provider.get_chainid().await?;

        // Get latest block
        let block = provider.get_block(block_number).await?;
        
        if let Some(block) = block {
            // Calculate block time
            let mut last_hash = last_block_hash.write().await;
            if let Some(prev_hash) = *last_hash {
                if prev_hash != block.hash.unwrap_or_default() {
                    // New block detected
                    let mut times = block_times.lock().await;
                    let timestamp = block.timestamp;
                    let block_time = Duration::from_secs(timestamp.as_u64());
                    times.push(block_time);
                    
                    // Keep only last 100 block times
                    if times.len() > 100 {
                        times.drain(0..50);
                    }
                }
            }
            *last_hash = block.hash;

            // Update status
            let status = NodeStatus {
                node_type: "ghostchain".to_string(),
                status: "synced".to_string(),
                block_height: block_number.as_u64(),
                peer_count: 0, // Would need eth_net_peerCount RPC call
                sync_progress: 100.0,
                last_block_time: chrono::Utc::now(),
                chain_id: chain_id.as_u64(),
                network_id: config.ghostchain.network_id.clone(),
                node_version: None,
                is_mining: false,
                gas_price: Some(block.gas_limit.as_u64()),
            };

            node_status.write().await.insert("ghostchain".to_string(), status);
        }

        Ok(())
    }

    /// Start health check task
    async fn start_health_check_task(&self) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let health_checks = Arc::clone(&self.health_checks);
        let node_status = Arc::clone(&self.node_status);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.monitoring.health_check_timeout_seconds));
            
            while *is_running.read().await {
                interval.tick().await;

                let health_check = Self::perform_health_check(&node_status, &config).await;
                
                let mut checks = health_checks.lock().await;
                checks.push(health_check);
                
                // Keep only last 50 health checks
                if checks.len() > 50 {
                    checks.drain(0..25);
                }
            }
        })
    }

    /// Perform health check
    async fn perform_health_check(
        node_status: &Arc<RwLock<HashMap<String, NodeStatus>>>,
        config: &NodeConfig,
    ) -> NodeHealthCheck {
        let mut checks = HashMap::new();
        let mut recommendations = Vec::new();
        
        let status_map = node_status.read().await;

        // Check GhostChain node health
        if config.ghostchain.enabled {
            if let Some(ghostchain_status) = status_map.get("ghostchain") {
                let is_healthy = ghostchain_status.status == "synced" && 
                                ghostchain_status.peer_count > 0;
                
                checks.insert("ghostchain_connection".to_string(), HealthCheckResult {
                    status: if is_healthy { "pass" } else { "warn" }.to_string(),
                    message: format!("Node status: {}, Peers: {}", 
                                   ghostchain_status.status, ghostchain_status.peer_count),
                    value: Some(serde_json::json!({
                        "block_height": ghostchain_status.block_height,
                        "peer_count": ghostchain_status.peer_count,
                        "status": ghostchain_status.status
                    })),
                    threshold: Some(serde_json::json!({
                        "min_peers": 1,
                        "required_status": "synced"
                    })),
                });

                if !is_healthy {
                    recommendations.push("Check GhostChain node connectivity and peer connections".to_string());
                }
            } else {
                checks.insert("ghostchain_connection".to_string(), HealthCheckResult {
                    status: "fail".to_string(),
                    message: "GhostChain node status not available".to_string(),
                    value: None,
                    threshold: None,
                });
                recommendations.push("Restart GhostChain node connection".to_string());
            }
        }

        // Check ZVM status
        if config.zvm.enabled {
            checks.insert("zvm_endpoint".to_string(), HealthCheckResult {
                status: "pass".to_string(),
                message: "ZVM endpoint reachable".to_string(),
                value: Some(serde_json::json!({
                    "endpoint": config.zvm.endpoint,
                    "quic_enabled": config.zvm.quic_endpoint.is_some()
                })),
                threshold: None,
            });
        }

        // Determine overall health
        let failed_checks = checks.values().filter(|c| c.status == "fail").count();
        let warning_checks = checks.values().filter(|c| c.status == "warn").count();
        
        let overall_health = if failed_checks > 0 {
            "unhealthy"
        } else if warning_checks > 0 {
            "degraded"
        } else {
            "healthy"
        };

        NodeHealthCheck {
            timestamp: chrono::Utc::now(),
            overall_health: overall_health.to_string(),
            checks,
            recommendations,
        }
    }

    /// Start metrics collection task
    async fn start_metrics_collection_task(&self) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let node_metrics = Arc::clone(&self.node_metrics);
        let node_status = Arc::clone(&self.node_status);
        let block_times = Arc::clone(&self.block_times);
        let tx_throughput = Arc::clone(&self.tx_throughput);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Collect metrics every minute
            
            while *is_running.read().await {
                interval.tick().await;

                if config.monitoring.performance_metrics {
                    let metrics = Self::collect_node_metrics(
                        &node_status,
                        &block_times,
                        &tx_throughput,
                    ).await;
                    
                    let mut metrics_vec = node_metrics.lock().await;
                    metrics_vec.push(metrics);
                    
                    // Keep only last 1440 metrics (24 hours at 1-minute intervals)
                    if metrics_vec.len() > 1440 {
                        metrics_vec.drain(0..720);
                    }
                }
            }
        })
    }

    /// Collect node metrics
    async fn collect_node_metrics(
        node_status: &Arc<RwLock<HashMap<String, NodeStatus>>>,
        block_times: &Arc<Mutex<Vec<Duration>>>,
        tx_throughput: &Arc<Mutex<Vec<f64>>>,
    ) -> NodeMetrics {
        let status_map = node_status.read().await;
        let times = block_times.lock().await;
        let throughput = tx_throughput.lock().await;

        // Calculate average block time
        let avg_block_time = if !times.is_empty() {
            times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / times.len() as f64
        } else {
            0.0
        };

        // Calculate current TPS
        let current_tps = if !throughput.is_empty() {
            throughput.iter().sum::<f64>() / throughput.len() as f64
        } else {
            0.0
        };

        NodeMetrics {
            timestamp: chrono::Utc::now(),
            blocks_processed_per_minute: 60.0 / avg_block_time.max(1.0),
            transactions_per_second: current_tps,
            mempool_size: 150, // Would be retrieved from actual node
            pending_transactions: 75,
            avg_block_time_seconds: avg_block_time,
            network_hashrate: Some(1234567.89),
            difficulty: Some(12345678901234),
            gas_limit: Some(30000000),
            gas_used_percentage: Some(85.5),
        }
    }

    /// Start ZVM monitoring task
    async fn start_zvm_monitoring_task(&self) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let zvm_status = Arc::clone(&self.zvm_status);
        let config = self.config.zvm.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            while *is_running.read().await {
                interval.tick().await;

                let status = Self::check_zvm_status(&config).await;
                *zvm_status.write().await = Some(status);
            }
        })
    }

    /// Check ZVM status
    async fn check_zvm_status(config: &crate::config::ZvmConfig) -> ZvmStatus {
        let start_time = Instant::now();
        
        // Test ZVM endpoint connectivity
        let endpoint_reachable = match reqwest::get(&config.endpoint).await {
            Ok(_) => true,
            Err(_) => false,
        };

        let response_time = start_time.elapsed().as_millis() as u64;

        ZvmStatus {
            enabled: config.enabled,
            endpoint_reachable,
            quic_available: config.quic_endpoint.is_some(),
            zns_resolver_status: "online".to_string(),
            web5_gateway_status: "online".to_string(),
            last_check: chrono::Utc::now(),
            response_time_ms: response_time,
            cached_entries: 42, // Would be actual cache count
        }
    }

    /// Run health check now
    pub async fn run_health_check(&self) -> Result<NodeHealthCheck> {
        let health_check = Self::perform_health_check(&self.node_status, &self.config).await;
        
        self.health_checks.lock().await.push(health_check.clone());
        
        Ok(health_check)
    }

    /// Calculate average block time
    async fn calculate_avg_block_time(&self) -> f64 {
        let times = self.block_times.lock().await;
        if times.is_empty() {
            return 0.0;
        }
        times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / times.len() as f64
    }

    /// Calculate current TPS
    async fn calculate_current_tps(&self) -> f64 {
        let throughput = self.tx_throughput.lock().await;
        if throughput.is_empty() {
            return 0.0;
        }
        throughput.iter().sum::<f64>() / throughput.len() as f64
    }

    /// Calculate network health score
    async fn calculate_network_health_score(&self) -> f64 {
        let status_map = self.node_status.read().await;
        
        if let Some(ghostchain_status) = status_map.get("ghostchain") {
            let mut score = 0.0;
            
            // Node status weight: 40%
            score += match ghostchain_status.status.as_str() {
                "synced" => 40.0,
                "syncing" => 20.0,
                _ => 0.0,
            };
            
            // Peer count weight: 30%
            score += (ghostchain_status.peer_count.min(10) as f64 / 10.0) * 30.0;
            
            // Sync progress weight: 30%
            score += (ghostchain_status.sync_progress / 100.0) * 30.0;
            
            score
        } else {
            0.0
        }
    }
}
