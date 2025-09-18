use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Core workflow execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    pub workflow_id: Uuid,
    pub execution_id: Uuid,
    pub current_node: String,
    pub variables: HashMap<String, serde_json::Value>,
    pub memory_context: Option<MemoryContext>,
    pub agent_states: HashMap<String, AgentState>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Memory context for cross-workflow persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContext {
    pub session_id: Uuid,
    pub context_entries: Vec<ContextEntry>,
    pub semantic_tags: Vec<String>,
    pub relevance_score: f64,
}

/// Individual context entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: Uuid,
    pub content: String,
    pub entry_type: ContextEntryType,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextEntryType {
    UserInput,
    AIResponse,
    SystemEvent,
    WorkflowResult,
    NodeOutput,
}

/// Agent state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub agent_id: String,
    pub agent_type: AgentType,
    pub status: AgentStatus,
    pub current_task: Option<String>,
    pub progress: f64,
    pub error_message: Option<String>,
    pub metrics: AgentMetrics,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    LLMRouter,
    MemoryManager,
    BlockchainMonitor,
    NetworkOptimizer,
    TaskOrchestrator,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Running,
    Completed,
    Failed,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub execution_count: u64,
    pub success_rate: f64,
    pub average_duration_ms: f64,
    pub last_execution: Option<DateTime<Utc>>,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time_ms: u64,
    pub memory_mb: u64,
    pub network_requests: u64,
    pub tokens_consumed: u64,
}

/// Node execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionResult {
    pub node_id: String,
    pub execution_id: Uuid,
    pub status: ExecutionStatus,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub metadata: HashMap<String, serde_json::Value>,
    pub next_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Success,
    Failure,
    Partial,
    Skipped,
}

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub version: String,
    pub nodes: Vec<NodeDefinition>,
    pub connections: Vec<NodeConnection>,
    pub triggers: Vec<WorkflowTrigger>,
    pub settings: WorkflowSettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDefinition {
    pub id: String,
    pub node_type: String,
    pub name: String,
    pub description: String,
    pub config: HashMap<String, serde_json::Value>,
    pub position: NodePosition,
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConnection {
    pub id: String,
    pub source_node: String,
    pub source_output: String,
    pub target_node: String,
    pub target_input: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTrigger {
    pub id: String,
    pub trigger_type: TriggerType,
    pub config: HashMap<String, serde_json::Value>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    Manual,
    Scheduled,
    Webhook,
    Event,
    MemoryThreshold,
    AgentCompletion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSettings {
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub memory_persistence: bool,
    pub agent_coordination: bool,
    pub real_time_updates: bool,
    pub cost_optimization: bool,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub context_window: usize,
    pub cost_per_token: f64,
    pub priority: u32,
}

/// Blockchain integration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    pub network: String,
    pub rpc_url: String,
    pub contract_address: Option<String>,
    pub private_key: Option<String>,
    pub gas_settings: GasSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasSettings {
    pub gas_limit: u64,
    pub gas_price: Option<u64>,
    pub max_fee_per_gas: Option<u64>,
    pub max_priority_fee_per_gas: Option<u64>,
}

/// Network optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkOptimizationConfig {
    pub enable_quic: bool,
    pub enable_http3: bool,
    pub ipv6_optimization: bool,
    pub connection_pooling: bool,
    pub compression: bool,
    pub timeout_ms: u64,
    pub retry_attempts: u32,
}

impl Default for WorkflowContext {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            workflow_id: Uuid::new_v4(),
            execution_id: Uuid::new_v4(),
            current_node: String::new(),
            variables: HashMap::new(),
            memory_context: None,
            agent_states: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl Default for AgentMetrics {
    fn default() -> Self {
        Self {
            execution_count: 0,
            success_rate: 0.0,
            average_duration_ms: 0.0,
            last_execution: None,
            resource_usage: ResourceUsage {
                cpu_time_ms: 0,
                memory_mb: 0,
                network_requests: 0,
                tokens_consumed: 0,
            },
        }
    }
}

impl Default for WorkflowSettings {
    fn default() -> Self {
        Self {
            timeout_seconds: 3600, // 1 hour
            retry_attempts: 3,
            memory_persistence: true,
            agent_coordination: true,
            real_time_updates: false,
            cost_optimization: true,
        }
    }
}