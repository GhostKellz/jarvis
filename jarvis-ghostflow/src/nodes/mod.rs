pub mod llm_router;
pub mod memory;
pub mod orchestrator;
pub mod blockchain;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{Result, WorkflowContext, NodeExecutionResult};

/// Core trait that all GhostFlow nodes must implement
#[async_trait]
pub trait GhostFlowNode: Send + Sync {
    /// Get the node type identifier
    fn node_type(&self) -> &'static str;
    
    /// Get the node display name
    fn display_name(&self) -> &str;
    
    /// Get node description
    fn description(&self) -> &str;
    
    /// Get the input schema for this node
    fn input_schema(&self) -> serde_json::Value;
    
    /// Get the output schema for this node
    fn output_schema(&self) -> serde_json::Value;
    
    /// Get the configuration schema for this node
    fn config_schema(&self) -> serde_json::Value;
    
    /// Execute the node with the given context and inputs
    async fn execute(
        &self,
        context: &mut WorkflowContext,
        inputs: HashMap<String, serde_json::Value>,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<NodeExecutionResult>;
    
    /// Validate the node configuration
    fn validate_config(&self, config: &HashMap<String, serde_json::Value>) -> Result<()>;
    
    /// Check if the node is ready to execute
    async fn can_execute(&self, context: &WorkflowContext) -> bool {
        true
    }
    
    /// Get node health status
    async fn health_check(&self) -> NodeHealth;
}

/// Node health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
    pub error_count: u32,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Node factory for creating instances
pub struct NodeFactory;

impl NodeFactory {
    pub fn create_node(node_type: &str) -> Result<Box<dyn GhostFlowNode>> {
        match node_type {
            "jarvis.llm_router" => Ok(Box::new(llm_router::LLMRouterNode::new()?)),
            "jarvis.memory" => Ok(Box::new(memory::MemoryNode::new()?)),
            "jarvis.orchestrator" => Ok(Box::new(orchestrator::OrchestratorNode::new()?)),
            "jarvis.blockchain.monitor" => Ok(Box::new(blockchain::BlockchainMonitorNode::new()?)),
            "jarvis.blockchain.transaction" => Ok(Box::new(blockchain::TransactionNode::new()?)),
            _ => Err(crate::GhostFlowError::NodeExecution(
                format!("Unknown node type: {}", node_type)
            )),
        }
    }
    
    pub fn list_available_nodes() -> Vec<NodeInfo> {
        vec![
            NodeInfo {
                node_type: "jarvis.llm_router".to_string(),
                display_name: "Smart LLM Router".to_string(),
                description: "Intelligent routing to optimal LLM providers with failover".to_string(),
                category: "AI/LLM".to_string(),
                version: "1.0.0".to_string(),
            },
            NodeInfo {
                node_type: "jarvis.memory".to_string(),
                display_name: "Context Memory".to_string(),
                description: "Persistent memory with semantic search across workflows".to_string(),
                category: "Memory".to_string(),
                version: "1.0.0".to_string(),
            },
            NodeInfo {
                node_type: "jarvis.orchestrator".to_string(),
                display_name: "Agent Orchestrator".to_string(),
                description: "Coordinate multiple AI agents with health monitoring".to_string(),
                category: "Orchestration".to_string(),
                version: "1.0.0".to_string(),
            },
            NodeInfo {
                node_type: "jarvis.blockchain.monitor".to_string(),
                display_name: "Blockchain Monitor".to_string(),
                description: "Monitor blockchain networks and smart contracts".to_string(),
                category: "Blockchain".to_string(),
                version: "1.0.0".to_string(),
            },
            NodeInfo {
                node_type: "jarvis.blockchain.transaction".to_string(),
                display_name: "Blockchain Transaction".to_string(),
                description: "Execute blockchain transactions with gas optimization".to_string(),
                category: "Blockchain".to_string(),
                version: "1.0.0".to_string(),
            },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_type: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub version: String,
}