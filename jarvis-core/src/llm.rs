use crate::config::{Config, LLMConfig};
use crate::memory::{MemoryStore, ContextType, Interaction, InteractionType};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn generate(&self, prompt: &str, context: Option<&str>) -> Result<String>;
    async fn generate_stream(
        &self,
        prompt: &str,
        context: Option<&str>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin>>;
    fn model_name(&self) -> &str;
    fn context_window(&self) -> usize;
}

#[derive(Clone)]
pub struct LLMRouter {
    primary_provider: Arc<dyn LLMProvider>,
    fallback_providers: Vec<Arc<dyn LLMProvider>>,
    model_selector: ModelSelector,
    prompt_enhancer: PromptEnhancer,
    response_cache: Arc<RwLock<ResponseCache>>,
    memory_store: Option<Arc<RwLock<MemoryStore>>>,
    usage_stats: Arc<RwLock<UsageStats>>,
}

impl LLMRouter {
    pub async fn new(config: &Config) -> Result<Self> {
        let primary_provider = create_provider(&config.llm).await?;
        let fallback_providers = create_fallback_providers(&config.llm).await?;

        Ok(Self {
            primary_provider,
            fallback_providers,
            model_selector: ModelSelector::new(),
            prompt_enhancer: PromptEnhancer::new(),
            response_cache: Arc::new(RwLock::new(ResponseCache::new())),
            memory_store: None,
            usage_stats: Arc::new(RwLock::new(UsageStats::new())),
        })
    }

    pub fn with_memory_store(mut self, memory_store: Arc<RwLock<MemoryStore>>) -> Self {
        self.memory_store = Some(memory_store);
        self
    }

    pub async fn generate(&self, prompt: &str, context: Option<&str>) -> Result<String> {
        let start_time = std::time::Instant::now();
        
        // Check cache first
        let cache_key = self.generate_cache_key(prompt, context);
        if let Some(cached_response) = self.get_cached_response(&cache_key).await {
            return Ok(cached_response);
        }
        
        // Enhance prompt with context
        let enhanced_prompt = self.prompt_enhancer.enhance(prompt, context).await?;
        
        // Select optimal model based on task
        let selected_provider = self.model_selector.select_optimal_provider(
            &enhanced_prompt,
            &self.primary_provider,
            &self.fallback_providers
        ).await;
        
        // Try selected provider first
        let response = match selected_provider.generate(&enhanced_prompt, context).await {
            Ok(response) => {
                self.cache_response(&cache_key, &response).await;
                response
            },
            Err(e) => {
                tracing::warn!("Selected LLM provider failed: {}", e);

                // Try remaining providers
                for provider in &self.fallback_providers {
                    if Arc::ptr_eq(provider, &selected_provider) {
                        continue; // Skip already tried provider
                    }
                    
                    match provider.generate(&enhanced_prompt, context).await {
                        Ok(response) => {
                            self.cache_response(&cache_key, &response).await;
                            return self.process_response(response, start_time, provider.model_name()).await;
                        },
                        Err(e) => tracing::warn!("Fallback provider {} failed: {}", provider.model_name(), e),
                    }
                }

                return Err(anyhow::anyhow!("All LLM providers failed"));
            }
        };
        
        self.process_response(response, start_time, selected_provider.model_name()).await
    }

    pub async fn generate_stream(
        &self,
        prompt: &str,
        context: Option<&str>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin>> {
        // Try primary provider first
        match self.primary_provider.generate_stream(prompt, context).await {
            Ok(stream) => Ok(stream),
            Err(e) => {
                tracing::warn!("Primary LLM provider streaming failed: {}", e);

                // Try fallback providers
                for provider in &self.fallback_providers {
                    match provider.generate_stream(prompt, context).await {
                        Ok(stream) => return Ok(stream),
                        Err(e) => tracing::warn!("Fallback provider streaming failed: {}", e),
                    }
                }

                Err(anyhow::anyhow!("All LLM providers failed for streaming"))
            }
        }
    }

    pub async fn generate_with_system_context(
        &self,
        prompt: &str,
        system_context: &str,
    ) -> Result<String> {
        self.generate(prompt, Some(system_context)).await
    }
}

async fn create_provider(config: &LLMConfig) -> Result<Arc<dyn LLMProvider>> {
    match config.primary_provider.as_str() {
        "ollama" => Ok(Arc::new(OllamaProvider::new(config).await?)),
        "openai" => Ok(Arc::new(OpenAIProvider::new(config).await?)),
        "claude" => Ok(Arc::new(ClaudeProvider::new(config).await?)),
        _ => Err(anyhow::anyhow!(
            "Unknown LLM provider: {}",
            config.primary_provider
        )),
    }
}

async fn create_fallback_providers(config: &LLMConfig) -> Result<Vec<Arc<dyn LLMProvider>>> {
    let mut providers = Vec::new();
    
    // Add all available providers as fallbacks except the primary one
    let provider_types = ["ollama", "openai", "claude"];
    
    for provider_type in provider_types {
        if provider_type != config.primary_provider.as_str() {
            match create_single_provider(provider_type, config).await {
                Ok(provider) => providers.push(provider),
                Err(e) => tracing::warn!("Failed to create fallback provider {}: {}", provider_type, e),
            }
        }
    }
    
    Ok(providers)
}

async fn create_single_provider(provider_type: &str, config: &LLMConfig) -> Result<Arc<dyn LLMProvider>> {
    match provider_type {
        "ollama" => Ok(Arc::new(OllamaProvider::new(config).await?)),
        "openai" => Ok(Arc::new(OpenAIProvider::new(config).await?)),
        "claude" => Ok(Arc::new(ClaudeProvider::new(config).await?)),
        _ => Err(anyhow::anyhow!("Unknown provider type: {}", provider_type)),
    }
}

// Ollama Provider
pub struct OllamaProvider {
    client: Client,
    base_url: String,
    model: String,
    context_window: usize,
}

impl OllamaProvider {
    pub async fn new(config: &LLMConfig) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            base_url: config.ollama_url.clone(),
            model: config
                .default_model
                .clone()
                .unwrap_or_else(|| "llama3.1:8b".to_string()),
            context_window: config.context_window,
        })
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn generate(&self, prompt: &str, _context: Option<&str>) -> Result<String> {
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let response = self
            .client
            .post(&format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await?;

        let result: OllamaResponse = response.json().await?;
        Ok(result.response)
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        _context: Option<&str>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin>> {
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: true,
        };

        let response = self
            .client
            .post(&format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Ollama streaming request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Ollama streaming API error {}: {}",
                status,
                error_text
            ));
        }

        let stream = response
            .bytes_stream()
            .map(|chunk_result| match chunk_result {
                Ok(chunk) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    for line in chunk_str.lines() {
                        if let Ok(response) = serde_json::from_str::<OllamaStreamResponse>(line) {
                            if !response.done {
                                return Ok(response.response);
                            }
                        }
                    }
                    Ok(String::new())
                }
                Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
            })
            .filter(|result| {
                futures::future::ready(match result {
                    Ok(s) => !s.is_empty(),
                    Err(_) => true,
                })
            });

        Ok(Box::new(Box::pin(stream)))
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn context_window(&self) -> usize {
        self.context_window
    }
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

#[derive(Deserialize)]
struct OllamaStreamResponse {
    response: String,
    done: bool,
}

// OpenAI API structures
#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    stream: bool,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Deserialize)]
struct OpenAIStreamResponse {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIMessage,
}

// Claude API structures
#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
    stream: Option<bool>,
}

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    text: String,
}

#[derive(Deserialize)]
struct ClaudeStreamResponse {
    #[serde(rename = "type")]
    type_field: String,
    delta: Option<ClaudeStreamDelta>,
}

#[derive(Deserialize)]
struct ClaudeStreamDelta {
    text: Option<String>,
}

// OpenAI Provider
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    model: String,
    context_window: usize,
}

impl OpenAIProvider {
    pub async fn new(config: &LLMConfig) -> Result<Self> {
        let api_key = config
            .openai_api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("OpenAI API key not configured"))?;

        Ok(Self {
            client: Client::new(),
            api_key: api_key.clone(),
            model: config
                .default_model
                .clone()
                .unwrap_or_else(|| "gpt-4o".to_string()),
            context_window: config.context_window,
        })
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn generate(&self, prompt: &str, context: Option<&str>) -> Result<String> {
        let messages = self.build_messages(prompt, context);

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            stream: false,
            max_tokens: Some(4096),
            temperature: Some(0.7),
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "OpenAI API error {}: {}",
                status,
                error_text
            ));
        }

        let result: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse OpenAI response: {}", e))?;

        result
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .map(|content| content.clone())
            .ok_or_else(|| anyhow::anyhow!("No content in OpenAI response"))
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        context: Option<&str>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin>> {
        let messages = self.build_messages(prompt, context);

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            max_tokens: Some(4096),
            temperature: Some(0.7),
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI streaming request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "OpenAI streaming API error {}: {}",
                status,
                error_text
            ));
        }

        let stream = response
            .bytes_stream()
            .map(|chunk_result| {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        // Parse SSE format: "data: {json}"
                        for line in chunk_str.lines() {
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    continue;
                                }
                                match serde_json::from_str::<OpenAIStreamResponse>(data) {
                                    Ok(response) => {
                                        if let Some(choice) = response.choices.first() {
                                            if let Some(content) = &choice.delta.content {
                                                return Ok(content.clone());
                                            }
                                        }
                                    }
                                    Err(_) => continue,
                                }
                            }
                        }
                        Ok(String::new())
                    }
                    Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
                }
            })
            .filter(|result| {
                futures::future::ready(match result {
                    Ok(s) => !s.is_empty(),
                    Err(_) => true,
                })
            });

        Ok(Box::new(Box::pin(stream)))
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn context_window(&self) -> usize {
        self.context_window
    }
}

impl OpenAIProvider {
    fn build_messages(&self, prompt: &str, context: Option<&str>) -> Vec<OpenAIMessage> {
        let mut messages = Vec::new();

        if let Some(ctx) = context {
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: Some(ctx.to_string()),
            });
        }

        messages.push(OpenAIMessage {
            role: "user".to_string(),
            content: Some(prompt.to_string()),
        });

        messages
    }
}

// Claude Provider
pub struct ClaudeProvider {
    client: Client,
    api_key: String,
    model: String,
    context_window: usize,
}

impl ClaudeProvider {
    pub async fn new(config: &LLMConfig) -> Result<Self> {
        let api_key = config
            .claude_api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Claude API key not configured"))?;

        Ok(Self {
            client: Client::new(),
            api_key: api_key.clone(),
            model: config
                .default_model
                .clone()
                .unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string()),
            context_window: config.context_window,
        })
    }
}

#[async_trait]
impl LLMProvider for ClaudeProvider {
    async fn generate(&self, prompt: &str, context: Option<&str>) -> Result<String> {
        let messages = self.build_messages(prompt, context);

        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            messages,
            stream: Some(false),
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Claude API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Claude API error {}: {}",
                status,
                error_text
            ));
        }

        let result: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse Claude response: {}", e))?;

        result
            .content
            .first()
            .map(|content| content.text.clone())
            .ok_or_else(|| anyhow::anyhow!("No content in Claude response"))
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        context: Option<&str>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin>> {
        let messages = self.build_messages(prompt, context);

        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            messages,
            stream: Some(true),
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Claude streaming request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Claude streaming API error {}: {}",
                status,
                error_text
            ));
        }

        let stream = response
            .bytes_stream()
            .map(|chunk_result| {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        // Parse SSE format: "data: {json}"
                        for line in chunk_str.lines() {
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    continue;
                                }
                                match serde_json::from_str::<ClaudeStreamResponse>(data) {
                                    Ok(response) => {
                                        if response.type_field == "content_block_delta" {
                                            if let Some(delta) = response.delta {
                                                if let Some(text) = delta.text {
                                                    return Ok(text);
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => continue,
                                }
                            }
                        }
                        Ok(String::new())
                    }
                    Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
                }
            })
            .filter(|result| {
                futures::future::ready(match result {
                    Ok(s) => !s.is_empty(),
                    Err(_) => true,
                })
            });

        Ok(Box::new(Box::pin(stream)))
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn context_window(&self) -> usize {
        self.context_window
    }
}

impl ClaudeProvider {
    fn build_messages(&self, prompt: &str, context: Option<&str>) -> Vec<ClaudeMessage> {
        let mut messages = Vec::new();

        if let Some(ctx) = context {
            messages.push(ClaudeMessage {
                role: "user".to_string(),
                content: format!("Context: {}\n\nUser: {}", ctx, prompt),
            });
        } else {
            messages.push(ClaudeMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            });
        }

        messages
    }
}

// Enhanced LLM Components

/// Intelligent model selection based on task characteristics
#[derive(Clone, Debug)]
pub struct ModelSelector {
    task_patterns: HashMap<String, String>, // pattern -> preferred model
    performance_history: HashMap<String, ModelPerformance>,
}

#[derive(Clone, Debug)]
pub struct ModelPerformance {
    average_latency: f64,
    success_rate: f64,
    quality_score: f64,
    cost_efficiency: f64,
}

impl ModelSelector {
    pub fn new() -> Self {
        let mut task_patterns = HashMap::new();
        task_patterns.insert("code".to_string(), "claude".to_string());
        task_patterns.insert("explain".to_string(), "claude".to_string());
        task_patterns.insert("creative".to_string(), "openai".to_string());
        task_patterns.insert("simple".to_string(), "ollama".to_string());
        task_patterns.insert("fast".to_string(), "ollama".to_string());
        
        Self {
            task_patterns,
            performance_history: HashMap::new(),
        }
    }

    pub async fn select_optimal_provider(
        &self,
        prompt: &str,
        primary: &Arc<dyn LLMProvider>,
        fallbacks: &[Arc<dyn LLMProvider>],
    ) -> Arc<dyn LLMProvider> {
        // Analyze prompt characteristics
        let task_type = self.analyze_task_type(prompt);
        
        // Check if we have a preferred provider for this task type
        if let Some(preferred_model) = self.task_patterns.get(&task_type) {
            // Try to find the preferred provider
            if primary.model_name().contains(preferred_model) {
                return Arc::clone(primary);
            }
            
            for provider in fallbacks {
                if provider.model_name().contains(preferred_model) {
                    return Arc::clone(provider);
                }
            }
        }
        
        // Fallback to primary provider
        Arc::clone(primary)
    }

    fn analyze_task_type(&self, prompt: &str) -> String {
        let prompt_lower = prompt.to_lowercase();
        
        if prompt_lower.contains("code") || prompt_lower.contains("rust") || prompt_lower.contains("function") {
            "code".to_string()
        } else if prompt_lower.contains("explain") || prompt_lower.contains("what") || prompt_lower.contains("how") {
            "explain".to_string()
        } else if prompt_lower.contains("create") || prompt_lower.contains("write") || prompt_lower.contains("generate") {
            "creative".to_string()
        } else if prompt.len() < 50 {
            "simple".to_string()
        } else {
            "complex".to_string()
        }
    }
}

/// Advanced prompt enhancement with context injection
#[derive(Clone, Debug)]
pub struct PromptEnhancer {
    system_prompts: HashMap<String, String>,
    context_templates: HashMap<String, String>,
}

impl PromptEnhancer {
    pub fn new() -> Self {
        let mut system_prompts = HashMap::new();
        system_prompts.insert(
            "code".to_string(),
            "You are an expert software engineer specialized in Rust, systems programming, and DevOps. Provide precise, efficient, and well-documented solutions.".to_string()
        );
        system_prompts.insert(
            "explain".to_string(),
            "You are a helpful technical assistant. Explain concepts clearly and concisely, providing practical examples when appropriate.".to_string()
        );
        
        let mut context_templates = HashMap::new();
        context_templates.insert(
            "default".to_string(),
            "Current context: {context}\n\nUser request: {prompt}".to_string()
        );
        
        Self {
            system_prompts,
            context_templates,
        }
    }

    pub async fn enhance(&self, prompt: &str, context: Option<&str>) -> Result<String> {
        let task_type = self.detect_task_type(prompt);
        
        let mut enhanced = String::new();
        
        // Add system prompt if available
        if let Some(system_prompt) = self.system_prompts.get(&task_type) {
            enhanced.push_str(system_prompt);
            enhanced.push_str("\n\n");
        }
        
        // Add context if provided
        if let Some(ctx) = context {
            if let Some(template) = self.context_templates.get("default") {
                let contextualized = template
                    .replace("{context}", ctx)
                    .replace("{prompt}", prompt);
                enhanced.push_str(&contextualized);
            } else {
                enhanced.push_str(&format!("Context: {}\n\n{}", ctx, prompt));
            }
        } else {
            enhanced.push_str(prompt);
        }
        
        Ok(enhanced)
    }

    fn detect_task_type(&self, prompt: &str) -> String {
        let prompt_lower = prompt.to_lowercase();
        
        if prompt_lower.contains("code") || prompt_lower.contains("rust") || prompt_lower.contains("function") {
            "code".to_string()
        } else if prompt_lower.contains("explain") || prompt_lower.contains("what") || prompt_lower.contains("how") {
            "explain".to_string()
        } else {
            "general".to_string()
        }
    }
}

/// Response caching for improved performance
#[derive(Clone, Debug)]
pub struct ResponseCache {
    cache: HashMap<String, CachedResponse>,
    max_size: usize,
    ttl_seconds: u64,
}

#[derive(Clone, Debug)]
pub struct CachedResponse {
    content: String,
    timestamp: DateTime<Utc>,
    access_count: u32,
}

impl ResponseCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_size: 1000,
            ttl_seconds: 3600, // 1 hour
        }
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        if let Some(cached) = self.cache.get_mut(key) {
            let now = Utc::now();
            let age = now.signed_duration_since(cached.timestamp).num_seconds() as u64;
            
            if age < self.ttl_seconds {
                cached.access_count += 1;
                return Some(cached.content.clone());
            } else {
                self.cache.remove(key);
            }
        }
        None
    }

    pub fn insert(&mut self, key: String, content: String) {
        // Evict old entries if at capacity
        if self.cache.len() >= self.max_size {
            self.evict_oldest();
        }
        
        self.cache.insert(key, CachedResponse {
            content,
            timestamp: Utc::now(),
            access_count: 1,
        });
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self.cache
            .iter()
            .min_by_key(|(_, v)| v.timestamp)
            .map(|(k, _)| k.clone())
        {
            self.cache.remove(&oldest_key);
        }
    }
}

/// Usage statistics and analytics
#[derive(Clone, Debug)]
pub struct UsageStats {
    provider_stats: HashMap<String, ProviderStats>,
    total_requests: u64,
    total_tokens: u64,
    total_cost: f64,
}

#[derive(Clone, Debug)]
pub struct ProviderStats {
    requests: u64,
    failures: u64,
    total_latency: f64,
    tokens_used: u64,
    estimated_cost: f64,
}

impl UsageStats {
    pub fn new() -> Self {
        Self {
            provider_stats: HashMap::new(),
            total_requests: 0,
            total_tokens: 0,
            total_cost: 0.0,
        }
    }

    pub fn record_request(&mut self, provider: &str, latency: f64, tokens: u64, cost: f64, success: bool) {
        let stats = self.provider_stats.entry(provider.to_string()).or_insert(ProviderStats {
            requests: 0,
            failures: 0,
            total_latency: 0.0,
            tokens_used: 0,
            estimated_cost: 0.0,
        });

        stats.requests += 1;
        stats.total_latency += latency;
        stats.tokens_used += tokens;
        stats.estimated_cost += cost;
        
        if !success {
            stats.failures += 1;
        }

        self.total_requests += 1;
        self.total_tokens += tokens;
        self.total_cost += cost;
    }

    pub fn get_summary(&self) -> serde_json::Value {
        serde_json::json!({
            "total_requests": self.total_requests,
            "total_tokens": self.total_tokens,
            "total_cost": self.total_cost,
            "providers": self.provider_stats.iter().map(|(name, stats)| {
                serde_json::json!({
                    "provider": name,
                    "requests": stats.requests,
                    "success_rate": if stats.requests > 0 { 
                        ((stats.requests - stats.failures) as f64 / stats.requests as f64) * 100.0 
                    } else { 0.0 },
                    "avg_latency": if stats.requests > 0 { 
                        stats.total_latency / stats.requests as f64 
                    } else { 0.0 },
                    "tokens_used": stats.tokens_used,
                    "estimated_cost": stats.estimated_cost,
                })
            }).collect::<Vec<_>>()
        })
    }
}

impl LLMRouter {
    // Helper methods for enhanced functionality
    
    fn generate_cache_key(&self, prompt: &str, context: Option<&str>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        prompt.hash(&mut hasher);
        if let Some(ctx) = context {
            ctx.hash(&mut hasher);
        }
        format!("{:x}", hasher.finish())
    }

    async fn get_cached_response(&self, key: &str) -> Option<String> {
        self.response_cache.write().await.get(key)
    }

    async fn cache_response(&self, key: &str, response: &str) {
        self.response_cache.write().await.insert(key.to_string(), response.to_string());
    }

    async fn process_response(&self, response: String, start_time: std::time::Instant, provider: &str) -> Result<String> {
        let latency = start_time.elapsed().as_millis() as f64;
        let estimated_tokens = response.split_whitespace().count() as u64;
        let estimated_cost = self.estimate_cost(provider, estimated_tokens);
        
        // Record usage stats
        self.usage_stats.write().await.record_request(
            provider,
            latency,
            estimated_tokens,
            estimated_cost,
            true
        );
        
        // Store interaction in memory if available
        if let Some(memory) = &self.memory_store {
            let interaction = Interaction {
                timestamp: Utc::now(),
                interaction_type: InteractionType::Query,
                content: response.clone(),
                success: true,
                execution_time: Some(latency as u64),
                context_tags: vec!["llm".to_string(), provider.to_string()],
            };
            
            if let Ok(mut mem) = memory.try_write() {
                let _ = mem.update_global_context(interaction).await;
            }
        }
        
        Ok(response)
    }

    fn estimate_cost(&self, provider: &str, tokens: u64) -> f64 {
        // Rough cost estimates per 1k tokens
        let cost_per_1k = match provider {
            name if name.contains("gpt-4") => 0.03,
            name if name.contains("gpt-3.5") => 0.002,
            name if name.contains("claude") => 0.025,
            _ => 0.0, // Ollama is typically free
        };
        
        (tokens as f64 / 1000.0) * cost_per_1k
    }

    /// Get comprehensive usage statistics
    pub async fn get_usage_stats(&self) -> serde_json::Value {
        self.usage_stats.read().await.get_summary()
    }

    /// Clear response cache
    pub async fn clear_cache(&self) {
        self.response_cache.write().await.cache.clear();
    }

    /// Generate with enhanced context from memory
    pub async fn generate_with_memory_context(&self, prompt: &str) -> Result<String> {
        if let Some(memory) = &self.memory_store {
            if let Ok(mut mem) = memory.try_write() {
                // Search for relevant context
                let context_entries = mem.search_context(prompt, 3).await?;
                let context = context_entries.iter()
                    .map(|entry| entry.content.clone())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                
                if !context.is_empty() {
                    return self.generate(prompt, Some(&context)).await;
                }
            }
        }
        
        self.generate(prompt, None).await
    }
}
