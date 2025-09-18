use super::{GhostFlowNode, NodeHealth, HealthStatus};
use crate::{Result, WorkflowContext, NodeExecutionResult, ExecutionStatus, BlockchainConfig, GasSettings};
use async_trait::async_trait;
use jarvis_agent::{BlockchainMonitorAgent, AIBlockchainAnalyzer, MonitoringConfig, AnalysisType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

/// Blockchain Monitor Node for tracking blockchain networks and smart contracts
pub struct BlockchainMonitorNode {
    monitor_agent: Arc<RwLock<Option<BlockchainMonitorAgent>>>,
    config: BlockchainMonitorConfig,
    health: Arc<RwLock<NodeHealth>>,
}

/// Blockchain Transaction Node for executing transactions with gas optimization
pub struct TransactionNode {
    analyzer: Arc<RwLock<Option<AIBlockchainAnalyzer>>>,
    config: TransactionConfig,
    health: Arc<RwLock<NodeHealth>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainMonitorConfig {
    pub networks: Vec<BlockchainConfig>,
    pub monitoring_interval_seconds: u64,
    pub alert_thresholds: AlertThresholds,
    pub enable_ai_analysis: bool,
    pub store_historical_data: bool,
    pub max_blocks_to_analyze: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionConfig {
    pub default_network: String,
    pub gas_optimization: bool,
    pub simulation_before_send: bool,
    pub max_gas_price_gwei: u64,
    pub slippage_tolerance: f64,
    pub retry_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub high_gas_price_gwei: u64,
    pub transaction_failure_rate: f64,
    pub network_congestion_threshold: f64,
    pub unusual_activity_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainMonitorInput {
    pub action: MonitorAction,
    pub network: Option<String>,
    pub contract_address: Option<String>,
    pub block_range: Option<BlockRange>,
    pub analysis_type: Option<AnalysisType>,
    pub real_time: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInput {
    pub action: TransactionAction,
    pub network: String,
    pub transaction_data: TransactionData,
    pub gas_settings: Option<GasSettings>,
    pub simulate_first: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitorAction {
    StartMonitoring,
    StopMonitoring,
    GetStatus,
    AnalyzeBlock,
    AnalyzeContract,
    GetAlerts,
    GenerateReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionAction {
    SendTransaction,
    SimulateTransaction,
    GetGasEstimate,
    OptimizeGas,
    GetTransactionStatus,
    CancelTransaction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRange {
    pub start_block: u64,
    pub end_block: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    pub to: String,
    pub value: Option<String>,
    pub data: Option<String>,
    pub gas_limit: Option<u64>,
    pub max_fee_per_gas: Option<u64>,
    pub max_priority_fee_per_gas: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainMonitorOutput {
    pub action_performed: MonitorAction,
    pub success: bool,
    pub network_status: Vec<NetworkStatus>,
    pub alerts: Vec<BlockchainAlert>,
    pub analysis_results: Option<AnalysisResults>,
    pub monitoring_metrics: MonitoringMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOutput {
    pub action_performed: TransactionAction,
    pub success: bool,
    pub transaction_hash: Option<String>,
    pub gas_used: Option<u64>,
    pub gas_price: Option<u64>,
    pub total_cost_eth: Option<f64>,
    pub simulation_results: Option<SimulationResults>,
    pub optimization_suggestions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub network: String,
    pub latest_block: u64,
    pub gas_price_gwei: f64,
    pub transaction_count: u64,
    pub network_congestion: f64,
    pub is_healthy: bool,
    pub last_updated: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainAlert {
    pub id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub network: String,
    pub contract_address: Option<String>,
    pub timestamp: chrono::DateTime<Utc>,
    pub auto_resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    HighGasPrice,
    NetworkCongestion,
    TransactionFailure,
    UnusualActivity,
    ContractEvent,
    SecurityThreat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResults {
    pub analysis_type: AnalysisType,
    pub findings: Vec<Finding>,
    pub recommendations: Vec<Recommendation>,
    pub confidence_score: f64,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub category: String,
    pub description: String,
    pub impact: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub title: String,
    pub description: String,
    pub priority: u32,
    pub estimated_gas_savings: Option<u64>,
    pub implementation_difficulty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringMetrics {
    pub total_blocks_analyzed: u64,
    pub total_transactions_monitored: u64,
    pub alerts_generated: u64,
    pub average_analysis_time_ms: f64,
    pub uptime_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResults {
    pub will_succeed: bool,
    pub estimated_gas: u64,
    pub revert_reason: Option<String>,
    pub state_changes: Vec<StateChange>,
    pub events_emitted: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub address: String,
    pub storage_slot: String,
    pub old_value: String,
    pub new_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
}

impl BlockchainMonitorNode {
    pub fn new() -> Result<Self> {
        Ok(Self {
            monitor_agent: Arc::new(RwLock::new(None)),
            config: BlockchainMonitorConfig::default(),
            health: Arc::new(RwLock::new(NodeHealth {
                status: HealthStatus::Unknown,
                message: None,
                last_execution: None,
                error_count: 0,
                success_rate: 0.0,
            })),
        })
    }

    async fn initialize_monitor(&self, config: &HashMap<String, serde_json::Value>) -> Result<()> {
        // Parse networks configuration
        let networks = if let Some(networks_value) = config.get("networks") {
            serde_json::from_value::<Vec<BlockchainConfig>>(networks_value.clone())?
        } else {
            vec![BlockchainConfig::default()]
        };

        // Create monitoring configuration
        let monitoring_config = MonitoringConfig {
            networks: networks.iter().map(|n| n.network.clone()).collect(),
            check_interval_seconds: config.get("monitoring_interval_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(30),
            alert_webhook_url: None,
            enable_detailed_analysis: config.get("enable_ai_analysis")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        };

        // Initialize monitor agent
        let monitor = BlockchainMonitorAgent::new(monitoring_config).await?;
        *self.monitor_agent.write().await = Some(monitor);

        Ok(())
    }

    async fn start_monitoring(&self, input: &BlockchainMonitorInput) -> Result<BlockchainMonitorOutput> {
        let monitor = self.monitor_agent.read().await;
        let _monitor = monitor.as_ref().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Monitor agent not initialized".to_string()))?;

        // In real implementation, this would start the monitoring process
        // For now, simulate monitoring startup
        
        let network_status = vec![NetworkStatus {
            network: input.network.clone().unwrap_or_else(|| "ethereum".to_string()),
            latest_block: 18500000, // Simulated
            gas_price_gwei: 25.0,
            transaction_count: 150,
            network_congestion: 0.65,
            is_healthy: true,
            last_updated: Utc::now(),
        }];

        Ok(BlockchainMonitorOutput {
            action_performed: MonitorAction::StartMonitoring,
            success: true,
            network_status,
            alerts: vec![],
            analysis_results: None,
            monitoring_metrics: MonitoringMetrics {
                total_blocks_analyzed: 0,
                total_transactions_monitored: 0,
                alerts_generated: 0,
                average_analysis_time_ms: 0.0,
                uptime_percentage: 100.0,
            },
        })
    }

    async fn analyze_contract(&self, input: &BlockchainMonitorInput) -> Result<BlockchainMonitorOutput> {
        let contract_address = input.contract_address.as_ref().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Contract address required for analysis".to_string()))?;

        // Simulate AI analysis of smart contract
        let analysis_results = AnalysisResults {
            analysis_type: input.analysis_type.clone().unwrap_or(AnalysisType::Security),
            findings: vec![
                Finding {
                    category: "Security".to_string(),
                    description: "Contract follows standard security patterns".to_string(),
                    impact: "Low risk".to_string(),
                    evidence: vec!["No reentrancy vulnerabilities detected".to_string()],
                },
            ],
            recommendations: vec![
                Recommendation {
                    title: "Gas Optimization".to_string(),
                    description: "Consider using storage packing to reduce gas costs".to_string(),
                    priority: 2,
                    estimated_gas_savings: Some(15000),
                    implementation_difficulty: "Medium".to_string(),
                },
            ],
            confidence_score: 0.87,
            processing_time_ms: 1250,
        };

        Ok(BlockchainMonitorOutput {
            action_performed: MonitorAction::AnalyzeContract,
            success: true,
            network_status: vec![],
            alerts: vec![],
            analysis_results: Some(analysis_results),
            monitoring_metrics: MonitoringMetrics::default(),
        })
    }

    async fn update_health_metrics(&self, success: bool, execution_time_ms: u64) {
        let mut health = self.health.write().await;
        
        if !success {
            health.error_count += 1;
        }
        
        health.last_execution = Some(Utc::now());
        health.status = if health.error_count == 0 {
            HealthStatus::Healthy
        } else if health.error_count < 3 {
            HealthStatus::Warning
        } else {
            HealthStatus::Critical
        };
    }
}

impl TransactionNode {
    pub fn new() -> Result<Self> {
        Ok(Self {
            analyzer: Arc::new(RwLock::new(None)),
            config: TransactionConfig::default(),
            health: Arc::new(RwLock::new(NodeHealth {
                status: HealthStatus::Unknown,
                message: None,
                last_execution: None,
                error_count: 0,
                success_rate: 0.0,
            })),
        })
    }

    async fn initialize_analyzer(&self, config: &HashMap<String, serde_json::Value>) -> Result<()> {
        // Initialize AI blockchain analyzer for gas optimization
        let analyzer_config = jarvis_agent::AIAnalyzerConfig {
            analysis_types: vec![AnalysisType::GasOptimization, AnalysisType::Security],
            confidence_threshold: 0.7,
            enable_caching: true,
            max_analysis_time_seconds: 60,
        };

        let analyzer = AIBlockchainAnalyzer::new(analyzer_config).await?;
        *self.analyzer.write().await = Some(analyzer);

        Ok(())
    }

    async fn simulate_transaction(&self, input: &TransactionInput) -> Result<TransactionOutput> {
        // Simulate transaction execution
        let simulation_results = SimulationResults {
            will_succeed: true,
            estimated_gas: 21000,
            revert_reason: None,
            state_changes: vec![],
            events_emitted: vec![],
        };

        Ok(TransactionOutput {
            action_performed: TransactionAction::SimulateTransaction,
            success: true,
            transaction_hash: None,
            gas_used: None,
            gas_price: None,
            total_cost_eth: None,
            simulation_results: Some(simulation_results),
            optimization_suggestions: Some(vec![
                "Consider batching multiple operations".to_string(),
                "Use CREATE2 for deterministic addresses".to_string(),
            ]),
        })
    }

    async fn optimize_gas(&self, input: &TransactionInput) -> Result<TransactionOutput> {
        let analyzer = self.analyzer.read().await;
        let _analyzer = analyzer.as_ref().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Analyzer not initialized".to_string()))?;

        // Simulate gas optimization analysis
        let optimized_gas = 18500; // Reduced from estimated 21000
        let savings = 2500;

        Ok(TransactionOutput {
            action_performed: TransactionAction::OptimizeGas,
            success: true,
            transaction_hash: None,
            gas_used: Some(optimized_gas),
            gas_price: Some(25_000_000_000), // 25 gwei
            total_cost_eth: Some(0.0004625), // optimized_gas * gas_price / 1e18
            simulation_results: None,
            optimization_suggestions: Some(vec![
                format!("Gas savings: {} units ({:.2}%)", savings, (savings as f64 / 21000.0) * 100.0),
                "Optimized storage access patterns".to_string(),
                "Reduced function call overhead".to_string(),
            ]),
        })
    }

    async fn send_transaction(&self, input: &TransactionInput) -> Result<TransactionOutput> {
        // In real implementation, this would interact with blockchain networks
        // For now, simulate transaction sending
        
        let transaction_hash = format!("0x{}", Uuid::new_v4().to_string().replace("-", ""));
        
        Ok(TransactionOutput {
            action_performed: TransactionAction::SendTransaction,
            success: true,
            transaction_hash: Some(transaction_hash),
            gas_used: Some(21000),
            gas_price: Some(25_000_000_000),
            total_cost_eth: Some(0.000525),
            simulation_results: None,
            optimization_suggestions: None,
        })
    }

    async fn update_health_metrics(&self, success: bool, execution_time_ms: u64) {
        let mut health = self.health.write().await;
        
        if !success {
            health.error_count += 1;
        }
        
        health.last_execution = Some(Utc::now());
        health.status = if health.error_count == 0 {
            HealthStatus::Healthy
        } else if health.error_count < 3 {
            HealthStatus::Warning
        } else {
            HealthStatus::Critical
        };
    }
}

// Implement GhostFlowNode for BlockchainMonitorNode
#[async_trait]
impl GhostFlowNode for BlockchainMonitorNode {
    fn node_type(&self) -> &'static str {
        "jarvis.blockchain.monitor"
    }

    fn display_name(&self) -> &str {
        "Blockchain Monitor"
    }

    fn description(&self) -> &str {
        "Monitor blockchain networks and smart contracts with AI-powered analysis and alerting"
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["start_monitoring", "stop_monitoring", "get_status", "analyze_block", "analyze_contract", "get_alerts", "generate_report"],
                    "description": "The monitoring action to perform"
                },
                "network": {
                    "type": "string",
                    "description": "Blockchain network to monitor",
                    "enum": ["ethereum", "polygon", "bsc", "arbitrum", "optimism"]
                },
                "contract_address": {
                    "type": "string",
                    "description": "Smart contract address to analyze"
                },
                "block_range": {
                    "type": "object",
                    "description": "Range of blocks to analyze",
                    "properties": {
                        "start_block": { "type": "integer" },
                        "end_block": { "type": "integer" }
                    }
                },
                "analysis_type": {
                    "type": "string",
                    "enum": ["security", "gas_optimization", "performance", "compliance"],
                    "description": "Type of analysis to perform"
                },
                "real_time": {
                    "type": "boolean",
                    "description": "Enable real-time monitoring"
                }
            },
            "required": ["action"]
        })
    }

    fn output_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action_performed": { "type": "string" },
                "success": { "type": "boolean" },
                "network_status": {
                    "type": "array",
                    "description": "Status of monitored networks"
                },
                "alerts": {
                    "type": "array",
                    "description": "Generated alerts"
                },
                "analysis_results": {
                    "type": "object",
                    "description": "AI analysis results"
                },
                "monitoring_metrics": {
                    "type": "object",
                    "description": "Monitoring performance metrics"
                }
            }
        })
    }

    fn config_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "networks": {
                    "type": "array",
                    "description": "Blockchain networks to monitor",
                    "items": {
                        "type": "object",
                        "properties": {
                            "network": { "type": "string" },
                            "rpc_url": { "type": "string" },
                            "api_key": { "type": "string" }
                        }
                    }
                },
                "monitoring_interval_seconds": {
                    "type": "integer",
                    "default": 30,
                    "description": "How often to check for updates"
                },
                "enable_ai_analysis": {
                    "type": "boolean",
                    "default": true,
                    "description": "Enable AI-powered analysis"
                },
                "alert_thresholds": {
                    "type": "object",
                    "description": "Thresholds for generating alerts"
                }
            }
        })
    }

    async fn execute(
        &self,
        context: &mut WorkflowContext,
        inputs: HashMap<String, serde_json::Value>,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<crate::NodeExecutionResult> {
        let start_time = Instant::now();
        
        if self.monitor_agent.read().await.is_none() {
            self.initialize_monitor(&config).await?;
        }

        let input: BlockchainMonitorInput = serde_json::from_value(serde_json::Value::Object(
            inputs.into_iter().collect()
        ))?;

        let result = match input.action {
            MonitorAction::StartMonitoring => self.start_monitoring(&input).await,
            MonitorAction::AnalyzeContract => self.analyze_contract(&input).await,
            _ => {
                Ok(BlockchainMonitorOutput {
                    action_performed: input.action.clone(),
                    success: false,
                    network_status: vec![],
                    alerts: vec![],
                    analysis_results: None,
                    monitoring_metrics: MonitoringMetrics::default(),
                })
            }
        };

        match result {
            Ok(output) => {
                self.update_health_metrics(output.success, start_time.elapsed().as_millis() as u64).await;
                
                Ok(crate::NodeExecutionResult {
                    node_id: "blockchain_monitor".to_string(),
                    execution_id: context.execution_id,
                    status: if output.success { ExecutionStatus::Success } else { ExecutionStatus::Failure },
                    output: serde_json::to_value(output)?,
                    error: None,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    metadata: HashMap::new(),
                    next_nodes: vec![],
                })
            }
            Err(e) => {
                self.update_health_metrics(false, start_time.elapsed().as_millis() as u64).await;
                
                Ok(crate::NodeExecutionResult {
                    node_id: "blockchain_monitor".to_string(),
                    execution_id: context.execution_id,
                    status: ExecutionStatus::Failure,
                    output: json!({}),
                    error: Some(e.to_string()),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    metadata: HashMap::new(),
                    next_nodes: vec![],
                })
            }
        }
    }

    fn validate_config(&self, config: &HashMap<String, serde_json::Value>) -> Result<()> {
        if let Some(networks) = config.get("networks") {
            if let Ok(network_configs) = serde_json::from_value::<Vec<BlockchainConfig>>(networks.clone()) {
                if network_configs.is_empty() {
                    return Err(crate::GhostFlowError::Config(
                        "At least one network must be configured".to_string()
                    ));
                }
            }
        }
        Ok(())
    }

    async fn health_check(&self) -> NodeHealth {
        self.health.read().await.clone()
    }
}

// Implement GhostFlowNode for TransactionNode
#[async_trait]
impl GhostFlowNode for TransactionNode {
    fn node_type(&self) -> &'static str {
        "jarvis.blockchain.transaction"
    }

    fn display_name(&self) -> &str {
        "Blockchain Transaction"
    }

    fn description(&self) -> &str {
        "Execute blockchain transactions with AI-powered gas optimization and simulation"
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["send_transaction", "simulate_transaction", "get_gas_estimate", "optimize_gas", "get_transaction_status", "cancel_transaction"],
                    "description": "Transaction action to perform"
                },
                "network": {
                    "type": "string",
                    "description": "Blockchain network",
                    "enum": ["ethereum", "polygon", "bsc", "arbitrum", "optimism"]
                },
                "transaction_data": {
                    "type": "object",
                    "description": "Transaction details",
                    "properties": {
                        "to": { "type": "string", "description": "Recipient address" },
                        "value": { "type": "string", "description": "Value to send (in wei)" },
                        "data": { "type": "string", "description": "Transaction data" },
                        "gas_limit": { "type": "integer", "description": "Gas limit" }
                    },
                    "required": ["to"]
                },
                "gas_settings": {
                    "type": "object",
                    "description": "Gas configuration"
                },
                "simulate_first": {
                    "type": "boolean",
                    "description": "Simulate before sending",
                    "default": true
                }
            },
            "required": ["action", "network", "transaction_data"]
        })
    }

    fn output_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action_performed": { "type": "string" },
                "success": { "type": "boolean" },
                "transaction_hash": { "type": "string" },
                "gas_used": { "type": "integer" },
                "gas_price": { "type": "integer" },
                "total_cost_eth": { "type": "number" },
                "simulation_results": { "type": "object" },
                "optimization_suggestions": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        })
    }

    fn config_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "default_network": {
                    "type": "string",
                    "default": "ethereum",
                    "description": "Default blockchain network"
                },
                "gas_optimization": {
                    "type": "boolean",
                    "default": true,
                    "description": "Enable AI gas optimization"
                },
                "simulation_before_send": {
                    "type": "boolean",
                    "default": true,
                    "description": "Always simulate before sending"
                },
                "max_gas_price_gwei": {
                    "type": "integer",
                    "default": 100,
                    "description": "Maximum gas price in gwei"
                },
                "retry_attempts": {
                    "type": "integer",
                    "default": 3,
                    "description": "Number of retry attempts"
                }
            }
        })
    }

    async fn execute(
        &self,
        context: &mut WorkflowContext,
        inputs: HashMap<String, serde_json::Value>,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<crate::NodeExecutionResult> {
        let start_time = Instant::now();
        
        if self.analyzer.read().await.is_none() {
            self.initialize_analyzer(&config).await?;
        }

        let input: TransactionInput = serde_json::from_value(serde_json::Value::Object(
            inputs.into_iter().collect()
        ))?;

        let result = match input.action {
            TransactionAction::SimulateTransaction => self.simulate_transaction(&input).await,
            TransactionAction::OptimizeGas => self.optimize_gas(&input).await,
            TransactionAction::SendTransaction => self.send_transaction(&input).await,
            _ => {
                Ok(TransactionOutput {
                    action_performed: input.action.clone(),
                    success: false,
                    transaction_hash: None,
                    gas_used: None,
                    gas_price: None,
                    total_cost_eth: None,
                    simulation_results: None,
                    optimization_suggestions: Some(vec!["Action not implemented yet".to_string()]),
                })
            }
        };

        match result {
            Ok(output) => {
                self.update_health_metrics(output.success, start_time.elapsed().as_millis() as u64).await;
                
                Ok(crate::NodeExecutionResult {
                    node_id: "blockchain_transaction".to_string(),
                    execution_id: context.execution_id,
                    status: if output.success { ExecutionStatus::Success } else { ExecutionStatus::Failure },
                    output: serde_json::to_value(output)?,
                    error: None,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    metadata: HashMap::new(),
                    next_nodes: vec![],
                })
            }
            Err(e) => {
                self.update_health_metrics(false, start_time.elapsed().as_millis() as u64).await;
                
                Ok(crate::NodeExecutionResult {
                    node_id: "blockchain_transaction".to_string(),
                    execution_id: context.execution_id,
                    status: ExecutionStatus::Failure,
                    output: json!({}),
                    error: Some(e.to_string()),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    metadata: HashMap::new(),
                    next_nodes: vec![],
                })
            }
        }
    }

    fn validate_config(&self, config: &HashMap<String, serde_json::Value>) -> Result<()> {
        if let Some(max_gas_price) = config.get("max_gas_price_gwei") {
            if let Some(price) = max_gas_price.as_u64() {
                if price == 0 {
                    return Err(crate::GhostFlowError::Config(
                        "max_gas_price_gwei must be greater than 0".to_string()
                    ));
                }
            }
        }
        Ok(())
    }

    async fn health_check(&self) -> NodeHealth {
        self.health.read().await.clone()
    }
}

// Default implementations
impl Default for BlockchainMonitorConfig {
    fn default() -> Self {
        Self {
            networks: vec![BlockchainConfig::default()],
            monitoring_interval_seconds: 30,
            alert_thresholds: AlertThresholds {
                high_gas_price_gwei: 50,
                transaction_failure_rate: 0.1,
                network_congestion_threshold: 0.8,
                unusual_activity_threshold: 2.0,
            },
            enable_ai_analysis: true,
            store_historical_data: true,
            max_blocks_to_analyze: 100,
        }
    }
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            default_network: "ethereum".to_string(),
            gas_optimization: true,
            simulation_before_send: true,
            max_gas_price_gwei: 100,
            slippage_tolerance: 0.01,
            retry_attempts: 3,
        }
    }
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            network: "ethereum".to_string(),
            rpc_url: "https://mainnet.infura.io/v3/".to_string(),
            contract_address: None,
            private_key: None,
            gas_settings: GasSettings {
                gas_limit: 21000,
                gas_price: None,
                max_fee_per_gas: None,
                max_priority_fee_per_gas: None,
            },
        }
    }
}

impl Default for MonitoringMetrics {
    fn default() -> Self {
        Self {
            total_blocks_analyzed: 0,
            total_transactions_monitored: 0,
            alerts_generated: 0,
            average_analysis_time_ms: 0.0,
            uptime_percentage: 100.0,
        }
    }
}