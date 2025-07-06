/*!
 * AI/LLM Integration for JARVIS-NV
 *
 * Handles integration with AI models via Ollama, providing blockchain analytics,
 * diagnostic helpers, and autonomous decision making capabilities.
 */

use anyhow::{Context, Result};
use ollama_rs::{
    Ollama,
    generation::{
        chat::{ChatMessage, MessageRole, request::ChatMessageRequest},
        completion::{GenerationResponse, request::GenerationRequest},
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use url::Url;

use crate::config::AgentConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModel {
    pub name: String,
    pub size: String,
    pub status: String, // "available", "downloading", "loaded", "error"
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub digest: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
    pub family: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub model: String,
    pub prompt: String,
    pub context: HashMap<String, serde_json::Value>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub id: String,
    pub request_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub model: String,
    pub response_text: String,
    pub tokens_generated: u32,
    pub inference_time_ms: u64,
    pub tokens_per_second: f32,
    pub confidence_score: Option<f32>,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub context_tokens: u32,
    pub total_tokens: u32,
    pub system_prompt: Option<String>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
    pub total_tokens_generated: u64,
    pub avg_tokens_per_second: f32,
    pub active_sessions: u32,
    pub models_loaded: u32,
    pub memory_usage_mb: f64,
}

pub struct OllamaManager {
    config: AgentConfig,
    client: Ollama,

    // Model management
    available_models: Arc<RwLock<HashMap<String, AiModel>>>,
    loaded_models: Arc<RwLock<Vec<String>>>,

    // Session management
    chat_sessions: Arc<Mutex<HashMap<String, ChatSession>>>,

    // Request tracking
    pending_requests: Arc<Mutex<HashMap<String, InferenceRequest>>>,
    completed_requests: Arc<Mutex<Vec<InferenceResponse>>>,

    // Metrics
    metrics: Arc<RwLock<AiMetrics>>,

    // Runtime state
    is_running: Arc<RwLock<bool>>,
}

impl OllamaManager {
    /// Create new Ollama manager
    pub async fn new(config: &AgentConfig) -> Result<Self> {
        info!("🧠 Initializing Ollama AI Manager");

        let ollama_url = config
            .ollama_endpoint
            .clone()
            .unwrap_or_else(|| "http://localhost:11434".to_string());

        let parsed_url = Url::parse(&ollama_url)?;
        let host = parsed_url.host_str().unwrap_or("localhost").to_string();
        let port = parsed_url.port().unwrap_or(11434);

        let client = Ollama::new(host, port);

        let manager = Self {
            config: config.clone(),
            client,
            available_models: Arc::new(RwLock::new(HashMap::new())),
            loaded_models: Arc::new(RwLock::new(Vec::new())),
            chat_sessions: Arc::new(Mutex::new(HashMap::new())),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            completed_requests: Arc::new(Mutex::new(Vec::new())),
            metrics: Arc::new(RwLock::new(AiMetrics {
                timestamp: chrono::Utc::now(),
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                avg_response_time_ms: 0.0,
                total_tokens_generated: 0,
                avg_tokens_per_second: 0.0,
                active_sessions: 0,
                models_loaded: 0,
                memory_usage_mb: 0.0,
            })),
            is_running: Arc::new(RwLock::new(false)),
        };

        // Test connection to Ollama
        match manager.test_connection().await {
            Ok(_) => info!("✅ Connected to Ollama successfully"),
            Err(e) => warn!("⚠️  Failed to connect to Ollama: {}", e),
        }

        Ok(manager)
    }

    /// Start the Ollama manager
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Ollama AI Manager...");

        *self.is_running.write().await = true;

        // Discover available models
        self.refresh_models().await?;

        // Load default models if configured
        if let Some(default_models) = &self.config.default_ai_models {
            for model in default_models {
                if let Err(e) = self.ensure_model_available(model).await {
                    warn!("Failed to load default model '{}': {}", model, e);
                }
            }
        }

        // Start background tasks
        self.start_metrics_collection().await;
        self.start_session_cleanup().await;

        info!("✅ Ollama AI Manager started successfully");
        Ok(())
    }

    /// Stop the Ollama manager
    pub async fn stop(&self) -> Result<()> {
        info!("🛑 Stopping Ollama AI Manager...");

        *self.is_running.write().await = false;

        // Clean up active sessions
        self.chat_sessions.lock().await.clear();

        info!("✅ Ollama AI Manager stopped");
        Ok(())
    }

    /// Test connection to Ollama
    async fn test_connection(&self) -> Result<()> {
        // Try to list models as a connection test
        self.client
            .list_local_models()
            .await
            .context("Failed to connect to Ollama server")
            .map(|_| ())
    }

    /// Refresh available models from Ollama
    pub async fn refresh_models(&self) -> Result<()> {
        debug!("🔄 Refreshing available models...");

        let models = self
            .client
            .list_local_models()
            .await
            .context("Failed to list models from Ollama")?;

        let mut available_models = self.available_models.write().await;
        available_models.clear();

        for model in models {
            let ai_model = AiModel {
                name: model.name.clone(),
                size: model.size.to_string(),
                status: "available".to_string(),
                modified_at: Some(chrono::Utc::now()), // Simplified since timestamp parsing is complex
                digest: None,                          // Field not available in current API
                parameter_size: None,                  // Field not available in current API
                quantization_level: None,              // Field not available in current API
                family: None,                          // Field not available in current API
            };

            available_models.insert(model.name, ai_model);
        }

        info!("📚 Found {} available models", available_models.len());
        Ok(())
    }

    /// Ensure a model is available (download if necessary)
    pub async fn ensure_model_available(&self, model_name: &str) -> Result<()> {
        let available_models = self.available_models.read().await;

        if !available_models.contains_key(model_name) {
            info!("📥 Downloading model: {}", model_name);

            // This would typically trigger a model download
            // For now, we'll just log it
            warn!("Model download not implemented yet: {}", model_name);
        }

        Ok(())
    }

    /// Generate completion for a prompt
    pub async fn generate_completion(
        &self,
        request: InferenceRequest,
    ) -> Result<InferenceResponse> {
        let start_time = Instant::now();

        debug!("🤖 Generating completion for model: {}", request.model);

        // Add to pending requests
        self.pending_requests
            .lock()
            .await
            .insert(request.id.clone(), request.clone());

        let mut generation_request =
            GenerationRequest::new(request.model.clone(), request.prompt.clone());
        if let Some(temp) = request.temperature {
            // Note: temperature setting may need to be handled differently in the new API
            debug!("Temperature setting: {}", temp);
        }
        // Note: stream setting may not be available in current API version

        let response = self
            .client
            .generate(generation_request)
            .await
            .context("Failed to generate completion")?;

        let inference_time = start_time.elapsed();
        let tokens_generated = response.response.split_whitespace().count() as u32; // Rough estimate
        let tokens_per_second = tokens_generated as f32 / inference_time.as_secs_f32();

        let inference_response = InferenceResponse {
            id: uuid::Uuid::new_v4().to_string(),
            request_id: request.id.clone(),
            timestamp: chrono::Utc::now(),
            model: request.model,
            response_text: response.response,
            tokens_generated,
            inference_time_ms: inference_time.as_millis() as u64,
            tokens_per_second,
            confidence_score: None,
            finish_reason: if response.done {
                "completed".to_string()
            } else {
                "incomplete".to_string()
            },
        };

        // Remove from pending and add to completed
        self.pending_requests.lock().await.remove(&request.id);
        self.completed_requests
            .lock()
            .await
            .push(inference_response.clone());

        // Update metrics
        self.update_metrics(&inference_response).await;

        Ok(inference_response)
    }

    /// Start a chat session
    pub async fn start_chat_session(
        &self,
        model: &str,
        system_prompt: Option<String>,
    ) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();

        let session = ChatSession {
            id: session_id.clone(),
            created_at: chrono::Utc::now(),
            model: model.to_string(),
            messages: Vec::new(),
            context_tokens: 0,
            total_tokens: 0,
            system_prompt,
            last_activity: chrono::Utc::now(),
        };

        self.chat_sessions
            .lock()
            .await
            .insert(session_id.clone(), session);

        info!(
            "💬 Started chat session: {} with model: {}",
            session_id, model
        );
        Ok(session_id)
    }

    /// Send message in chat session
    pub async fn chat_message(&self, session_id: &str, message: &str) -> Result<String> {
        let mut sessions = self.chat_sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Chat session not found: {}", session_id))?;

        // Add user message
        session
            .messages
            .push(ChatMessage::user(message.to_string()));
        session.last_activity = chrono::Utc::now();

        // Create chat request
        let chat_request = ChatMessageRequest::new(session.model.clone(), session.messages.clone());

        let response = self
            .client
            .send_chat_messages(chat_request)
            .await
            .context("Failed to send chat message")?;

        // Add assistant response
        if let Some(message) = response.message {
            session.messages.push(message.clone());
            Ok(message.content)
        } else {
            Err(anyhow::anyhow!("No response from chat model"))
        }
    }

    /// Get specialized prompts for blockchain analysis
    pub fn get_blockchain_analysis_prompt(&self, data: &serde_json::Value) -> String {
        format!(
            "You are JARVIS-NV, an AI agent specialized in blockchain analysis and optimization. \
            Analyze the following blockchain data and provide insights on performance, \
            anomalies, and optimization opportunities:\n\n{}\n\n\
            Please provide:\n\
            1. Performance summary\n\
            2. Any detected anomalies\n\
            3. Optimization recommendations\n\
            4. Risk assessment\n\
            \nRespond in JSON format with structured analysis.",
            serde_json::to_string_pretty(data).unwrap_or_else(|_| "Invalid data".to_string())
        )
    }

    /// Get diagnostic prompt for node issues
    pub fn get_diagnostic_prompt(
        &self,
        issue_description: &str,
        metrics: &serde_json::Value,
    ) -> String {
        format!(
            "You are JARVIS-NV, an expert blockchain node diagnostician. \
            A node issue has been detected: {}\n\n\
            Current metrics:\n{}\n\n\
            Please provide:\n\
            1. Root cause analysis\n\
            2. Step-by-step diagnostic procedure\n\
            3. Recommended remediation actions\n\
            4. Prevention strategies\n\
            \nRespond with actionable technical guidance.",
            issue_description,
            serde_json::to_string_pretty(metrics).unwrap_or_else(|_| "No metrics".to_string())
        )
    }

    /// Update metrics after inference
    async fn update_metrics(&self, response: &InferenceResponse) {
        let mut metrics = self.metrics.write().await;

        metrics.timestamp = chrono::Utc::now();
        metrics.total_requests += 1;
        metrics.successful_requests += 1;
        metrics.total_tokens_generated += response.tokens_generated as u64;

        // Update averages
        let total_requests = metrics.total_requests as f64;
        metrics.avg_response_time_ms = (metrics.avg_response_time_ms * (total_requests - 1.0)
            + response.inference_time_ms as f64)
            / total_requests;
        metrics.avg_tokens_per_second = (metrics.avg_tokens_per_second
            * (total_requests - 1.0) as f32
            + response.tokens_per_second)
            / total_requests as f32;

        metrics.active_sessions = self.chat_sessions.lock().await.len() as u32;
        metrics.models_loaded = self.loaded_models.read().await.len() as u32;
    }

    /// Start metrics collection task
    async fn start_metrics_collection(&self) {
        let metrics = self.metrics.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            while *is_running.read().await {
                interval.tick().await;

                // Update metrics timestamp
                metrics.write().await.timestamp = chrono::Utc::now();
            }
        });
    }

    /// Start session cleanup task
    async fn start_session_cleanup(&self) {
        let sessions = self.chat_sessions.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes

            while *is_running.read().await {
                interval.tick().await;

                let mut sessions_map = sessions.lock().await;
                let now = chrono::Utc::now();

                // Remove sessions inactive for more than 1 hour
                sessions_map.retain(|_id, session| {
                    let inactive_duration = now.signed_duration_since(session.last_activity);
                    inactive_duration.num_hours() < 1
                });
            }
        });
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> AiMetrics {
        self.metrics.read().await.clone()
    }

    /// Get available models
    pub async fn get_available_models(&self) -> HashMap<String, AiModel> {
        self.available_models.read().await.clone()
    }

    /// Get active chat sessions count
    pub async fn get_active_sessions_count(&self) -> usize {
        self.chat_sessions.lock().await.len()
    }
}
