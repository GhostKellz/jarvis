use crate::types::{AgentTask, Conversation, Message, MessageMetadata, MessageRole};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite, SqlitePool};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct MemoryStore {
    pool: Pool<Sqlite>,
    context_manager: ContextManager,
    embedding_cache: EmbeddingCache,
    session_state: SessionState,
}

/// Enhanced context management for cross-session awareness
#[derive(Clone, Debug)]
pub struct ContextManager {
    pub active_contexts: HashMap<String, ContextEntry>,
    pub global_context: GlobalContext,
    pub project_contexts: HashMap<String, ProjectContext>,
}

/// Global context that persists across all sessions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlobalContext {
    pub user_preferences: UserPreferences,
    pub system_state: SystemState,
    pub learning_data: LearningData,
    pub last_updated: DateTime<Utc>,
}

/// User preferences and personalization data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserPreferences {
    pub coding_style: String,
    pub preferred_languages: Vec<String>,
    pub project_structure_preferences: HashMap<String, String>,
    pub ai_interaction_style: String,
    pub shortcuts: HashMap<String, String>,
}

/// Current system state and environment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemState {
    pub current_directory: String,
    pub active_projects: Vec<String>,
    pub installed_tools: Vec<String>,
    pub environment_variables: HashMap<String, String>,
    pub git_repositories: Vec<GitRepoInfo>,
}

/// Learning data from past interactions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearningData {
    pub frequent_commands: HashMap<String, u32>,
    pub successful_patterns: Vec<String>,
    pub error_patterns: Vec<String>,
    pub context_associations: HashMap<String, Vec<String>>,
}

/// Project-specific context
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project_id: String,
    pub project_type: String,
    pub dependencies: Vec<String>,
    pub recent_files: Vec<String>,
    pub build_commands: Vec<String>,
    pub test_commands: Vec<String>,
    pub documentation_links: Vec<String>,
}

/// Git repository information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitRepoInfo {
    pub path: String,
    pub remote_url: Option<String>,
    pub current_branch: String,
    pub status: String,
}

/// Individual context entry with metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: String,
    pub context_type: ContextType,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub relevance_score: f64,
    pub created_at: DateTime<Utc>,
    pub accessed_count: u32,
    pub last_accessed: DateTime<Utc>,
}

/// Types of context entries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ContextType {
    Conversation,
    CodeSnippet,
    Command,
    Error,
    Solution,
    Documentation,
    ProjectInfo,
    UserPattern,
}

/// Embedding cache for semantic search
#[derive(Clone, Debug)]
pub struct EmbeddingCache {
    pub embeddings: HashMap<String, Vec<f32>>,
    pub index: SemanticIndex,
}

/// Semantic search index
#[derive(Clone, Debug)]
pub struct SemanticIndex {
    pub entries: Vec<IndexEntry>,
    pub dimension: usize,
}

/// Index entry for semantic search
#[derive(Clone, Debug)]
pub struct IndexEntry {
    pub id: String,
    pub embedding: Vec<f32>,
    pub content_hash: String,
}

/// Session state management
#[derive(Clone, Debug)]
pub struct SessionState {
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub current_context: Vec<String>,
    pub recent_interactions: Vec<Interaction>,
    pub focus_areas: Vec<String>,
}

/// Individual interaction record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interaction {
    pub timestamp: DateTime<Utc>,
    pub interaction_type: InteractionType,
    pub content: String,
    pub success: bool,
    pub execution_time: Option<u64>,
    pub context_tags: Vec<String>,
}

/// Types of interactions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InteractionType {
    Query,
    Command,
    FileEdit,
    GitOperation,
    BuildTask,
    TestExecution,
    DeploymentTask,
}

impl MemoryStore {
    pub async fn new(database_path: &str) -> Result<Self> {
        let expanded_path = shellexpand::tilde(database_path);
        tracing::debug!("Database path: {} -> {}", database_path, expanded_path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(&*expanded_path).parent() {
            tracing::debug!("Creating parent directory: {:?}", parent);
            tokio::fs::create_dir_all(parent).await?;
        }

        // Ensure the database file exists by touching it
        if !std::path::Path::new(&*expanded_path).exists() {
            tracing::debug!("Creating database file: {}", expanded_path);
            tokio::fs::write(&*expanded_path, "").await?;
        }

        let db_url = format!("sqlite:{}", expanded_path);
        tracing::debug!("Database URL: {}", db_url);
        let pool = SqlitePool::connect(&db_url).await?;

        // Initialize the database schema manually for now
        // TODO: Implement proper migrations
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations (id)
            );
            
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                task_type TEXT NOT NULL,
                description TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                result TEXT
            );
            
            CREATE TABLE IF NOT EXISTS documents (
                key TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            
            CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages (conversation_id);
            CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages (created_at);
            CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks (created_at);
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks (status);
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { 
            pool,
            context_manager: ContextManager::new(),
            embedding_cache: EmbeddingCache::new(),
            session_state: SessionState::new(),
        })
    }

    pub async fn create_conversation(&self, title: &str) -> Result<Conversation> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO conversations (id, title, created_at, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(title)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(Conversation {
            id: id.to_string(), // Convert to String
            title: title.to_string(),
            created_at: now,
            updated_at: now,
            messages: vec![],
        })
    }

    pub async fn add_message(
        &self,
        conversation_id: Uuid,
        role: MessageRole,
        content: &str,
        metadata: MessageMetadata,
    ) -> Result<Message> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let role_str = match role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
            MessageRole::Tool => "tool",
        };
        let metadata_json = serde_json::to_string(&metadata)?;

        sqlx::query(
            "INSERT INTO messages (id, conversation_id, role, content, metadata, created_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(conversation_id.to_string())
        .bind(role_str)
        .bind(content)
        .bind(metadata_json)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(Message {
            id: id.to_string(),                           // Convert to String
            conversation_id: conversation_id.to_string(), // Convert to String
            role,
            content: content.to_string(),
            metadata,
            created_at: now,
        })
    }

    pub async fn get_conversation(&self, conversation_id: Uuid) -> Result<Option<Conversation>> {
        let conv_row = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, title, created_at, updated_at FROM conversations WHERE id = ?",
        )
        .bind(conversation_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = conv_row {
            let messages = self.get_conversation_messages(conversation_id).await?;

            Ok(Some(Conversation {
                id: row.0, // Use String directly
                title: row.1,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.2)?
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.3)?
                    .with_timezone(&chrono::Utc),
                messages,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_conversation_messages(&self, conversation_id: Uuid) -> Result<Vec<Message>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, role, content, metadata, created_at FROM messages WHERE conversation_id = ? ORDER BY created_at ASC"
        )
        .bind(conversation_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut messages = Vec::new();
        for row in rows {
            let role = match row.1.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                "system" => MessageRole::System,
                "tool" => MessageRole::Tool,
                _ => MessageRole::User,
            };
            let metadata: MessageMetadata = serde_json::from_str(&row.3)?;

            messages.push(Message {
                id: row.0,                                    // Use String directly
                conversation_id: conversation_id.to_string(), // Convert to String
                role,
                content: row.2,
                metadata,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.4)?
                    .with_timezone(&chrono::Utc),
            });
        }

        Ok(messages)
    }

    pub async fn store_task(&self, task: &AgentTask) -> Result<()> {
        let task_type_str = format!("{:?}", task.task_type);
        let status_str = format!("{:?}", task.status);
        let result_json = task
            .result
            .as_ref()
            .map(|r| serde_json::to_string(r))
            .transpose()?;

        sqlx::query(
            "INSERT OR REPLACE INTO tasks (id, task_type, description, status, created_at, completed_at, result) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(task.id.to_string())
        .bind(task_type_str)
        .bind(&task.description)
        .bind(status_str)
        .bind(task.created_at.to_rfc3339())
        .bind(task.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(result_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_recent_tasks(&self, limit: i32) -> Result<Vec<AgentTask>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>, Option<String>)>(
            "SELECT id, task_type, description, status, created_at, completed_at, result FROM tasks ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut tasks = Vec::new();
        for row in rows {
            // This is a simplified version - you'd want proper enum parsing
            tasks.push(AgentTask {
                id: row.0,                                  // Use String directly
                task_type: crate::types::TaskType::Explain, // TODO: Parse properly
                description: row.2,
                status: crate::types::TaskStatus::Completed, // TODO: Parse properly
                created_at: chrono::DateTime::parse_from_rfc3339(&row.4)?
                    .with_timezone(&chrono::Utc),
                completed_at: row
                    .5
                    .map(|dt_str| chrono::DateTime::parse_from_rfc3339(&dt_str))
                    .transpose()?
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                result: row.6.map(|r| serde_json::from_str(&r)).transpose()?,
            });
        }

        Ok(tasks)
    }

    /// Store a generic document with a key-value pair
    pub async fn store_document(&self, key: &str, data: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO documents (key, data, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?3)
            "#,
        )
        .bind(key)
        .bind(data)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Retrieve a document by key
    pub async fn get_document(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT data FROM documents WHERE key = ?1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get::<String, _>(0)))
    }

    /// Delete a document by key
    pub async fn delete_document(&self, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM documents WHERE key = ?1")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Enhanced context-aware memory operations
    
    /// Store context entry with automatic relevance scoring
    pub async fn store_context(&mut self, content: &str, context_type: ContextType, metadata: HashMap<String, String>) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let context_entry = ContextEntry {
            id: id.clone(),
            context_type: context_type.clone(),
            content: content.to_string(),
            metadata: metadata.clone(),
            relevance_score: self.calculate_relevance_score(content, &context_type).await,
            created_at: now,
            accessed_count: 0,
            last_accessed: now,
        };

        // Store in database
        let context_type_str = format!("{:?}", context_type);
        let metadata_json = serde_json::to_string(&metadata)?;
        
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO context_entries 
            (id, context_type, content, metadata, relevance_score, created_at, accessed_count, last_accessed)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#
        )
        .bind(&id)
        .bind(&context_type_str)
        .bind(content)
        .bind(&metadata_json)
        .bind(context_entry.relevance_score)
        .bind(now.to_rfc3339())
        .bind(0i32)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        // Add to in-memory cache
        self.context_manager.active_contexts.insert(id.clone(), context_entry);
        
        // Update semantic index
        self.update_semantic_index(&id, content).await?;
        
        Ok(id)
    }

    /// Retrieve context with semantic search
    pub async fn search_context(&mut self, query: &str, limit: usize) -> Result<Vec<ContextEntry>> {
        // First try semantic search
        let semantic_results = self.semantic_search(query, limit).await?;
        
        if !semantic_results.is_empty() {
            return Ok(semantic_results);
        }
        
        // Fallback to text search
        let rows = sqlx::query_as::<_, (String, String, String, String, f64, String, i32, String)>(
            r#"
            SELECT id, context_type, content, metadata, relevance_score, created_at, accessed_count, last_accessed
            FROM context_entries 
            WHERE content LIKE ?1 
            ORDER BY relevance_score DESC, last_accessed DESC 
            LIMIT ?2
            "#
        )
        .bind(format!("%{}%", query))
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            let context_type = match row.1.as_str() {
                "Conversation" => ContextType::Conversation,
                "CodeSnippet" => ContextType::CodeSnippet,
                "Command" => ContextType::Command,
                "Error" => ContextType::Error,
                "Solution" => ContextType::Solution,
                "Documentation" => ContextType::Documentation,
                "ProjectInfo" => ContextType::ProjectInfo,
                "UserPattern" => ContextType::UserPattern,
                _ => ContextType::Conversation,
            };
            
            let metadata: HashMap<String, String> = serde_json::from_str(&row.3)?;
            
            let entry = ContextEntry {
                id: row.0,
                context_type,
                content: row.2,
                metadata,
                relevance_score: row.4,
                created_at: DateTime::parse_from_rfc3339(&row.5)?.with_timezone(&Utc),
                accessed_count: row.6 as u32,
                last_accessed: DateTime::parse_from_rfc3339(&row.7)?.with_timezone(&Utc),
            };
            
            results.push(entry);
        }
        
        Ok(results)
    }

    /// Update global context with learning
    pub async fn update_global_context(&mut self, interaction: Interaction) -> Result<()> {
        // Update learning data
        match &interaction.interaction_type {
            InteractionType::Command => {
                *self.context_manager.global_context.learning_data
                    .frequent_commands
                    .entry(interaction.content.clone())
                    .or_insert(0) += 1;
            },
            _ => {}
        }
        
        if interaction.success {
            self.context_manager.global_context.learning_data
                .successful_patterns
                .push(interaction.content.clone());
        } else {
            self.context_manager.global_context.learning_data
                .error_patterns
                .push(interaction.content.clone());
        }
        
        // Store updated global context
        let global_context_json = serde_json::to_string(&self.context_manager.global_context)?;
        self.store_document("global_context", &global_context_json).await?;
        
        Ok(())
    }

    /// Get contextual suggestions based on current session
    pub async fn get_contextual_suggestions(&self, current_input: &str) -> Result<Vec<String>> {
        let mut suggestions = Vec::new();
        
        // Analyze current input for patterns
        if current_input.starts_with("git") {
            suggestions.extend(self.get_git_suggestions().await?);
        } else if current_input.contains("cargo") || current_input.contains("rust") {
            suggestions.extend(self.get_rust_suggestions().await?);
        } else if current_input.contains("docker") {
            suggestions.extend(self.get_docker_suggestions().await?);
        }
        
        // Add frequent commands
        for (command, count) in &self.context_manager.global_context.learning_data.frequent_commands {
            if command.contains(current_input) && *count > 5 {
                suggestions.push(command.clone());
            }
        }
        
        Ok(suggestions)
    }

    /// Initialize enhanced database schema
    pub async fn initialize_enhanced_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS context_entries (
                id TEXT PRIMARY KEY,
                context_type TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT NOT NULL,
                relevance_score REAL NOT NULL,
                created_at TEXT NOT NULL,
                accessed_count INTEGER NOT NULL DEFAULT 0,
                last_accessed TEXT NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS semantic_embeddings (
                content_id TEXT PRIMARY KEY,
                embedding BLOB NOT NULL,
                content_hash TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS interaction_history (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                interaction_type TEXT NOT NULL,
                content TEXT NOT NULL,
                success BOOLEAN NOT NULL,
                execution_time INTEGER,
                context_tags TEXT NOT NULL
            );
            
            CREATE INDEX IF NOT EXISTS idx_context_type ON context_entries (context_type);
            CREATE INDEX IF NOT EXISTS idx_context_relevance ON context_entries (relevance_score DESC);
            CREATE INDEX IF NOT EXISTS idx_context_accessed ON context_entries (last_accessed DESC);
            CREATE INDEX IF NOT EXISTS idx_interactions_session ON interaction_history (session_id);
            CREATE INDEX IF NOT EXISTS idx_interactions_timestamp ON interaction_history (timestamp DESC);
            "#
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    async fn calculate_relevance_score(&self, content: &str, context_type: &ContextType) -> f64 {
        let mut score: f64 = 0.5; // Base score
        
        // Boost score based on content characteristics
        if content.len() > 100 {
            score += 0.1;
        }
        
        // Boost score based on context type
        match context_type {
            ContextType::Solution => score += 0.3,
            ContextType::Error => score += 0.2,
            ContextType::CodeSnippet => score += 0.2,
            ContextType::Command => score += 0.1,
            _ => {}
        }
        
        // Check for code patterns
        if content.contains("fn ") || content.contains("impl ") || content.contains("struct ") {
            score += 0.2;
        }
        
        score.min(1.0)
    }

    async fn update_semantic_index(&mut self, id: &str, content: &str) -> Result<()> {
        // Simplified semantic indexing - in production would use actual embeddings
        let embedding = self.generate_simple_embedding(content);
        
        let index_entry = IndexEntry {
            id: id.to_string(),
            embedding: embedding.clone(),
            content_hash: format!("{:x}", md5::compute(content.as_bytes())),
        };
        
        self.embedding_cache.index.entries.push(index_entry);
        self.embedding_cache.embeddings.insert(id.to_string(), embedding);
        
        Ok(())
    }

    async fn semantic_search(&self, query: &str, limit: usize) -> Result<Vec<ContextEntry>> {
        // Simplified semantic search - in production would use proper vector similarity
        let query_embedding = self.generate_simple_embedding(query);
        
        let mut scored_entries = Vec::new();
        for entry in &self.embedding_cache.index.entries {
            if let Some(embedding) = self.embedding_cache.embeddings.get(&entry.id) {
                let similarity = self.cosine_similarity(&query_embedding, embedding);
                if similarity > 0.3 { // Threshold
                    scored_entries.push((entry.id.clone(), similarity));
                }
            }
        }
        
        scored_entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored_entries.truncate(limit);
        
        let mut results = Vec::new();
        for (id, _score) in scored_entries {
            if let Some(context) = self.context_manager.active_contexts.get(&id) {
                results.push(context.clone());
            }
        }
        
        Ok(results)
    }

    fn generate_simple_embedding(&self, text: &str) -> Vec<f32> {
        // Simplified embedding generation - in production would use a proper model
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut embedding = vec![0.0; 384]; // Standard embedding dimension
        
        for (i, word) in words.iter().enumerate().take(384) {
            let hash = word.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
            embedding[i % 384] = (hash % 1000) as f32 / 1000.0;
        }
        
        embedding
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    async fn get_git_suggestions(&self) -> Result<Vec<String>> {
        Ok(vec![
            "git status".to_string(),
            "git add .".to_string(),
            "git commit -m \"\"".to_string(),
            "git push".to_string(),
            "git pull".to_string(),
            "git checkout -b ".to_string(),
        ])
    }

    async fn get_rust_suggestions(&self) -> Result<Vec<String>> {
        Ok(vec![
            "cargo build".to_string(),
            "cargo test".to_string(),
            "cargo run".to_string(),
            "cargo check".to_string(),
            "cargo fmt".to_string(),
            "cargo clippy".to_string(),
        ])
    }

    async fn get_docker_suggestions(&self) -> Result<Vec<String>> {
        Ok(vec![
            "docker build .".to_string(),
            "docker run".to_string(),
            "docker ps".to_string(),
            "docker logs".to_string(),
            "docker stop".to_string(),
            "docker system prune".to_string(),
        ])
    }
}

// Implementation for new structs

impl ContextManager {
    pub fn new() -> Self {
        Self {
            active_contexts: HashMap::new(),
            global_context: GlobalContext::default(),
            project_contexts: HashMap::new(),
        }
    }
}

impl Default for GlobalContext {
    fn default() -> Self {
        Self {
            user_preferences: UserPreferences::default(),
            system_state: SystemState::default(),
            learning_data: LearningData::default(),
            last_updated: Utc::now(),
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            coding_style: "conventional".to_string(),
            preferred_languages: vec!["rust".to_string(), "javascript".to_string()],
            project_structure_preferences: HashMap::new(),
            ai_interaction_style: "helpful".to_string(),
            shortcuts: HashMap::new(),
        }
    }
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            current_directory: "/".to_string(),
            active_projects: Vec::new(),
            installed_tools: Vec::new(),
            environment_variables: HashMap::new(),
            git_repositories: Vec::new(),
        }
    }
}

impl Default for LearningData {
    fn default() -> Self {
        Self {
            frequent_commands: HashMap::new(),
            successful_patterns: Vec::new(),
            error_patterns: Vec::new(),
            context_associations: HashMap::new(),
        }
    }
}

impl EmbeddingCache {
    pub fn new() -> Self {
        Self {
            embeddings: HashMap::new(),
            index: SemanticIndex::new(),
        }
    }
}

impl SemanticIndex {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            dimension: 384,
        }
    }
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            start_time: Utc::now(),
            current_context: Vec::new(),
            recent_interactions: Vec::new(),
            focus_areas: Vec::new(),
        }
    }
}
