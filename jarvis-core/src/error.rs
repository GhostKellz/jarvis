use std::fmt;

/// Custom error types for Jarvis
#[derive(Debug, Clone)]
pub enum JarvisError {
    /// Configuration related errors
    Config(String),
    /// LLM provider errors
    LLM(String),
    /// Database/Memory errors
    Database(String),
    /// System integration errors
    System(String),
    /// Plugin errors
    Plugin(String),
    /// Network/API errors
    Network(String),
    /// General internal errors
    Internal(String),
}

impl fmt::Display for JarvisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JarvisError::Config(msg) => write!(f, "Configuration error: {}", msg),
            JarvisError::LLM(msg) => write!(f, "LLM error: {}", msg),
            JarvisError::Database(msg) => write!(f, "Database error: {}", msg),
            JarvisError::System(msg) => write!(f, "System error: {}", msg),
            JarvisError::Plugin(msg) => write!(f, "Plugin error: {}", msg),
            JarvisError::Network(msg) => write!(f, "Network error: {}", msg),
            JarvisError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for JarvisError {}

impl From<anyhow::Error> for JarvisError {
    fn from(err: anyhow::Error) -> Self {
        JarvisError::Internal(err.to_string())
    }
}

impl From<sqlx::Error> for JarvisError {
    fn from(err: sqlx::Error) -> Self {
        JarvisError::Database(err.to_string())
    }
}

impl From<reqwest::Error> for JarvisError {
    fn from(err: reqwest::Error) -> Self {
        JarvisError::Network(err.to_string())
    }
}

impl From<serde_json::Error> for JarvisError {
    fn from(err: serde_json::Error) -> Self {
        JarvisError::Internal(format!("JSON parsing error: {}", err))
    }
}

impl From<toml::de::Error> for JarvisError {
    fn from(err: toml::de::Error) -> Self {
        JarvisError::Config(format!("TOML parsing error: {}", err))
    }
}

impl From<std::io::Error> for JarvisError {
    fn from(err: std::io::Error) -> Self {
        JarvisError::System(format!("IO error: {}", err))
    }
}

/// Result type alias for Jarvis operations
pub type JarvisResult<T> = Result<T, JarvisError>;

/// Error context helper for better error reporting
pub trait ErrorContext<T> {
    fn with_context(self, context: &str) -> JarvisResult<T>;
    fn with_config_context(self, context: &str) -> JarvisResult<T>;
    fn with_llm_context(self, context: &str) -> JarvisResult<T>;
    fn with_database_context(self, context: &str) -> JarvisResult<T>;
    fn with_system_context(self, context: &str) -> JarvisResult<T>;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_context(self, context: &str) -> JarvisResult<T> {
        self.map_err(|e| JarvisError::Internal(format!("{}: {}", context, e)))
    }

    fn with_config_context(self, context: &str) -> JarvisResult<T> {
        self.map_err(|e| JarvisError::Config(format!("{}: {}", context, e)))
    }

    fn with_llm_context(self, context: &str) -> JarvisResult<T> {
        self.map_err(|e| JarvisError::LLM(format!("{}: {}", context, e)))
    }

    fn with_database_context(self, context: &str) -> JarvisResult<T> {
        self.map_err(|e| JarvisError::Database(format!("{}: {}", context, e)))
    }

    fn with_system_context(self, context: &str) -> JarvisResult<T> {
        self.map_err(|e| JarvisError::System(format!("{}: {}", context, e)))
    }
}

/// Macro for easy error creation
#[macro_export]
macro_rules! jarvis_error {
    (Config, $msg:expr) => {
        JarvisError::Config($msg.to_string())
    };
    (LLM, $msg:expr) => {
        JarvisError::LLM($msg.to_string())
    };
    (Database, $msg:expr) => {
        JarvisError::Database($msg.to_string())
    };
    (System, $msg:expr) => {
        JarvisError::System($msg.to_string())
    };
    (Plugin, $msg:expr) => {
        JarvisError::Plugin($msg.to_string())
    };
    (Network, $msg:expr) => {
        JarvisError::Network($msg.to_string())
    };
    (Internal, $msg:expr) => {
        JarvisError::Internal($msg.to_string())
    };
}

/// Macro for easy error creation with format
#[macro_export]
macro_rules! jarvis_error_fmt {
    (Config, $fmt:expr $(, $args:expr)*) => {
        JarvisError::Config(format!($fmt $(, $args)*))
    };
    (LLM, $fmt:expr $(, $args:expr)*) => {
        JarvisError::LLM(format!($fmt $(, $args)*))
    };
    (Database, $fmt:expr $(, $args:expr)*) => {
        JarvisError::Database(format!($fmt $(, $args)*))
    };
    (System, $fmt:expr $(, $args:expr)*) => {
        JarvisError::System(format!($fmt $(, $args)*))
    };
    (Plugin, $fmt:expr $(, $args:expr)*) => {
        JarvisError::Plugin(format!($fmt $(, $args)*))
    };
    (Network, $fmt:expr $(, $args:expr)*) => {
        JarvisError::Network(format!($fmt $(, $args)*))
    };
    (Internal, $fmt:expr $(, $args:expr)*) => {
        JarvisError::Internal(format!($fmt $(, $args)*))
    };
}
