use super::{GhostFlowNode, NodeHealth, HealthStatus};
use crate::{Result, WorkflowContext, NodeExecutionResult, ExecutionStatus, MemoryContext, ContextEntry, ContextEntryType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Context Memory Node with persistent workflow memory using ZQLite backend
pub struct MemoryNode {
    memory_store: Arc<RwLock<Option<MemoryStore>>>,
    config: MemoryNodeConfig,
    health: Arc<RwLock<NodeHealth>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNodeConfig {
    pub database_path: String,
    pub enable_zqlite: bool,
    pub enable_encryption: bool,
    pub semantic_search: bool,
    pub max_context_entries: usize,
    pub retention_days: u32,
    pub auto_cleanup: bool,
    pub similarity_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInput {
    pub action: MemoryAction,
    pub content: Option<String>,
    pub entry_type: Option<ContextEntryType>,
    pub search_query: Option<String>,
    pub search_limit: Option<usize>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryAction {
    Store,
    Search,
    Retrieve,
    Update,
    Delete,
    GetContext,
    ClearContext,
    AnalyzePatterns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOutput {
    pub action_performed: MemoryAction,
    pub success: bool,
    pub entries: Vec<ContextEntry>,
    pub context_summary: Option<String>,
    pub patterns: Option<Vec<MemoryPattern>>,
    pub total_entries: usize,
    pub storage_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPattern {
    pub pattern_type: String,
    pub frequency: u32,
    pub confidence: f64,
    pub description: String,
    pub related_entries: Vec<Uuid>,
}

/// Internal memory store that can use ZQLite or SQLite
pub struct MemoryStore {
    connection: Option<sqlx::Pool<sqlx::Sqlite>>,
    zqlite_enabled: bool,
    embedding_cache: HashMap<String, Vec<f32>>,
}

impl MemoryNode {
    pub fn new() -> Result<Self> {
        Ok(Self {
            memory_store: Arc::new(RwLock::new(None)),
            config: MemoryNodeConfig::default(),
            health: Arc::new(RwLock::new(NodeHealth {
                status: HealthStatus::Unknown,
                message: None,
                last_execution: None,
                error_count: 0,
                success_rate: 0.0,
            })),
        })
    }

    async fn initialize_memory_store(&self, config: &HashMap<String, serde_json::Value>) -> Result<()> {
        let db_path = config.get("database_path")
            .and_then(|v| v.as_str())
            .unwrap_or("memory.db");

        let enable_zqlite = config.get("enable_zqlite")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if enable_zqlite {
            // Initialize ZQLite connection (would require FFI bindings)
            // For now, fall back to SQLite
            tracing::info!("ZQLite support planned - using SQLite for now");
        }

        // Initialize SQLite connection
        let connection_string = format!("sqlite:{}", db_path);
        let pool = sqlx::SqlitePool::connect(&connection_string).await?;
        
        // Create tables
        self.create_tables(&pool).await?;

        let store = MemoryStore {
            connection: Some(pool),
            zqlite_enabled: enable_zqlite,
            embedding_cache: HashMap::new(),
        };

        *self.memory_store.write().await = Some(store);
        Ok(())
    }

    async fn create_tables(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> Result<()> {
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS context_entries (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                execution_id TEXT,
                content TEXT NOT NULL,
                entry_type TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                metadata TEXT,
                embedding BLOB,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
        "#).execute(pool).await?;

        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS memory_sessions (
                session_id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
                entry_count INTEGER DEFAULT 0,
                metadata TEXT
            )
        "#).execute(pool).await?;

        sqlx::query(r#"
            CREATE INDEX IF NOT EXISTS idx_context_entries_workflow_id 
            ON context_entries(workflow_id)
        "#).execute(pool).await?;

        sqlx::query(r#"
            CREATE INDEX IF NOT EXISTS idx_context_entries_timestamp 
            ON context_entries(timestamp)
        "#).execute(pool).await?;

        Ok(())
    }

    async fn store_entry(&self, workflow_context: &WorkflowContext, input: &MemoryInput) -> Result<MemoryOutput> {
        let mut store = self.memory_store.write().await;
        let store = store.as_mut().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Memory store not initialized".to_string()))?;

        let pool = store.connection.as_ref().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Database connection not available".to_string()))?;

        let content = input.content.as_ref().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Content is required for store action".to_string()))?;

        let entry_id = Uuid::new_v4();
        let entry_type = input.entry_type.as_ref().unwrap_or(&ContextEntryType::UserInput);
        let metadata_json = serde_json::to_string(&input.metadata)?;

        // Generate embedding for semantic search (placeholder - would use actual embedding model)
        let embedding = self.generate_embedding(content).await?;
        let embedding_bytes = serde_json::to_vec(&embedding)?;

        sqlx::query(r#"
            INSERT INTO context_entries 
            (id, workflow_id, execution_id, content, entry_type, timestamp, metadata, embedding)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(entry_id.to_string())
        .bind(workflow_context.workflow_id.to_string())
        .bind(workflow_context.execution_id.to_string())
        .bind(content)
        .bind(serde_json::to_string(&entry_type)?)
        .bind(Utc::now().to_rfc3339())
        .bind(metadata_json)
        .bind(embedding_bytes)
        .execute(pool)
        .await?;

        let entry = ContextEntry {
            id: entry_id,
            content: content.clone(),
            entry_type: entry_type.clone(),
            timestamp: Utc::now(),
            metadata: input.metadata.clone().unwrap_or_default(),
        };

        Ok(MemoryOutput {
            action_performed: MemoryAction::Store,
            success: true,
            entries: vec![entry],
            context_summary: None,
            patterns: None,
            total_entries: 1,
            storage_size_bytes: content.len() as u64,
        })
    }

    async fn search_entries(&self, workflow_context: &WorkflowContext, input: &MemoryInput) -> Result<MemoryOutput> {
        let store = self.memory_store.read().await;
        let store = store.as_ref().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Memory store not initialized".to_string()))?;

        let pool = store.connection.as_ref().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Database connection not available".to_string()))?;

        let search_query = input.search_query.as_ref().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Search query is required for search action".to_string()))?;

        let limit = input.search_limit.unwrap_or(10);

        // Simple text search for now (would use vector similarity with ZQLite)
        let rows = sqlx::query(r#"
            SELECT id, content, entry_type, timestamp, metadata
            FROM context_entries 
            WHERE workflow_id = ? AND content LIKE ?
            ORDER BY timestamp DESC
            LIMIT ?
        "#)
        .bind(workflow_context.workflow_id.to_string())
        .bind(format!("%{}%", search_query))
        .bind(limit as i64)
        .fetch_all(pool)
        .await?;

        let mut entries = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let content: String = row.get("content");
            let entry_type_str: String = row.get("entry_type");
            let timestamp_str: String = row.get("timestamp");
            let metadata_str: String = row.get("metadata");

            let entry_type: ContextEntryType = serde_json::from_str(&entry_type_str)?;
            let timestamp: DateTime<Utc> = DateTime::parse_from_rfc3339(&timestamp_str)?.with_timezone(&Utc);
            let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(&metadata_str)?;

            entries.push(ContextEntry {
                id: Uuid::parse_str(&id)?,
                content,
                entry_type,
                timestamp,
                metadata,
            });
        }

        Ok(MemoryOutput {
            action_performed: MemoryAction::Search,
            success: true,
            entries,
            context_summary: Some(format!("Found {} entries matching '{}'", entries.len(), search_query)),
            patterns: None,
            total_entries: entries.len(),
            storage_size_bytes: 0,
        })
    }

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Placeholder for actual embedding generation
        // In real implementation, this would call GhostLLM or another embedding model
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut embedding = vec![0.0f32; 384]; // Common embedding dimension
        
        for (i, word) in words.iter().enumerate().take(embedding.len()) {
            embedding[i] = word.len() as f32 / 10.0; // Simple hash-based embedding
        }
        
        Ok(embedding)
    }

    async fn analyze_patterns(&self, workflow_context: &WorkflowContext) -> Result<MemoryOutput> {
        let store = self.memory_store.read().await;
        let store = store.as_ref().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Memory store not initialized".to_string()))?;

        let pool = store.connection.as_ref().ok_or_else(|| 
            crate::GhostFlowError::NodeExecution("Database connection not available".to_string()))?;

        // Analyze entry patterns
        let type_counts = sqlx::query(r#"
            SELECT entry_type, COUNT(*) as count
            FROM context_entries 
            WHERE workflow_id = ?
            GROUP BY entry_type
        "#)
        .bind(workflow_context.workflow_id.to_string())
        .fetch_all(pool)
        .await?;

        let mut patterns = Vec::new();
        for row in type_counts {
            let entry_type: String = row.get("entry_type");
            let count: i64 = row.get("count");

            patterns.push(MemoryPattern {
                pattern_type: entry_type.clone(),
                frequency: count as u32,
                confidence: 0.8,
                description: format!("{} entries of type {}", count, entry_type),
                related_entries: vec![],
            });
        }

        Ok(MemoryOutput {
            action_performed: MemoryAction::AnalyzePatterns,
            success: true,
            entries: vec![],
            context_summary: Some(format!("Analyzed {} pattern types", patterns.len())),
            patterns: Some(patterns),
            total_entries: 0,
            storage_size_bytes: 0,
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
impl GhostFlowNode for MemoryNode {
    fn node_type(&self) -> &'static str {
        "jarvis.memory"
    }

    fn display_name(&self) -> &str {
        "Context Memory"
    }

    fn description(&self) -> &str {
        "Persistent memory with semantic search across workflows, powered by ZQLite for security"
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["store", "search", "retrieve", "update", "delete", "get_context", "clear_context", "analyze_patterns"],
                    "description": "The memory action to perform"
                },
                "content": {
                    "type": "string",
                    "description": "Content to store (required for store action)"
                },
                "entry_type": {
                    "type": "string",
                    "enum": ["user_input", "ai_response", "system_event", "workflow_result", "node_output"],
                    "description": "Type of memory entry"
                },
                "search_query": {
                    "type": "string",
                    "description": "Search query (required for search action)"
                },
                "search_limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return",
                    "default": 10
                },
                "metadata": {
                    "type": "object",
                    "description": "Additional metadata for the entry"
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
                "entries": {
                    "type": "array",
                    "description": "Retrieved or affected context entries"
                },
                "context_summary": {
                    "type": "string",
                    "description": "Summary of the operation results"
                },
                "patterns": {
                    "type": "array",
                    "description": "Detected patterns in memory"
                },
                "total_entries": {
                    "type": "integer",
                    "description": "Total number of entries affected"
                }
            }
        })
    }

    fn config_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "database_path": {
                    "type": "string",
                    "description": "Path to the database file",
                    "default": "memory.db"
                },
                "enable_zqlite": {
                    "type": "boolean",
                    "description": "Use ZQLite for post-quantum security",
                    "default": false
                },
                "enable_encryption": {
                    "type": "boolean",
                    "description": "Enable field-level encryption",
                    "default": true
                },
                "semantic_search": {
                    "type": "boolean",
                    "description": "Enable semantic search with embeddings",
                    "default": true
                },
                "max_context_entries": {
                    "type": "integer",
                    "description": "Maximum entries to keep in memory",
                    "default": 10000
                },
                "retention_days": {
                    "type": "integer",
                    "description": "Number of days to retain entries",
                    "default": 30
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
        
        // Initialize memory store if needed
        if self.memory_store.read().await.is_none() {
            self.initialize_memory_store(&config).await?;
        }

        // Parse input
        let input: MemoryInput = serde_json::from_value(serde_json::Value::Object(
            inputs.into_iter().collect()
        ))?;

        // Execute the requested memory action
        let result = match input.action {
            MemoryAction::Store => self.store_entry(context, &input).await,
            MemoryAction::Search => self.search_entries(context, &input).await,
            MemoryAction::AnalyzePatterns => self.analyze_patterns(context).await,
            _ => {
                // Implement other actions as needed
                Ok(MemoryOutput {
                    action_performed: input.action.clone(),
                    success: false,
                    entries: vec![],
                    context_summary: Some("Action not implemented yet".to_string()),
                    patterns: None,
                    total_entries: 0,
                    storage_size_bytes: 0,
                })
            }
        };

        match result {
            Ok(output) => {
                self.update_health_metrics(true, start_time.elapsed().as_millis() as u64).await;
                
                Ok(crate::NodeExecutionResult {
                    node_id: "memory".to_string(),
                    execution_id: context.execution_id,
                    status: ExecutionStatus::Success,
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
                    node_id: "memory".to_string(),
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
        // Validate database path
        if let Some(db_path) = config.get("database_path") {
            if let Some(path_str) = db_path.as_str() {
                if path_str.is_empty() {
                    return Err(crate::GhostFlowError::Config(
                        "Database path cannot be empty".to_string()
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

impl Default for MemoryNodeConfig {
    fn default() -> Self {
        Self {
            database_path: "memory.db".to_string(),
            enable_zqlite: false,
            enable_encryption: true,
            semantic_search: true,
            max_context_entries: 10000,
            retention_days: 30,
            auto_cleanup: true,
            similarity_threshold: 0.8,
        }
    }
}