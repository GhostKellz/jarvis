//! Ollama Client Integration
//!
//! Provides direct client access to Ollama for local model inference.
//! Supports chat completions, streaming, and model management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Client for interacting with Ollama directly
#[derive(Clone)]
pub struct OllamaClient {
    http_client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Serialize)]
pub struct OllamaChatRequest {
    pub model: String,
    pub messages: Vec<OllamaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct OllamaChatResponse {
    pub model: String,
    pub message: OllamaMessage,
    #[serde(default)]
    pub done: bool,
}

#[derive(Debug, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub modified_at: String,
    pub size: i64,
}

#[derive(Debug, Deserialize)]
pub struct OllamaListResponse {
    pub models: Vec<OllamaModel>,
}

impl OllamaClient {
    /// Create a new Ollama client
    pub fn new(base_url: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url,
        }
    }

    /// Create Ollama client from configuration
    pub fn from_config(config: &crate::config::LLMConfig) -> Result<Self> {
        Ok(Self::new(config.ollama_url.clone()))
    }

    /// Send a chat completion request to Ollama
    pub async fn chat(&self, model: &str, messages: Vec<OllamaMessage>, temperature: Option<f32>) -> Result<String> {
        let options = temperature.map(|t| OllamaOptions {
            temperature: Some(t),
            num_predict: None,
            top_k: None,
            top_p: None,
        });

        let request = OllamaChatRequest {
            model: model.to_string(),
            messages,
            stream: Some(false),
            options,
        };

        let url = format!("{}/api/chat", self.base_url);
        tracing::debug!("Sending request to Ollama: {}", url);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| String::from("Unknown error"));
            anyhow::bail!("Ollama API error ({}): {}", status, error_text);
        }

        let result: OllamaChatResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        Ok(result.message.content)
    }

    /// Send a simple text prompt and get response
    pub async fn complete(&self, model: &str, prompt: &str, temperature: Option<f32>) -> Result<String> {
        let messages = vec![OllamaMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        self.chat(model, messages, temperature).await
    }

    /// Complete with system prompt
    pub async fn complete_with_system(
        &self,
        model: &str,
        system: &str,
        user: &str,
        temperature: Option<f32>,
    ) -> Result<String> {
        let messages = vec![
            OllamaMessage {
                role: "system".to_string(),
                content: system.to_string(),
            },
            OllamaMessage {
                role: "user".to_string(),
                content: user.to_string(),
            },
        ];

        self.chat(model, messages, temperature).await
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<OllamaModel>> {
        let url = format!("{}/api/tags", self.base_url);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to list Ollama models")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to list models: {}", response.status());
        }

        let result: OllamaListResponse = response
            .json()
            .await
            .context("Failed to parse model list")?;

        Ok(result.models)
    }

    /// Check if Ollama is healthy
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.base_url);

        match self.http_client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Generate code with appropriate system prompt
    pub async fn code(&self, model: &str, request: &str, temperature: Option<f32>) -> Result<String> {
        let system = "You are an expert Rust programmer. Generate clean, idiomatic, and well-documented code. \
                      Focus on safety, performance, and correctness.";
        self.complete_with_system(model, system, request, temperature).await
    }

    /// System administration task
    pub async fn system(&self, model: &str, request: &str, temperature: Option<f32>) -> Result<String> {
        let system = "You are an expert Arch Linux system administrator. Provide safe, tested commands with clear explanations. \
                      Always explain what each command does and any potential risks. Use pacman and yay appropriately.";
        self.complete_with_system(model, system, request, temperature).await
    }

    /// DevOps task
    pub async fn devops(&self, model: &str, request: &str, temperature: Option<f32>) -> Result<String> {
        let system = "You are an expert DevOps engineer. Provide infrastructure solutions using Docker, Kubernetes, and modern tooling. \
                      Focus on best practices, security, and maintainability.";
        self.complete_with_system(model, system, request, temperature).await
    }

    /// Streaming chat completion
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<OllamaMessage>,
        temperature: Option<f32>,
    ) -> Result<impl futures::Stream<Item = Result<String>>> {
        use futures::stream::StreamExt;

        let options = temperature.map(|t| OllamaOptions {
            temperature: Some(t),
            num_predict: None,
            top_k: None,
            top_p: None,
        });

        let request = OllamaChatRequest {
            model: model.to_string(),
            messages,
            stream: Some(true),
            options,
        };

        let url = format!("{}/api/chat", self.base_url);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to Ollama")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama streaming API error: {}", response.status());
        }

        let stream = response.bytes_stream().map(|chunk| {
            chunk
                .map_err(Into::into)
                .and_then(|bytes| {
                    let text = String::from_utf8(bytes.to_vec())?;
                    // Parse JSONL format
                    if let Ok(response) = serde_json::from_str::<OllamaChatResponse>(&text) {
                        Ok(response.message.content)
                    } else {
                        Ok(String::new()) // Skip invalid lines
                    }
                })
        });

        Ok(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ollama_client_creation() {
        let client = OllamaClient::new("http://localhost:11434".to_string());
        assert_eq!(client.base_url, "http://localhost:11434");
    }

    #[tokio::test]
    async fn test_message_serialization() {
        let msg = OllamaMessage {
            role: "user".to_string(),
            content: "test".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"test\""));
    }
}
