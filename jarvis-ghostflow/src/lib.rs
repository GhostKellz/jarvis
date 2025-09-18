pub mod nodes;
pub mod integration;
pub mod config;
pub mod server;
pub mod types;
pub mod memory;
pub mod orchestration;
pub mod blockchain;
pub mod network;
pub mod workflow_engine;
pub mod api;

// Re-export main components
pub use config::GhostFlowConfig;
pub use integration::{JarvisGhostFlowBridge, JarvisGhostFlowIntegration, IntegrationConfig, create_ghostflow_server};
pub use workflow_engine::{WorkflowEngine, Workflow, WorkflowNode, ExecutionResult, ExecutionMode};
pub use api::{ApiState, create_router};
pub use nodes::*;
pub use server::GhostFlowServer;
pub use types::*;

// Core error type for the integration
#[derive(Debug, thiserror::Error)]
pub enum GhostFlowError {
    #[error("Jarvis core error: {0}")]
    JarvisCore(#[from] jarvis_core::JarvisError),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Node execution error: {0}")]
    NodeExecution(String),
    
    #[error("Agent orchestration error: {0}")]
    Orchestration(String),
}

pub type Result<T> = std::result::Result<T, GhostFlowError>;