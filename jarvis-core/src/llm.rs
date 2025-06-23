use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::config::{Config, LLMConfig};

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn generate(&self, prompt: &str, context: Option<&str>) -> Result<String>;
    async fn generate_stream(&self, prompt: &str, context: Option<&str>) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin>>;
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

    pub async fn generate_with_system_context(&self, prompt: &str, system_context: &str) -> Result<String> {
        let full_prompt = format!(
            "System Context:\n{}\n\nUser Query:\n{}",
            system_context, prompt
        );
        
        self.generate(&full_prompt, None).await
    }
}

async fn create_provider(config: &LLMConfig) -> Result<std::sync::Arc<dyn LLMProvider>> {
    match config.primary_provider.as_str() {
        "ollama" => Ok(std::sync::Arc::new(OllamaProvider::new(config).await?)),
        "openai" => Ok(std::sync::Arc::new(OpenAIProvider::new(config).await?)),
        "claude" => Ok(std::sync::Arc::new(ClaudeProvider::new(config).await?)),
        _ => Err(anyhow::anyhow!("Unknown LLM provider: {}", config.primary_provider)),
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
            model: config.default_model.clone(),
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

        let response = self.client
            .post(&format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await?;

        let result: OllamaResponse = response.json().await?;
        Ok(result.response)
    }

    async fn generate_stream(&self, _prompt: &str, _context: Option<&str>) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin>> {
        todo!("Implement streaming for Ollama")
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

// OpenAI Provider (placeholder)
pub struct OpenAIProvider {
    api_key: String,
    model: String,
    context_window: usize,
}

impl OpenAIProvider {
    pub async fn new(config: &LLMConfig) -> Result<Self> {
        let api_key = config.openai_api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("OpenAI API key not configured"))?;
            
        Ok(Self {
            api_key: api_key.clone(),
            model: "gpt-4".to_string(),
            context_window: config.context_window,
        })
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn generate(&self, prompt: &str, _context: Option<&str>) -> Result<String> {
        // TODO: Implement OpenAI API calls
        Ok(format!("OpenAI response to: {}", prompt))
    }

    async fn generate_stream(&self, _prompt: &str, _context: Option<&str>) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin>> {
        todo!("Implement streaming for OpenAI")
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn context_window(&self) -> usize {
        self.context_window
    }
}

// Claude Provider (placeholder)
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    context_window: usize,
}

impl ClaudeProvider {
    pub async fn new(config: &LLMConfig) -> Result<Self> {
        let api_key = config.claude_api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Claude API key not configured"))?;
            
        Ok(Self {
            api_key: api_key.clone(),
            model: "claude-3-sonnet-20240229".to_string(),
            context_window: config.context_window,
        })
    }
}

#[async_trait]
impl LLMProvider for ClaudeProvider {
    async fn generate(&self, prompt: &str, _context: Option<&str>) -> Result<String> {
        // TODO: Implement Claude API calls
        Ok(format!("Claude response to: {}", prompt))
    }

    async fn generate_stream(&self, _prompt: &str, _context: Option<&str>) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin>> {
        todo!("Implement streaming for Claude")
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn context_window(&self) -> usize {
        self.context_window
    }
}
