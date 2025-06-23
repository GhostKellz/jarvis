use anyhow::Result;
use sqlx::{Pool, Sqlite, SqlitePool};
use uuid::Uuid;
use chrono::Utc;
use crate::types::{Conversation, Message, MessageRole, MessageMetadata, AgentTask};

#[derive(Clone)]
pub struct MemoryStore {
    pool: Pool<Sqlite>,
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
            
            CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages (conversation_id);
            CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages (created_at);
            CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks (created_at);
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks (status);
            "#
        )
        .execute(&pool)
        .await?;
        
        Ok(Self { pool })
    }

    pub async fn create_conversation(&self, title: &str) -> Result<Conversation> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        sqlx::query(
            "INSERT INTO conversations (id, title, created_at, updated_at) VALUES (?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(title)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;
        
        Ok(Conversation {
            id,
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
            id,
            conversation_id,
            role,
            content: content.to_string(),
            metadata,
            created_at: now,
        })
    }

    pub async fn get_conversation(&self, conversation_id: Uuid) -> Result<Option<Conversation>> {
        let conv_row = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, title, created_at, updated_at FROM conversations WHERE id = ?"
        )
        .bind(conversation_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(row) = conv_row {
            let messages = self.get_conversation_messages(conversation_id).await?;
            
            Ok(Some(Conversation {
                id: Uuid::parse_str(&row.0)?,
                title: row.1,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.2)?.with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.3)?.with_timezone(&chrono::Utc),
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
                id: Uuid::parse_str(&row.0)?,
                conversation_id,
                role,
                content: row.2,
                metadata,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.4)?.with_timezone(&chrono::Utc),
            });
        }
        
        Ok(messages)
    }

    pub async fn store_task(&self, task: &AgentTask) -> Result<()> {
        let task_type_str = format!("{:?}", task.task_type);
        let status_str = format!("{:?}", task.status);
        let result_json = task.result.as_ref().map(|r| serde_json::to_string(r)).transpose()?;
        
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
                id: Uuid::parse_str(&row.0)?,
                task_type: crate::types::TaskType::Explain, // TODO: Parse properly
                description: row.2,
                status: crate::types::TaskStatus::Completed, // TODO: Parse properly
                created_at: chrono::DateTime::parse_from_rfc3339(&row.4)?.with_timezone(&chrono::Utc),
                completed_at: row.5
                    .map(|dt_str| chrono::DateTime::parse_from_rfc3339(&dt_str))
                    .transpose()?
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                result: row.6.map(|r| serde_json::from_str(&r)).transpose()?,
            });
        }
        
        Ok(tasks)
    }
}
