use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    pub metadata: MessageMetadata,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub tokens: Option<u32>,
    pub model: Option<String>,
    pub cost: Option<f64>,
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self {
            tokens: None,
            model: None,
            cost: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "User"),
            MessageRole::Assistant => write!(f, "Assistant"),
            MessageRole::System => write!(f, "System"),
            MessageRole::Tool => write!(f, "Tool"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub task_type: TaskType,
    pub description: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Explain,
    Diagnose,
    Write,
    Check,
    Fix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Created,
    Running,
    Completed,
    Failed,
}

// Simple Environment stub
#[derive(Debug, Clone)]
pub struct Environment {
    pub os_type: String,
    pub hostname: String,
    pub working_directory: String,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            os_type: "linux".to_string(),
            hostname: "localhost".to_string(), 
            working_directory: "/tmp".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitContext {
    pub repo_path: String,
    pub current_branch: String,
    pub dirty: bool,
    pub last_commit: String,
}

impl Default for GitContext {
    fn default() -> Self {
        Self {
            repo_path: String::new(),
            current_branch: String::new(),
            dirty: false,
            last_commit: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os: String,
    pub kernel: String,
    pub hostname: String,
    pub arch: String,
    pub uptime: u64,
    pub load_avg: (f64, f64, f64),
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            os: "linux".to_string(),
            kernel: "unknown".to_string(),
            hostname: "localhost".to_string(),
            arch: "x86_64".to_string(),
            uptime: 0,
            load_avg: (0.0, 0.0, 0.0),
        }
    }
}
