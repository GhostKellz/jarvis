use anyhow::{Context, Result};
use axum::Router;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

use crate::api::{ApiState, create_router};
use crate::workflow_engine::WorkflowEngine;
use crate::network::QuicNetworkLayer;

/// Main integration bridge between Jarvis and GhostFlow
pub struct JarvisGhostFlowIntegration {
    workflow_engine: Arc<WorkflowEngine>,
    network_layer: QuicNetworkLayer,
    api_server: Option<ApiServer>,
    config: IntegrationConfig,
}

/// Integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub api_address: SocketAddr,
    pub enable_quic: bool,
    pub quic_address: Option<SocketAddr>,
    pub enable_websockets: bool,
    pub enable_metrics: bool,
    pub workflow_storage_path: String,
}

/// API server handle
pub struct ApiServer {
    handle: tokio::task::JoinHandle<Result<()>>,
}

impl JarvisGhostFlowIntegration {
    /// Create new integration instance
    pub async fn new(config: IntegrationConfig) -> Result<Self> {
        let workflow_engine = Arc::new(
            WorkflowEngine::new()
                .context("Failed to create workflow engine")?
        );
        
        let network_layer = QuicNetworkLayer::new().await
            .context("Failed to initialize QUIC network layer")?;
        
        Ok(Self {
            workflow_engine,
            network_layer,
            api_server: None,
            config,
        })
    }

    /// Initialize the integration with default configurations
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing Jarvis-GhostFlow integration");
        
        // Initialize workflow engine with default nodes
        self.workflow_engine.initialize_default_nodes().await
            .context("Failed to initialize default nodes")?;
        
        // Initialize network layer if enabled
        if self.config.enable_quic {
            self.network_layer.start().await
                .context("Failed to start QUIC network layer")?;
            info!("QUIC network layer started");
        }
        
        // Start API server
        self.start_api_server().await
            .context("Failed to start API server")?;
        
        info!("Jarvis-GhostFlow integration initialized successfully");
        Ok(())
    }

    /// Start the API server
    async fn start_api_server(&mut self) -> Result<()> {
        let api_state = ApiState {
            workflow_engine: self.workflow_engine.clone(),
        };
        
        let app = create_router(api_state)
            .layer(CorsLayer::permissive());
        
        let address = self.config.api_address;
        info!("Starting GhostFlow API server on {}", address);
        
        let listener = tokio::net::TcpListener::bind(address).await
            .context("Failed to bind API server")?;
        
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await
                .context("API server error")
        });
        
        self.api_server = Some(ApiServer { handle });
        
        info!("GhostFlow API server started on {}", address);
        Ok(())
    }

    /// Get workflow engine reference
    pub fn workflow_engine(&self) -> &Arc<WorkflowEngine> {
        &self.workflow_engine
    }

    /// Get network layer reference
    pub fn network_layer(&self) -> &QuicNetworkLayer {
        &self.network_layer
    }

    /// Shutdown the integration
    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down Jarvis-GhostFlow integration");
        
        // Shutdown API server
        if let Some(api_server) = self.api_server {
            api_server.handle.abort();
        }
        
        // Shutdown network layer
        if self.config.enable_quic {
            // Network layer shutdown would go here
            info!("QUIC network layer shutdown");
        }
        
        info!("Jarvis-GhostFlow integration shutdown complete");
        Ok(())
    }

    /// Create a demo workflow for testing
    pub async fn create_demo_workflow(&self) -> Result<uuid::Uuid> {
        use crate::workflow_engine::{
            Workflow, WorkflowNode, Connection, Position, WorkflowSettings, 
            WorkflowMetadata, WorkflowState, CallerPolicy
        };
        use std::collections::HashMap;

        let workflow_id = uuid::Uuid::new_v4();
        
        let mut nodes = HashMap::new();
        
        // Start node
        nodes.insert("start".to_string(), WorkflowNode {
            id: "start".to_string(),
            node_type: "start".to_string(),
            position: Position { x: 100.0, y: 100.0 },
            parameters: serde_json::json!({}),
            disabled: false,
            retry_on_fail: false,
            retry_count: 0,
            timeout_seconds: Some(30),
        });
        
        // LLM Router node
        nodes.insert("llm_router".to_string(), WorkflowNode {
            id: "llm_router".to_string(),
            node_type: "llm_router".to_string(),
            position: Position { x: 300.0, y: 100.0 },
            parameters: serde_json::json!({
                "provider": "openai",
                "model": "gpt-4",
                "prompt": "Process the following input: {{input}}"
            }),
            disabled: false,
            retry_on_fail: true,
            retry_count: 3,
            timeout_seconds: Some(60),
        });
        
        // Memory node
        nodes.insert("memory".to_string(), WorkflowNode {
            id: "memory".to_string(),
            node_type: "memory".to_string(),
            position: Position { x: 500.0, y: 100.0 },
            parameters: serde_json::json!({
                "key": "workflow_context",
                "operation": "store"
            }),
            disabled: false,
            retry_on_fail: false,
            retry_count: 0,
            timeout_seconds: Some(10),
        });
        
        let connections = vec![
            Connection {
                source_node: "start".to_string(),
                source_output: "output".to_string(),
                target_node: "llm_router".to_string(),
                target_input: "input".to_string(),
            },
            Connection {
                source_node: "llm_router".to_string(),
                source_output: "output".to_string(),
                target_node: "memory".to_string(),
                target_input: "input".to_string(),
            },
        ];
        
        let workflow = Workflow {
            id: workflow_id,
            name: "Demo AI Workflow".to_string(),
            description: Some("A demonstration workflow showing Jarvis AI integration".to_string()),
            version: "1.0.0".to_string(),
            nodes,
            connections,
            settings: WorkflowSettings {
                timeout_seconds: 300,
                error_workflow: None,
                save_data_execution_progress: true,
                save_data_success: true,
                save_data_error: true,
                save_manual_executions: true,
                caller_policy: CallerPolicy::WorkflowsFromSameOwner,
            },
            metadata: WorkflowMetadata {
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                created_by: "system".to_string(),
                tags: vec!["demo".to_string(), "ai".to_string(), "jarvis".to_string()],
                folder: Some("examples".to_string()),
            },
            state: WorkflowState::Active,
        };
        
        self.workflow_engine.create_workflow(workflow).await
            .context("Failed to create demo workflow")
    }

    /// Execute the demo workflow
    pub async fn execute_demo_workflow(&self, workflow_id: uuid::Uuid) -> Result<()> {
        use crate::workflow_engine::ExecutionMode;
        
        let trigger_data = serde_json::json!({
            "input": "Hello from Jarvis-GhostFlow integration!",
            "user_id": "demo_user",
            "timestamp": chrono::Utc::now()
        });
        
        let result = self.workflow_engine.execute_workflow(
            workflow_id,
            trigger_data,
            ExecutionMode::Manual,
        ).await
        .context("Failed to execute demo workflow")?;
        
        info!("Demo workflow executed successfully: {:?}", result.status);
        info!("Execution ID: {}", result.execution_id);
        info!("Duration: {:?}ms", result.duration_ms);
        
        if let Some(error) = result.error {
            warn!("Workflow execution had errors: {}", error);
        }
        
        Ok(())
    }
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            api_address: "127.0.0.1:8080".parse().unwrap(),
            enable_quic: true,
            quic_address: Some("127.0.0.1:8081".parse().unwrap()),
            enable_websockets: true,
            enable_metrics: true,
            workflow_storage_path: "./workflows".to_string(),
        }
    }
}

/// Integration metrics
#[derive(Debug, Default, Clone, Serialize)]
pub struct IntegrationMetrics {
    pub total_workflows: u64,
    pub active_workflows: u64,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time_ms: f64,
    pub api_requests: u64,
    pub websocket_connections: u32,
}

/// Create a full-featured GhostFlow server instance
pub async fn create_ghostflow_server(config: Option<IntegrationConfig>) -> Result<JarvisGhostFlowIntegration> {
    let config = config.unwrap_or_default();
    
    let mut integration = JarvisGhostFlowIntegration::new(config).await
        .context("Failed to create GhostFlow integration")?;
    
    integration.initialize().await
        .context("Failed to initialize GhostFlow integration")?;
    
    // Create and execute demo workflow for testing
    let demo_workflow_id = integration.create_demo_workflow().await
        .context("Failed to create demo workflow")?;
    
    info!("Created demo workflow: {}", demo_workflow_id);
    
    // Optionally execute the demo workflow
    if std::env::var("GHOSTFLOW_RUN_DEMO").unwrap_or_default() == "true" {
        let integration_clone = &integration;
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            if let Err(e) = integration_clone.execute_demo_workflow(demo_workflow_id).await {
                warn!("Failed to execute demo workflow: {}", e);
            }
        });
    }
    
    Ok(integration)
}