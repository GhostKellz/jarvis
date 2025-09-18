use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};
use uuid::Uuid;

use crate::workflow_engine::{
    WorkflowEngine, Workflow, ExecutionMode, ExecutionResult, WorkflowMetrics
};

/// API state
#[derive(Clone)]
pub struct ApiState {
    pub workflow_engine: Arc<WorkflowEngine>,
}

/// API error response
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// API success response
#[derive(Serialize)]
pub struct SuccessResponse<T> {
    pub data: T,
}

/// Workflow creation request
#[derive(Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub nodes: HashMap<String, serde_json::Value>,
    pub connections: Vec<serde_json::Value>,
    pub settings: Option<serde_json::Value>,
    pub tags: Option<Vec<String>>,
}

/// Workflow execution request
#[derive(Deserialize)]
pub struct ExecuteWorkflowRequest {
    pub trigger_data: Option<serde_json::Value>,
    pub execution_mode: Option<String>,
}

/// Workflow list query parameters
#[derive(Deserialize)]
pub struct WorkflowListQuery {
    pub tag: Option<String>,
    pub folder: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Create API router
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        // Workflow management endpoints
        .route("/api/workflows", post(create_workflow))
        .route("/api/workflows", get(list_workflows))
        .route("/api/workflows/:id", get(get_workflow))
        .route("/api/workflows/:id", put(update_workflow))
        .route("/api/workflows/:id", delete(delete_workflow))
        
        // Workflow execution endpoints
        .route("/api/workflows/:id/execute", post(execute_workflow))
        .route("/api/executions/:id", get(get_execution))
        
        // Node management endpoints
        .route("/api/node-types", get(list_node_types))
        .route("/api/node-types/:type", get(get_node_type))
        
        // Metrics and monitoring
        .route("/api/metrics", get(get_metrics))
        .route("/api/health", get(health_check))
        
        // WebSocket endpoint for real-time updates
        .route("/ws", get(websocket_handler))
        
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Create new workflow
async fn create_workflow(
    State(state): State<ApiState>,
    Json(request): Json<CreateWorkflowRequest>,
) -> Result<Json<SuccessResponse<Workflow>>, (StatusCode, Json<ErrorResponse>)> {
    let workflow = Workflow {
        id: Uuid::new_v4(),
        name: request.name,
        description: request.description,
        version: "1.0.0".to_string(),
        nodes: request.nodes.into_iter()
            .map(|(id, data)| {
                let node = serde_json::from_value(data).map_err(|e| {
                    (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                        error: format!("Invalid node data: {}", e),
                    }))
                })?;
                Ok((id, node))
            })
            .collect::<Result<HashMap<_, _>, _>>()?,
        connections: request.connections.into_iter()
            .map(|data| serde_json::from_value(data).map_err(|e| {
                (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                    error: format!("Invalid connection data: {}", e),
                }))
            }))
            .collect::<Result<Vec<_>, _>>()?,
        settings: request.settings
            .map(|data| serde_json::from_value(data))
            .transpose()
            .map_err(|e| {
                (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                    error: format!("Invalid settings data: {}", e),
                }))
            })?
            .unwrap_or_default(),
        metadata: crate::workflow_engine::WorkflowMetadata {
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            created_by: "api".to_string(), // TODO: Get from auth
            tags: request.tags.unwrap_or_default(),
            folder: None,
        },
        state: crate::workflow_engine::WorkflowState::Active,
    };

    let workflow_id = state.workflow_engine.create_workflow(workflow.clone()).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: format!("Failed to create workflow: {}", e),
            }))
        })?;

    info!("Created workflow via API: {}", workflow_id);
    
    Ok(Json(SuccessResponse {
        data: workflow,
    }))
}

/// List workflows
async fn list_workflows(
    State(state): State<ApiState>,
    Query(query): Query<WorkflowListQuery>,
) -> Result<Json<SuccessResponse<Vec<Workflow>>>, (StatusCode, Json<ErrorResponse>)> {
    let mut workflows = state.workflow_engine.list_workflows().await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: format!("Failed to list workflows: {}", e),
            }))
        })?;

    // Apply filters
    if let Some(tag) = &query.tag {
        workflows.retain(|w| w.metadata.tags.contains(tag));
    }

    if let Some(folder) = &query.folder {
        workflows.retain(|w| w.metadata.folder.as_ref() == Some(folder));
    }

    // Apply pagination
    if let Some(offset) = query.offset {
        let offset = offset as usize;
        if offset < workflows.len() {
            workflows = workflows[offset..].to_vec();
        } else {
            workflows.clear();
        }
    }

    if let Some(limit) = query.limit {
        let limit = limit as usize;
        if workflows.len() > limit {
            workflows.truncate(limit);
        }
    }

    Ok(Json(SuccessResponse {
        data: workflows,
    }))
}

/// Get workflow by ID
async fn get_workflow(
    State(state): State<ApiState>,
    Path(workflow_id): Path<Uuid>,
) -> Result<Json<SuccessResponse<Workflow>>, (StatusCode, Json<ErrorResponse>)> {
    let workflow = state.workflow_engine.get_workflow(workflow_id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: format!("Failed to get workflow: {}", e),
            }))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse {
                error: "Workflow not found".to_string(),
            }))
        })?;

    Ok(Json(SuccessResponse {
        data: workflow,
    }))
}

/// Update workflow
async fn update_workflow(
    State(state): State<ApiState>,
    Path(workflow_id): Path<Uuid>,
    Json(request): Json<CreateWorkflowRequest>,
) -> Result<Json<SuccessResponse<Workflow>>, (StatusCode, Json<ErrorResponse>)> {
    // Get existing workflow
    let mut workflow = state.workflow_engine.get_workflow(workflow_id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: format!("Failed to get workflow: {}", e),
            }))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse {
                error: "Workflow not found".to_string(),
            }))
        })?;

    // Update workflow fields
    workflow.name = request.name;
    workflow.description = request.description;
    workflow.nodes = request.nodes.into_iter()
        .map(|(id, data)| {
            let node = serde_json::from_value(data).map_err(|e| {
                (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                    error: format!("Invalid node data: {}", e),
                }))
            })?;
            Ok((id, node))
        })
        .collect::<Result<HashMap<_, _>, _>()?;
    workflow.connections = request.connections.into_iter()
        .map(|data| serde_json::from_value(data).map_err(|e| {
            (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                error: format!("Invalid connection data: {}", e),
            }))
        }))
        .collect::<Result<Vec<_>, _>()?;
    workflow.metadata.updated_at = chrono::Utc::now();
    if let Some(tags) = request.tags {
        workflow.metadata.tags = tags;
    }

    state.workflow_engine.update_workflow(workflow.clone()).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: format!("Failed to update workflow: {}", e),
            }))
        })?;

    info!("Updated workflow via API: {}", workflow_id);

    Ok(Json(SuccessResponse {
        data: workflow,
    }))
}

/// Delete workflow
async fn delete_workflow(
    State(state): State<ApiState>,
    Path(workflow_id): Path<Uuid>,
) -> Result<Json<SuccessResponse<()>>, (StatusCode, Json<ErrorResponse>)> {
    state.workflow_engine.delete_workflow(workflow_id).await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: format!("Failed to delete workflow: {}", e),
            }))
        })?;

    info!("Deleted workflow via API: {}", workflow_id);

    Ok(Json(SuccessResponse {
        data: (),
    }))
}

/// Execute workflow
async fn execute_workflow(
    State(state): State<ApiState>,
    Path(workflow_id): Path<Uuid>,
    Json(request): Json<ExecuteWorkflowRequest>,
) -> Result<Json<SuccessResponse<ExecutionResult>>, (StatusCode, Json<ErrorResponse>)> {
    let execution_mode = match request.execution_mode.as_deref() {
        Some("manual") => ExecutionMode::Manual,
        Some("trigger") => ExecutionMode::Trigger,
        Some("webhook") => ExecutionMode::Webhook,
        Some("scheduled") => ExecutionMode::Scheduled,
        Some("integration") => ExecutionMode::Integration,
        _ => ExecutionMode::Manual,
    };

    let trigger_data = request.trigger_data.unwrap_or_else(|| serde_json::json!({}));

    let result = state.workflow_engine.execute_workflow(
        workflow_id,
        trigger_data,
        execution_mode,
    ).await
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to execute workflow: {}", e),
        }))
    })?;

    info!("Executed workflow via API: {} -> {}", workflow_id, result.execution_id);

    Ok(Json(SuccessResponse {
        data: result,
    }))
}

/// Get execution result
async fn get_execution(
    _State(_state): State<ApiState>,
    Path(_execution_id): Path<Uuid>,
) -> Result<Json<SuccessResponse<ExecutionResult>>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement execution storage and retrieval
    Err((StatusCode::NOT_IMPLEMENTED, Json(ErrorResponse {
        error: "Execution history not implemented yet".to_string(),
    })))
}

/// List available node types
async fn list_node_types(
    _State(_state): State<ApiState>,
) -> Result<Json<SuccessResponse<Vec<NodeTypeInfo>>>, (StatusCode, Json<ErrorResponse>)> {
    let node_types = vec![
        NodeTypeInfo {
            name: "start".to_string(),
            display_name: "Start".to_string(),
            description: "Workflow entry point".to_string(),
            category: "Core".to_string(),
            inputs: vec![],
            outputs: vec![NodePort {
                name: "output".to_string(),
                data_type: "any".to_string(),
                required: false,
            }],
            parameters: vec![],
        },
        NodeTypeInfo {
            name: "llm_router".to_string(),
            display_name: "LLM Router".to_string(),
            description: "Route requests to appropriate LLM providers".to_string(),
            category: "AI".to_string(),
            inputs: vec![NodePort {
                name: "input".to_string(),
                data_type: "string".to_string(),
                required: true,
            }],
            outputs: vec![NodePort {
                name: "output".to_string(),
                data_type: "object".to_string(),
                required: false,
            }],
            parameters: vec![
                NodeParameter {
                    name: "provider".to_string(),
                    display_name: "Provider".to_string(),
                    data_type: "string".to_string(),
                    required: true,
                    default_value: Some(serde_json::json!("openai")),
                    description: "LLM provider to use".to_string(),
                },
            ],
        },
        NodeTypeInfo {
            name: "memory".to_string(),
            display_name: "Memory".to_string(),
            description: "Store and retrieve workflow context".to_string(),
            category: "Data".to_string(),
            inputs: vec![NodePort {
                name: "input".to_string(),
                data_type: "any".to_string(),
                required: true,
            }],
            outputs: vec![NodePort {
                name: "output".to_string(),
                data_type: "any".to_string(),
                required: false,
            }],
            parameters: vec![
                NodeParameter {
                    name: "key".to_string(),
                    display_name: "Memory Key".to_string(),
                    data_type: "string".to_string(),
                    required: true,
                    default_value: None,
                    description: "Key for storing/retrieving data".to_string(),
                },
            ],
        },
        NodeTypeInfo {
            name: "http_request".to_string(),
            display_name: "HTTP Request".to_string(),
            description: "Make HTTP requests".to_string(),
            category: "Network".to_string(),
            inputs: vec![NodePort {
                name: "input".to_string(),
                data_type: "any".to_string(),
                required: false,
            }],
            outputs: vec![NodePort {
                name: "output".to_string(),
                data_type: "object".to_string(),
                required: false,
            }],
            parameters: vec![
                NodeParameter {
                    name: "url".to_string(),
                    display_name: "URL".to_string(),
                    data_type: "string".to_string(),
                    required: true,
                    default_value: None,
                    description: "Request URL".to_string(),
                },
                NodeParameter {
                    name: "method".to_string(),
                    display_name: "Method".to_string(),
                    data_type: "string".to_string(),
                    required: true,
                    default_value: Some(serde_json::json!("GET")),
                    description: "HTTP method".to_string(),
                },
            ],
        },
    ];

    Ok(Json(SuccessResponse {
        data: node_types,
    }))
}

/// Get node type information
async fn get_node_type(
    _State(_state): State<ApiState>,
    Path(_node_type): Path<String>,
) -> Result<Json<SuccessResponse<NodeTypeInfo>>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement node type registry lookup
    Err((StatusCode::NOT_IMPLEMENTED, Json(ErrorResponse {
        error: "Node type lookup not implemented yet".to_string(),
    })))
}

/// Get system metrics
async fn get_metrics(
    State(state): State<ApiState>,
) -> Result<Json<SuccessResponse<WorkflowMetrics>>, (StatusCode, Json<ErrorResponse>)> {
    let metrics = state.workflow_engine.get_metrics().clone();
    
    Ok(Json(SuccessResponse {
        data: metrics,
    }))
}

/// Health check endpoint
async fn health_check() -> Json<SuccessResponse<HealthStatus>> {
    Json(SuccessResponse {
        data: HealthStatus {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    })
}

/// WebSocket handler for real-time updates
async fn websocket_handler() -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement WebSocket support for real-time workflow updates
    warn!("WebSocket handler not implemented yet");
    Err((StatusCode::NOT_IMPLEMENTED, Json(ErrorResponse {
        error: "WebSocket support not implemented yet".to_string(),
    })))
}

/// Node type information
#[derive(Serialize)]
pub struct NodeTypeInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub inputs: Vec<NodePort>,
    pub outputs: Vec<NodePort>,
    pub parameters: Vec<NodeParameter>,
}

/// Node port definition
#[derive(Serialize)]
pub struct NodePort {
    pub name: String,
    pub data_type: String,
    pub required: bool,
}

/// Node parameter definition
#[derive(Serialize)]
pub struct NodeParameter {
    pub name: String,
    pub display_name: String,
    pub data_type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: String,
}

/// Health status response
#[derive(Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
}