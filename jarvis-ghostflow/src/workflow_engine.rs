use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::nodes::{
    NodeDefinition, NodeInstance, NodeOutput, ExecutionContext,
    llm_router::LLMRouterNode,
    memory::MemoryNode,
    orchestrator::OrchestratorNode,
    blockchain::BlockchainNode,
};

/// Main workflow execution engine
pub struct WorkflowEngine {
    workflows: Arc<RwLock<HashMap<Uuid, Workflow>>>,
    node_registry: Arc<RwLock<HashMap<String, Box<dyn NodeDefinition + Send + Sync>>>>,
    execution_queue: mpsc::UnboundedSender<ExecutionRequest>,
    metrics: WorkflowMetrics,
}

/// Workflow definition structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub nodes: HashMap<String, WorkflowNode>,
    pub connections: Vec<Connection>,
    pub settings: WorkflowSettings,
    pub metadata: WorkflowMetadata,
    pub state: WorkflowState,
}

/// Individual node in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub node_type: String,
    pub position: Position,
    pub parameters: serde_json::Value,
    pub disabled: bool,
    pub retry_on_fail: bool,
    pub retry_count: u32,
    pub timeout_seconds: Option<u32>,
}

/// Connection between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub source_node: String,
    pub source_output: String,
    pub target_node: String,
    pub target_input: String,
}

/// Node position in visual editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Workflow settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSettings {
    pub timeout_seconds: u32,
    pub error_workflow: Option<Uuid>,
    pub save_data_execution_progress: bool,
    pub save_data_success: bool,
    pub save_data_error: bool,
    pub save_manual_executions: bool,
    pub caller_policy: CallerPolicy,
}

/// Workflow metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_by: String,
    pub tags: Vec<String>,
    pub folder: Option<String>,
}

/// Workflow execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowState {
    Active,
    Paused,
    Inactive,
    Error,
}

/// Caller policy for workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallerPolicy {
    WorkflowsFromSameOwner,
    WorkflowsFromAnyOwner,
    None,
}

/// Execution request
#[derive(Debug)]
pub struct ExecutionRequest {
    pub workflow_id: Uuid,
    pub trigger_data: serde_json::Value,
    pub execution_mode: ExecutionMode,
    pub response_sender: Option<mpsc::UnboundedSender<ExecutionResult>>,
}

/// Execution mode
#[derive(Debug, Clone)]
pub enum ExecutionMode {
    Manual,
    Trigger,
    Webhook,
    Scheduled,
    Integration,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
    pub status: ExecutionStatus,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<u64>,
    pub data: serde_json::Value,
    pub error: Option<String>,
    pub node_executions: Vec<NodeExecution>,
}

/// Individual node execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecution {
    pub node_id: String,
    pub node_type: String,
    pub status: ExecutionStatus,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<u64>,
    pub input_data: serde_json::Value,
    pub output_data: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Running,
    Success,
    Error,
    Canceled,
    Waiting,
}

/// Workflow metrics
#[derive(Debug, Default)]
pub struct WorkflowMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time_ms: f64,
    pub active_workflows: u32,
}

impl WorkflowEngine {
    /// Create new workflow engine
    pub fn new() -> Result<Self> {
        let (tx, mut rx) = mpsc::unbounded_channel::<ExecutionRequest>();
        
        let workflows = Arc::new(RwLock::new(HashMap::new()));
        let node_registry = Arc::new(RwLock::new(HashMap::new()));
        
        let engine = Self {
            workflows: workflows.clone(),
            node_registry: node_registry.clone(),
            execution_queue: tx,
            metrics: WorkflowMetrics::default(),
        };
        
        // Start execution processor
        let workflows_clone = workflows.clone();
        let node_registry_clone = node_registry.clone();
        tokio::spawn(async move {
            while let Some(request) = rx.recv().await {
                Self::process_execution_request(
                    request,
                    workflows_clone.clone(),
                    node_registry_clone.clone(),
                ).await;
            }
        });
        
        info!("Workflow engine initialized");
        Ok(engine)
    }

    /// Initialize with default node types
    pub async fn initialize_default_nodes(&self) -> Result<()> {
        let mut registry = self.node_registry.write().await;
        
        // Register core Jarvis nodes
        registry.insert("llm_router".to_string(), Box::new(LLMRouterNode::new()));
        registry.insert("memory".to_string(), Box::new(MemoryNode::new()));
        registry.insert("orchestrator".to_string(), Box::new(OrchestratorNode::new()));
        registry.insert("blockchain".to_string(), Box::new(BlockchainNode::new()));
        
        // Register system nodes
        registry.insert("start".to_string(), Box::new(StartNode::new()));
        registry.insert("merge".to_string(), Box::new(MergeNode::new()));
        registry.insert("split".to_string(), Box::new(SplitNode::new()));
        registry.insert("function".to_string(), Box::new(FunctionNode::new()));
        registry.insert("http_request".to_string(), Box::new(HttpRequestNode::new()));
        registry.insert("webhook".to_string(), Box::new(WebhookNode::new()));
        registry.insert("schedule_trigger".to_string(), Box::new(ScheduleTriggerNode::new()));
        
        info!("Default nodes registered in workflow engine");
        Ok(())
    }

    /// Create new workflow
    pub async fn create_workflow(&self, workflow: Workflow) -> Result<Uuid> {
        let mut workflows = self.workflows.write().await;
        let workflow_id = workflow.id;
        workflows.insert(workflow_id, workflow);
        
        info!("Created workflow: {}", workflow_id);
        Ok(workflow_id)
    }

    /// Get workflow by ID
    pub async fn get_workflow(&self, workflow_id: Uuid) -> Result<Option<Workflow>> {
        let workflows = self.workflows.read().await;
        Ok(workflows.get(&workflow_id).cloned())
    }

    /// List all workflows
    pub async fn list_workflows(&self) -> Result<Vec<Workflow>> {
        let workflows = self.workflows.read().await;
        Ok(workflows.values().cloned().collect())
    }

    /// Update workflow
    pub async fn update_workflow(&self, workflow: Workflow) -> Result<()> {
        let mut workflows = self.workflows.write().await;
        let workflow_id = workflow.id;
        
        if let Some(existing) = workflows.get_mut(&workflow_id) {
            existing.updated_at = chrono::Utc::now();
            *existing = workflow;
            info!("Updated workflow: {}", workflow_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Workflow not found: {}", workflow_id))
        }
    }

    /// Delete workflow
    pub async fn delete_workflow(&self, workflow_id: Uuid) -> Result<()> {
        let mut workflows = self.workflows.write().await;
        
        if workflows.remove(&workflow_id).is_some() {
            info!("Deleted workflow: {}", workflow_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Workflow not found: {}", workflow_id))
        }
    }

    /// Execute workflow
    pub async fn execute_workflow(
        &self,
        workflow_id: Uuid,
        trigger_data: serde_json::Value,
        execution_mode: ExecutionMode,
    ) -> Result<ExecutionResult> {
        let (tx, mut rx) = mpsc::unbounded_channel::<ExecutionResult>();
        
        let request = ExecutionRequest {
            workflow_id,
            trigger_data,
            execution_mode,
            response_sender: Some(tx),
        };
        
        self.execution_queue.send(request)
            .context("Failed to queue execution request")?;
        
        rx.recv().await
            .ok_or_else(|| anyhow::anyhow!("Execution result not received"))
    }

    /// Process execution request
    async fn process_execution_request(
        request: ExecutionRequest,
        workflows: Arc<RwLock<HashMap<Uuid, Workflow>>>,
        node_registry: Arc<RwLock<HashMap<String, Box<dyn NodeDefinition + Send + Sync>>>>,
    ) {
        let execution_id = Uuid::new_v4();
        let start_time = chrono::Utc::now();
        
        debug!("Processing execution request: {} for workflow: {}", execution_id, request.workflow_id);
        
        let result = match Self::execute_workflow_internal(
            execution_id,
            request.workflow_id,
            request.trigger_data,
            workflows,
            node_registry,
        ).await {
            Ok(mut result) => {
                result.end_time = Some(chrono::Utc::now());
                if let Some(end_time) = result.end_time {
                    result.duration_ms = Some(
                        (end_time - start_time).num_milliseconds() as u64
                    );
                }
                result
            }
            Err(e) => {
                error!("Workflow execution failed: {}", e);
                ExecutionResult {
                    execution_id,
                    workflow_id: request.workflow_id,
                    status: ExecutionStatus::Error,
                    start_time,
                    end_time: Some(chrono::Utc::now()),
                    duration_ms: Some(
                        (chrono::Utc::now() - start_time).num_milliseconds() as u64
                    ),
                    data: serde_json::json!({}),
                    error: Some(e.to_string()),
                    node_executions: vec![],
                }
            }
        };
        
        if let Some(sender) = request.response_sender {
            if let Err(e) = sender.send(result) {
                error!("Failed to send execution result: {}", e);
            }
        }
    }

    /// Internal workflow execution logic
    async fn execute_workflow_internal(
        execution_id: Uuid,
        workflow_id: Uuid,
        trigger_data: serde_json::Value,
        workflows: Arc<RwLock<HashMap<Uuid, Workflow>>>,
        node_registry: Arc<RwLock<HashMap<String, Box<dyn NodeDefinition + Send + Sync>>>>,
    ) -> Result<ExecutionResult> {
        let workflow = {
            let workflows_guard = workflows.read().await;
            workflows_guard.get(&workflow_id)
                .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", workflow_id))?
                .clone()
        };

        if workflow.state != WorkflowState::Active {
            return Err(anyhow::anyhow!("Workflow is not active: {:?}", workflow.state));
        }

        let mut execution_result = ExecutionResult {
            execution_id,
            workflow_id,
            status: ExecutionStatus::Running,
            start_time: chrono::Utc::now(),
            end_time: None,
            duration_ms: None,
            data: serde_json::json!({}),
            error: None,
            node_executions: vec![],
        };

        // Find start nodes
        let start_nodes = workflow.nodes.iter()
            .filter(|(_, node)| node.node_type == "start")
            .map(|(id, node)| (id.clone(), node.clone()))
            .collect::<Vec<_>>();

        if start_nodes.is_empty() {
            return Err(anyhow::anyhow!("No start node found in workflow"));
        }

        // Execute workflow nodes
        let mut execution_context = ExecutionContext {
            workflow_id,
            execution_id,
            data: trigger_data,
            node_outputs: HashMap::new(),
        };

        // Topological sort for node execution order
        let execution_order = Self::calculate_execution_order(&workflow)?;
        
        for node_id in execution_order {
            if let Some(node) = workflow.nodes.get(&node_id) {
                if node.disabled {
                    debug!("Skipping disabled node: {}", node_id);
                    continue;
                }

                let node_start_time = chrono::Utc::now();
                let node_execution_result = Self::execute_node(
                    node,
                    &mut execution_context,
                    &node_registry,
                ).await;

                let node_end_time = chrono::Utc::now();
                let node_duration = (node_end_time - node_start_time).num_milliseconds() as u64;

                let node_execution = match node_execution_result {
                    Ok(output) => {
                        execution_context.node_outputs.insert(node_id.clone(), output.clone());
                        
                        NodeExecution {
                            node_id: node_id.clone(),
                            node_type: node.node_type.clone(),
                            status: ExecutionStatus::Success,
                            start_time: node_start_time,
                            end_time: Some(node_end_time),
                            duration_ms: Some(node_duration),
                            input_data: node.parameters.clone(),
                            output_data: Some(output.data),
                            error: None,
                        }
                    }
                    Err(e) => {
                        error!("Node execution failed: {} - {}", node_id, e);
                        
                        let node_execution = NodeExecution {
                            node_id: node_id.clone(),
                            node_type: node.node_type.clone(),
                            status: ExecutionStatus::Error,
                            start_time: node_start_time,
                            end_time: Some(node_end_time),
                            duration_ms: Some(node_duration),
                            input_data: node.parameters.clone(),
                            output_data: None,
                            error: Some(e.to_string()),
                        };

                        execution_result.node_executions.push(node_execution);
                        execution_result.status = ExecutionStatus::Error;
                        execution_result.error = Some(format!("Node {} failed: {}", node_id, e));
                        
                        return Ok(execution_result);
                    }
                };

                execution_result.node_executions.push(node_execution);
            }
        }

        execution_result.status = ExecutionStatus::Success;
        execution_result.data = serde_json::to_value(execution_context.node_outputs)?;
        
        info!("Workflow execution completed: {}", execution_id);
        Ok(execution_result)
    }

    /// Execute individual node
    async fn execute_node(
        node: &WorkflowNode,
        context: &mut ExecutionContext,
        node_registry: &Arc<RwLock<HashMap<String, Box<dyn NodeDefinition + Send + Sync>>>>,
    ) -> Result<NodeOutput> {
        let registry = node_registry.read().await;
        
        if let Some(node_def) = registry.get(&node.node_type) {
            // Create node instance
            let mut node_instance = node_def.create_instance()?;
            
            // Configure node
            node_instance.configure(node.parameters.clone()).await?;
            
            // Execute node
            debug!("Executing node: {} ({})", node.id, node.node_type);
            node_instance.execute(context).await
        } else {
            Err(anyhow::anyhow!("Unknown node type: {}", node.node_type))
        }
    }

    /// Calculate execution order using topological sort
    fn calculate_execution_order(workflow: &Workflow) -> Result<Vec<String>> {
        let mut in_degree = HashMap::new();
        let mut graph = HashMap::new();
        
        // Initialize in-degree and graph
        for node_id in workflow.nodes.keys() {
            in_degree.insert(node_id.clone(), 0);
            graph.insert(node_id.clone(), Vec::new());
        }
        
        // Build graph and calculate in-degrees
        for connection in &workflow.connections {
            graph.entry(connection.source_node.clone())
                .or_insert_with(Vec::new)
                .push(connection.target_node.clone());
            
            *in_degree.entry(connection.target_node.clone()).or_insert(0) += 1;
        }
        
        // Topological sort using Kahn's algorithm
        let mut queue = Vec::new();
        let mut result = Vec::new();
        
        // Find nodes with no incoming edges
        for (node_id, degree) in &in_degree {
            if *degree == 0 {
                queue.push(node_id.clone());
            }
        }
        
        while let Some(node_id) = queue.pop() {
            result.push(node_id.clone());
            
            if let Some(neighbors) = graph.get(&node_id) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }
        
        if result.len() != workflow.nodes.len() {
            return Err(anyhow::anyhow!("Circular dependency detected in workflow"));
        }
        
        Ok(result)
    }

    /// Get workflow metrics
    pub fn get_metrics(&self) -> &WorkflowMetrics {
        &self.metrics
    }
}

// Basic node implementations for system functionality

/// Start node - entry point for workflows
pub struct StartNode;

impl StartNode {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NodeDefinition for StartNode {
    fn node_type(&self) -> &'static str {
        "start"
    }

    fn create_instance(&self) -> Result<Box<dyn NodeInstance + Send + Sync>> {
        Ok(Box::new(StartNodeInstance))
    }
}

pub struct StartNodeInstance;

#[async_trait::async_trait]
impl NodeInstance for StartNodeInstance {
    async fn configure(&mut self, _parameters: serde_json::Value) -> Result<()> {
        Ok(())
    }

    async fn execute(&mut self, context: &ExecutionContext) -> Result<NodeOutput> {
        Ok(NodeOutput {
            data: context.data.clone(),
        })
    }
}

/// Merge node - combines multiple inputs
pub struct MergeNode;

impl MergeNode {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NodeDefinition for MergeNode {
    fn node_type(&self) -> &'static str {
        "merge"
    }

    fn create_instance(&self) -> Result<Box<dyn NodeInstance + Send + Sync>> {
        Ok(Box::new(MergeNodeInstance))
    }
}

pub struct MergeNodeInstance;

#[async_trait::async_trait]
impl NodeInstance for MergeNodeInstance {
    async fn configure(&mut self, _parameters: serde_json::Value) -> Result<()> {
        Ok(())
    }

    async fn execute(&mut self, context: &ExecutionContext) -> Result<NodeOutput> {
        // Merge all node outputs
        let merged_data = serde_json::to_value(&context.node_outputs)?;
        
        Ok(NodeOutput {
            data: merged_data,
        })
    }
}

/// Split node - splits input to multiple outputs
pub struct SplitNode;

impl SplitNode {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NodeDefinition for SplitNode {
    fn node_type(&self) -> &'static str {
        "split"
    }

    fn create_instance(&self) -> Result<Box<dyn NodeInstance + Send + Sync>> {
        Ok(Box::new(SplitNodeInstance))
    }
}

pub struct SplitNodeInstance;

#[async_trait::async_trait]
impl NodeInstance for SplitNodeInstance {
    async fn configure(&mut self, _parameters: serde_json::Value) -> Result<()> {
        Ok(())
    }

    async fn execute(&mut self, context: &ExecutionContext) -> Result<NodeOutput> {
        Ok(NodeOutput {
            data: context.data.clone(),
        })
    }
}

/// Function node - custom JavaScript/Rust function execution
pub struct FunctionNode;

impl FunctionNode {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NodeDefinition for FunctionNode {
    fn node_type(&self) -> &'static str {
        "function"
    }

    fn create_instance(&self) -> Result<Box<dyn NodeInstance + Send + Sync>> {
        Ok(Box::new(FunctionNodeInstance { code: String::new() }))
    }
}

pub struct FunctionNodeInstance {
    code: String,
}

#[async_trait::async_trait]
impl NodeInstance for FunctionNodeInstance {
    async fn configure(&mut self, parameters: serde_json::Value) -> Result<()> {
        if let Some(code) = parameters.get("code").and_then(|v| v.as_str()) {
            self.code = code.to_string();
        }
        Ok(())
    }

    async fn execute(&mut self, context: &ExecutionContext) -> Result<NodeOutput> {
        // For now, just return the input data
        // In the future, this could execute JavaScript/WASM code
        warn!("Function node execution not implemented, returning input data");
        
        Ok(NodeOutput {
            data: context.data.clone(),
        })
    }
}

/// HTTP Request node
pub struct HttpRequestNode;

impl HttpRequestNode {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NodeDefinition for HttpRequestNode {
    fn node_type(&self) -> &'static str {
        "http_request"
    }

    fn create_instance(&self) -> Result<Box<dyn NodeInstance + Send + Sync>> {
        Ok(Box::new(HttpRequestNodeInstance {
            url: String::new(),
            method: "GET".to_string(),
            headers: HashMap::new(),
        }))
    }
}

pub struct HttpRequestNodeInstance {
    url: String,
    method: String,
    headers: HashMap<String, String>,
}

#[async_trait::async_trait]
impl NodeInstance for HttpRequestNodeInstance {
    async fn configure(&mut self, parameters: serde_json::Value) -> Result<()> {
        if let Some(url) = parameters.get("url").and_then(|v| v.as_str()) {
            self.url = url.to_string();
        }
        if let Some(method) = parameters.get("method").and_then(|v| v.as_str()) {
            self.method = method.to_string();
        }
        if let Some(headers) = parameters.get("headers").and_then(|v| v.as_object()) {
            for (key, value) in headers {
                if let Some(value_str) = value.as_str() {
                    self.headers.insert(key.clone(), value_str.to_string());
                }
            }
        }
        Ok(())
    }

    async fn execute(&mut self, _context: &ExecutionContext) -> Result<NodeOutput> {
        let client = reqwest::Client::new();
        let mut request = match self.method.as_str() {
            "GET" => client.get(&self.url),
            "POST" => client.post(&self.url),
            "PUT" => client.put(&self.url),
            "DELETE" => client.delete(&self.url),
            _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", self.method)),
        };

        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        let response = request.send().await?;
        let status = response.status().as_u16();
        let body = response.text().await?;

        Ok(NodeOutput {
            data: serde_json::json!({
                "status": status,
                "body": body,
            }),
        })
    }
}

/// Webhook node
pub struct WebhookNode;

impl WebhookNode {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NodeDefinition for WebhookNode {
    fn node_type(&self) -> &'static str {
        "webhook"
    }

    fn create_instance(&self) -> Result<Box<dyn NodeInstance + Send + Sync>> {
        Ok(Box::new(WebhookNodeInstance))
    }
}

pub struct WebhookNodeInstance;

#[async_trait::async_trait]
impl NodeInstance for WebhookNodeInstance {
    async fn configure(&mut self, _parameters: serde_json::Value) -> Result<()> {
        Ok(())
    }

    async fn execute(&mut self, context: &ExecutionContext) -> Result<NodeOutput> {
        // Webhook nodes are typically triggers, so they pass through data
        Ok(NodeOutput {
            data: context.data.clone(),
        })
    }
}

/// Schedule trigger node
pub struct ScheduleTriggerNode;

impl ScheduleTriggerNode {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NodeDefinition for ScheduleTriggerNode {
    fn node_type(&self) -> &'static str {
        "schedule_trigger"
    }

    fn create_instance(&self) -> Result<Box<dyn NodeInstance + Send + Sync>> {
        Ok(Box::new(ScheduleTriggerNodeInstance))
    }
}

pub struct ScheduleTriggerNodeInstance;

#[async_trait::async_trait]
impl NodeInstance for ScheduleTriggerNodeInstance {
    async fn configure(&mut self, _parameters: serde_json::Value) -> Result<()> {
        Ok(())
    }

    async fn execute(&mut self, context: &ExecutionContext) -> Result<NodeOutput> {
        // Schedule triggers provide timing data
        Ok(NodeOutput {
            data: serde_json::json!({
                "trigger_time": chrono::Utc::now(),
                "data": context.data,
            }),
        })
    }
}