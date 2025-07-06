use crate::config::{Config, LLMConfig};
use anyhow::Result;
use async_trait::async_trait;
use futures::stream::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

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
    primary_provider: std::sync::Arc<dyn LLMProvider>,
    fallback_providers: Vec<std::sync::Arc<dyn LLMProvider>>,
}

impl LLMRouter {
    pub async fn new(config: &Config) -> Result<Self> {
        let primary_provider = create_provider(&config.llm).await?;

        Ok(Self {
            primary_provider,
            fallback_providers: vec![], // TODO: Add fallback providers
        })
    }

    pub async fn generate(&self, prompt: &str, context: Option<&str>) -> Result<String> {
        // Try primary provider first
        match self.primary_provider.generate(prompt, context).await {
            Ok(response) => Ok(response),
            Err(e) => {
                tracing::warn!("Primary LLM provider failed: {}", e);

                // Try fallback providers
                for provider in &self.fallback_providers {
                    match provider.generate(prompt, context).await {
                        Ok(response) => return Ok(response),
                        Err(e) => tracing::warn!("Fallback provider failed: {}", e),
                    }
                }

                Err(anyhow::anyhow!("All LLM providers failed"))
            }
        }
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

async fn create_provider(config: &LLMConfig) -> Result<std::sync::Arc<dyn LLMProvider>> {
    match config.primary_provider.as_str() {
        "ollama" => Ok(std::sync::Arc::new(OllamaProvider::new(config).await?)),
        "openai" => Ok(std::sync::Arc::new(OpenAIProvider::new(config).await?)),
        "claude" => Ok(std::sync::Arc::new(ClaudeProvider::new(config).await?)),
        _ => Err(anyhow::anyhow!(
            "Unknown LLM provider: {}",
            config.primary_provider
        )),
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
