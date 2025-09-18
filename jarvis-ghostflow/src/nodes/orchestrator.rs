use super::{GhostFlowNode, NodeHealth, HealthStatus};
use crate::{Result, WorkflowContext, NodeExecutionResult, ExecutionStatus, AgentState, AgentType, AgentStatus, AgentMetrics};
use async_trait::async_trait;
use jarvis_agent::{BlockchainAgentOrchestrator, AgentStatus as JarvisAgentStatus, AgentMessage};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use chrono::Utc;

/// Agent Orchestrator Node for coordinating multiple AI agents
pub struct OrchestratorNode {
    orchestrator: Arc<RwLock<Option<MultiAgentOrchestrator>>>,
    config: OrchestratorConfig,
    health: Arc<RwLock<NodeHealth>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub max_concurrent_agents: usize,
    pub agent_timeout_seconds: u64,
    pub health_check_interval_seconds: u64,
    pub auto_recovery: bool,
    pub load_balancing: bool,
    pub priority_scheduling: bool,
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_cores: u32,
    pub max_network_connections: u32,
    pub max_tokens_per_minute: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorInput {
    pub action: OrchestratorAction,
    pub agent_configs: Option<Vec<AgentConfig>>,
    pub task_definition: Option<TaskDefinition>,
    pub agent_id: Option<String>,
    pub coordination_strategy: Option<CoordinationStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestratorAction {
    SpawnAgents,
    KillAgent,
    GetStatus,
    ExecuteTask,
    Coordinate,
    HealthCheck,
    Rebalance,
    GetMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_type: AgentType,
    pub config: HashMap<String, serde_json::Value>,
    pub priority: u32,
    pub max_execution_time: Option<Duration>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub task_id: String,
    pub task_type: TaskType,
    pub input_data: serde_json::Value,
    pub dependencies: Vec<String>,
    pub timeout: Option<Duration>,
    pub parallel: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    LLMGeneration,
    BlockchainAnalysis,
    DataProcessing,
    NetworkOptimization,
    MemoryManagement,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationStrategy {
    Sequential,
    Parallel,
    Pipeline,
    Adaptive,
    LoadBalanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorOutput {
    pub action_performed: OrchestratorAction,
    pub success: bool,
    pub agent_states: Vec<AgentState>,
    pub task_results: HashMap<String, serde_json::Value>,
    pub coordination_metrics: CoordinationMetrics,
    pub resource_usage: crate::ResourceUsage,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationMetrics {
    pub total_agents: usize,
    pub active_agents: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub average_task_duration_ms: f64,
    pub throughput_tasks_per_minute: f64,
    pub resource_efficiency: f64,
}

/// Multi-agent orchestration system
pub struct MultiAgentOrchestrator {
    agents: HashMap<String, ManagedAgent>,
    task_queue: Vec<TaskDefinition>,
    message_bus: mpsc::UnboundedSender<OrchestratorMessage>,
    message_receiver: Arc<RwLock<mpsc::UnboundedReceiver<OrchestratorMessage>>>,
    blockchain_orchestrator: Option<BlockchainAgentOrchestrator>,
    coordination_strategy: CoordinationStrategy,
    metrics: CoordinationMetrics,
}

#[derive(Debug)]
pub struct ManagedAgent {
    pub id: String,
    pub agent_type: AgentType,
    pub state: AgentState,
    pub handle: Option<tokio::task::JoinHandle<()>>,
    pub message_sender: Option<mpsc::UnboundedSender<AgentMessage>>,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum OrchestratorMessage {
    AgentStarted(String),
    AgentCompleted(String, serde_json::Value),
    AgentFailed(String, String),
    TaskCompleted(String),
    HealthUpdate(String, NodeHealth),
    ResourceAlert(String, String),
}

impl OrchestratorNode {
    pub fn new() -> Result<Self> {
        Ok(Self {
            orchestrator: Arc::new(RwLock::new(None)),
            config: OrchestratorConfig::default(),
            health: Arc::new(RwLock::new(NodeHealth {
                status: HealthStatus::Unknown,
                message: None,
                last_execution: None,
                error_count: 0,
                success_rate: 0.0,
            })),
        })
    }

    async fn initialize_orchestrator(&self, config: &HashMap<String, serde_json::Value>) -> Result<()> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let orchestrator = MultiAgentOrchestrator {
            agents: HashMap::new(),
            task_queue: Vec::new(),
            message_bus: tx,
            message_receiver: Arc::new(RwLock::new(rx)),
            blockchain_orchestrator: None,
            coordination_strategy: CoordinationStrategy::Adaptive,
            metrics: CoordinationMetrics::default(),
        };

        *self.orchestrator.write().await = Some(orchestrator);
        Ok(())
    }

    async fn spawn_agents(&self, agent_configs: &[AgentConfig]) -> Result<OrchestratorOutput> {
        let mut orchestrator = self.orchestrator.write().await;
        let orchestrator = orchestrator.as_mut().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Orchestrator not initialized".to_string()))?;

        let mut spawned_agents = Vec::new();
        let mut errors = Vec::new();

        for agent_config in agent_configs {
            match self.spawn_single_agent(orchestrator, agent_config).await {
                Ok(agent_state) => spawned_agents.push(agent_state),
                Err(e) => errors.push(e.to_string()),
            }
        }

        Ok(OrchestratorOutput {
            action_performed: OrchestratorAction::SpawnAgents,
            success: errors.is_empty(),
            agent_states: spawned_agents,
            task_results: HashMap::new(),
            coordination_metrics: orchestrator.metrics.clone(),
            resource_usage: self.calculate_resource_usage(orchestrator).await,
            errors,
        })
    }

    async fn spawn_single_agent(
        &self,
        orchestrator: &mut MultiAgentOrchestrator,
        config: &AgentConfig,
    ) -> Result<AgentState> {
        let agent_id = Uuid::new_v4().to_string();
        let (agent_tx, agent_rx) = mpsc::unbounded_channel();

        let agent_state = AgentState {
            agent_id: agent_id.clone(),
            agent_type: config.agent_type.clone(),
            status: AgentStatus::Idle,
            current_task: None,
            progress: 0.0,
            error_message: None,
            metrics: AgentMetrics::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Create managed agent entry
        let managed_agent = ManagedAgent {
            id: agent_id.clone(),
            agent_type: config.agent_type.clone(),
            state: agent_state.clone(),
            handle: None, // Would spawn actual agent task here
            message_sender: Some(agent_tx),
            created_at: Utc::now(),
        };

        orchestrator.agents.insert(agent_id.clone(), managed_agent);
        orchestrator.metrics.total_agents += 1;
        orchestrator.metrics.active_agents += 1;

        tracing::info!("Spawned agent {} of type {:?}", agent_id, config.agent_type);

        Ok(agent_state)
    }

    async fn execute_coordinated_task(&self, task: &TaskDefinition) -> Result<OrchestratorOutput> {
        let mut orchestrator = self.orchestrator.write().await;
        let orchestrator = orchestrator.as_mut().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Orchestrator not initialized".to_string()))?;

        let start_time = Instant::now();
        let mut task_results = HashMap::new();
        let mut errors = Vec::new();

        // Select appropriate agents for the task
        let suitable_agents = self.select_agents_for_task(orchestrator, task).await?;

        if suitable_agents.is_empty() {
            errors.push("No suitable agents available for task".to_string());
            return Ok(OrchestratorOutput {
                action_performed: OrchestratorAction::ExecuteTask,
                success: false,
                agent_states: vec![],
                task_results,
                coordination_metrics: orchestrator.metrics.clone(),
                resource_usage: self.calculate_resource_usage(orchestrator).await,
                errors,
            });
        }

        // Execute task based on coordination strategy
        match orchestrator.coordination_strategy {
            CoordinationStrategy::Sequential => {
                for agent_id in suitable_agents {
                    match self.execute_agent_task(orchestrator, &agent_id, task).await {
                        Ok(result) => {
                            task_results.insert(agent_id.clone(), result);
                        }
                        Err(e) => {
                            errors.push(format!("Agent {} failed: {}", agent_id, e));
                        }
                    }
                }
            }
            CoordinationStrategy::Parallel => {
                // Execute all agents in parallel (simplified implementation)
                for agent_id in suitable_agents {
                    match self.execute_agent_task(orchestrator, &agent_id, task).await {
                        Ok(result) => {
                            task_results.insert(agent_id.clone(), result);
                        }
                        Err(e) => {
                            errors.push(format!("Agent {} failed: {}", agent_id, e));
                        }
                    }
                }
            }
            _ => {
                // Default to sequential for other strategies
                for agent_id in suitable_agents {
                    match self.execute_agent_task(orchestrator, &agent_id, task).await {
                        Ok(result) => {
                            task_results.insert(agent_id.clone(), result);
                        }
                        Err(e) => {
                            errors.push(format!("Agent {} failed: {}", agent_id, e));
                        }
                    }
                }
            }
        }

        // Update metrics
        let execution_time = start_time.elapsed().as_millis() as f64;
        if errors.is_empty() {
            orchestrator.metrics.completed_tasks += 1;
        } else {
            orchestrator.metrics.failed_tasks += 1;
        }
        
        orchestrator.metrics.average_task_duration_ms = 
            (orchestrator.metrics.average_task_duration_ms + execution_time) / 2.0;

        Ok(OrchestratorOutput {
            action_performed: OrchestratorAction::ExecuteTask,
            success: errors.is_empty(),
            agent_states: orchestrator.agents.values().map(|a| a.state.clone()).collect(),
            task_results,
            coordination_metrics: orchestrator.metrics.clone(),
            resource_usage: self.calculate_resource_usage(orchestrator).await,
            errors,
        })
    }

    async fn select_agents_for_task(
        &self,
        orchestrator: &MultiAgentOrchestrator,
        task: &TaskDefinition,
    ) -> Result<Vec<String>> {
        let mut suitable_agents = Vec::new();

        for (agent_id, agent) in &orchestrator.agents {
            if agent.state.status == AgentStatus::Idle {
                let is_suitable = match (&task.task_type, &agent.agent_type) {
                    (TaskType::LLMGeneration, AgentType::LLMRouter) => true,
                    (TaskType::BlockchainAnalysis, AgentType::BlockchainMonitor) => true,
                    (TaskType::MemoryManagement, AgentType::MemoryManager) => true,
                    (TaskType::NetworkOptimization, AgentType::NetworkOptimizer) => true,
                    (TaskType::Custom(_), _) => true, // Custom tasks can use any agent
                    _ => false,
                };

                if is_suitable {
                    suitable_agents.push(agent_id.clone());
                }
            }
        }

        Ok(suitable_agents)
    }

    async fn execute_agent_task(
        &self,
        orchestrator: &mut MultiAgentOrchestrator,
        agent_id: &str,
        task: &TaskDefinition,
    ) -> Result<serde_json::Value> {
        // Update agent status
        if let Some(agent) = orchestrator.agents.get_mut(agent_id) {
            agent.state.status = AgentStatus::Running;
            agent.state.current_task = Some(task.task_id.clone());
            agent.state.updated_at = Utc::now();
        }

        // Simulate task execution (in real implementation, this would communicate with actual agents)
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Update agent status to completed
        if let Some(agent) = orchestrator.agents.get_mut(agent_id) {
            agent.state.status = AgentStatus::Completed;
            agent.state.progress = 100.0;
            agent.state.current_task = None;
            agent.state.updated_at = Utc::now();
            agent.state.metrics.execution_count += 1;
        }

        Ok(json!({
            "task_id": task.task_id,
            "agent_id": agent_id,
            "result": "Task completed successfully",
            "execution_time_ms": 100
        }))
    }

    async fn calculate_resource_usage(&self, orchestrator: &MultiAgentOrchestrator) -> crate::ResourceUsage {
        crate::ResourceUsage {
            cpu_time_ms: orchestrator.agents.len() as u64 * 10, // Simplified calculation
            memory_mb: orchestrator.agents.len() as u64 * 50,
            network_requests: orchestrator.metrics.completed_tasks as u64,
            tokens_consumed: orchestrator.metrics.completed_tasks as u64 * 100,
        }
    }

    async fn get_orchestrator_status(&self) -> Result<OrchestratorOutput> {
        let orchestrator = self.orchestrator.read().await;
        let orchestrator = orchestrator.as_ref().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Orchestrator not initialized".to_string()))?;

        let agent_states: Vec<AgentState> = orchestrator.agents.values()
            .map(|agent| agent.state.clone())
            .collect();

        Ok(OrchestratorOutput {
            action_performed: OrchestratorAction::GetStatus,
            success: true,
            agent_states,
            task_results: HashMap::new(),
            coordination_metrics: orchestrator.metrics.clone(),
            resource_usage: self.calculate_resource_usage(orchestrator).await,
            errors: vec![],
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

#[async_trait]
impl GhostFlowNode for OrchestratorNode {
    fn node_type(&self) -> &'static str {
        "jarvis.orchestrator"
    }

    fn display_name(&self) -> &str {
        "Agent Orchestrator"
    }

    fn description(&self) -> &str {
        "Coordinate multiple AI agents with health monitoring, load balancing, and adaptive scheduling"
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["spawn_agents", "kill_agent", "get_status", "execute_task", "coordinate", "health_check", "rebalance", "get_metrics"],
                    "description": "The orchestration action to perform"
                },
                "agent_configs": {
                    "type": "array",
                    "description": "Configuration for agents to spawn",
                    "items": {
                        "type": "object",
                        "properties": {
                            "agent_type": {
                                "type": "string",
                                "enum": ["llm_router", "memory_manager", "blockchain_monitor", "network_optimizer", "task_orchestrator"]
                            },
                            "priority": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 10
                            },
                            "config": {
                                "type": "object",
                                "description": "Agent-specific configuration"
                            }
                        }
                    }
                },
                "task_definition": {
                    "type": "object",
                    "description": "Task to execute with agents",
                    "properties": {
                        "task_id": { "type": "string" },
                        "task_type": {
                            "type": "string",
                            "enum": ["llm_generation", "blockchain_analysis", "data_processing", "network_optimization", "memory_management"]
                        },
                        "input_data": { "type": "object" },
                        "parallel": { "type": "boolean" }
                    }
                },
                "coordination_strategy": {
                    "type": "string",
                    "enum": ["sequential", "parallel", "pipeline", "adaptive", "load_balanced"],
                    "description": "How to coordinate agent execution"
                }
            },
            "required": ["action"]
        })
    }

    fn output_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action_performed": {
                    "type": "string",
                    "description": "The action that was performed"
                },
                "success": {
                    "type": "boolean",
                    "description": "Whether the action was successful"
                },
                "agent_states": {
                    "type": "array",
                    "description": "Current state of all managed agents"
                },
                "task_results": {
                    "type": "object",
                    "description": "Results from executed tasks"
                },
                "coordination_metrics": {
                    "type": "object",
                    "description": "Metrics about coordination performance"
                },
                "resource_usage": {
                    "type": "object",
                    "description": "Current resource usage"
                }
            }
        })
    }

    fn config_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "max_concurrent_agents": {
                    "type": "integer",
                    "description": "Maximum number of concurrent agents",
                    "default": 10,
                    "minimum": 1,
                    "maximum": 100
                },
                "agent_timeout_seconds": {
                    "type": "integer",
                    "description": "Timeout for agent operations",
                    "default": 300
                },
                "auto_recovery": {
                    "type": "boolean",
                    "description": "Enable automatic agent recovery",
                    "default": true
                },
                "load_balancing": {
                    "type": "boolean",
                    "description": "Enable load balancing across agents",
                    "default": true
                },
                "resource_limits": {
                    "type": "object",
                    "description": "Resource limits for orchestration",
                    "properties": {
                        "max_memory_mb": { "type": "integer", "default": 2048 },
                        "max_cpu_cores": { "type": "integer", "default": 4 },
                        "max_tokens_per_minute": { "type": "integer", "default": 10000 }
                    }
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
        
        // Initialize orchestrator if needed
        if self.orchestrator.read().await.is_none() {
            self.initialize_orchestrator(&config).await?;
        }

        // Parse input
        let input: OrchestratorInput = serde_json::from_value(serde_json::Value::Object(
            inputs.into_iter().collect()
        ))?;

        // Execute the requested orchestration action
        let result = match input.action {
            OrchestratorAction::SpawnAgents => {
                if let Some(agent_configs) = &input.agent_configs {
                    self.spawn_agents(agent_configs).await
                } else {
                    Err(crate::GhostFlowError::NodeExecution(
                        "Agent configs required for spawn_agents action".to_string()
                    ))
                }
            }
            OrchestratorAction::ExecuteTask => {
                if let Some(task_def) = &input.task_definition {
                    self.execute_coordinated_task(task_def).await
                } else {
                    Err(crate::GhostFlowError::NodeExecution(
                        "Task definition required for execute_task action".to_string()
                    ))
                }
            }
            OrchestratorAction::GetStatus => self.get_orchestrator_status().await,
            _ => {
                // Implement other actions as needed
                Ok(OrchestratorOutput {
                    action_performed: input.action.clone(),
                    success: false,
                    agent_states: vec![],
                    task_results: HashMap::new(),
                    coordination_metrics: CoordinationMetrics::default(),
                    resource_usage: crate::ResourceUsage {
                        cpu_time_ms: 0,
                        memory_mb: 0,
                        network_requests: 0,
                        tokens_consumed: 0,
                    },
                    errors: vec!["Action not implemented yet".to_string()],
                })
            }
        };

        match result {
            Ok(output) => {
                self.update_health_metrics(output.success, start_time.elapsed().as_millis() as u64).await;
                
                // Update workflow context with agent states
                for agent_state in &output.agent_states {
                    context.agent_states.insert(agent_state.agent_id.clone(), agent_state.clone());
                }
                
                Ok(crate::NodeExecutionResult {
                    node_id: "orchestrator".to_string(),
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
                    node_id: "orchestrator".to_string(),
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
        // Validate max concurrent agents
        if let Some(max_agents) = config.get("max_concurrent_agents") {
            if let Some(max) = max_agents.as_u64() {
                if max == 0 || max > 100 {
                    return Err(crate::GhostFlowError::Config(
                        "max_concurrent_agents must be between 1 and 100".to_string()
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

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_agents: 10,
            agent_timeout_seconds: 300,
            health_check_interval_seconds: 30,
            auto_recovery: true,
            load_balancing: true,
            priority_scheduling: true,
            resource_limits: ResourceLimits {
                max_memory_mb: 2048,
                max_cpu_cores: 4,
                max_network_connections: 100,
                max_tokens_per_minute: 10000,
            },
        }
    }
}

impl Default for CoordinationMetrics {
    fn default() -> Self {
        Self {
            total_agents: 0,
            active_agents: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            average_task_duration_ms: 0.0,
            throughput_tasks_per_minute: 0.0,
            resource_efficiency: 0.0,
        }
    }
}