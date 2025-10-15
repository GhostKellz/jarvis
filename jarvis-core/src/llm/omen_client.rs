//! Omen LLM Client Integration
//!
//! Provides a client for interacting with the Omen AI Gateway for intelligent
//! model routing, cost optimization, and multi-provider support.

use anyhow::{Context, Result};
use omen::types::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, MessageContent, OmenConfig,
};
use std::collections::HashMap;

/// Client for interacting with Omen AI Gateway
#[derive(Clone)]
pub struct OmenClient {
    http_client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

impl OmenClient {
    /// Create a new Omen client
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url,
            api_key,
        }
    }

    /// Create Omen client from configuration
    pub fn from_config(config: &crate::config::LLMConfig) -> Result<Self> {
        let base_url = config.omen_url();
        let api_key = config.omen_key();

        Ok(Self::new(base_url, api_key))
    }

    /// Send a chat completion request to Omen
    ///
    /// # Arguments
    /// * `messages` - Conversation messages
    /// * `intent` - Optional intent hint for routing (code, system, devops, reason)
    /// * `stream` - Enable streaming response
    pub async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        intent: Option<&str>,
        stream: bool,
    ) -> Result<ChatCompletionResponse> {
        let mut tags = HashMap::new();
        tags.insert("source".to_string(), "jarvis".to_string());

        if let Some(intent) = intent {
            tags.insert("intent".to_string(), intent.to_string());
        }

        let request = ChatCompletionRequest {
            model: "auto".to_string(), // Let Omen choose optimal model
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            tools: None,
            tool_choice: None,
            tags: Some(tags),
            omen: Some(OmenConfig {
                strategy: Some("single".to_string()),
                budget_usd: Some(0.10),
                max_latency_ms: Some(5000),
                ..Default::default()
            }),
        };

        let url = format!("{}/chat/completions", self.base_url);
        tracing::debug!("Sending request to Omen: {}", url);

        let mut req_builder = self.http_client.post(&url).json(&request);

        if let Some(ref key) = self.api_key {
            req_builder = req_builder.bearer_auth(key);
        }

        let response = req_builder
            .send()
            .await
            .context("Failed to send request to Omen")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| String::from("Unknown error"));
            anyhow::bail!("Omen API error ({}): {}", status, error_text);
        }

        let result = response
            .json()
            .await
            .context("Failed to parse Omen response")?;

        Ok(result)
    }

    /// Send a simple text prompt and get response
    ///
    /// # Arguments
    /// * `prompt` - The text prompt
    /// * `intent` - Optional intent hint (code, system, devops, reason)
    pub async fn complete(&self, prompt: &str, intent: Option<&str>) -> Result<String> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text(prompt.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];

        let response = self.chat_completion(messages, intent, false).await?;

        Ok(response
            .choices
            .first()
            .map(|c| c.message.content.to_string())
            .unwrap_or_default())
    }

    /// Complete with system prompt
    pub async fn complete_with_system(
        &self,
        system: &str,
        user: &str,
        intent: Option<&str>,
    ) -> Result<String> {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: MessageContent::Text(system.to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: MessageContent::Text(user.to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let response = self.chat_completion(messages, intent, false).await?;

        Ok(response
            .choices
            .first()
            .map(|c| c.message.content.to_string())
            .unwrap_or_default())
    }

    /// Code generation task
    pub async fn code(&self, request: &str) -> Result<String> {
        let system = "You are an expert Rust programmer. Generate clean, idiomatic, and well-documented code.";
        self.complete_with_system(system, request, Some("code"))
            .await
    }

    /// System administration task
    pub async fn system(&self, request: &str) -> Result<String> {
        let system = "You are an expert Linux system administrator specializing in Arch Linux. Provide safe, tested commands with explanations.";
        self.complete_with_system(system, request, Some("system"))
            .await
    }

    /// DevOps task
    pub async fn devops(&self, request: &str) -> Result<String> {
        let system = "You are an expert DevOps engineer. Provide infrastructure solutions using Docker, Kubernetes, and modern tooling.";
        self.complete_with_system(system, request, Some("devops"))
            .await
    }

    /// Complex reasoning task
    pub async fn reason(&self, question: &str) -> Result<String> {
        self.complete(question, Some("reason")).await
    }

    /// Get streaming response
    pub async fn complete_stream(
        &self,
        prompt: &str,
        intent: Option<&str>,
    ) -> Result<impl futures::Stream<Item = Result<String>>> {
        use futures::stream::StreamExt;

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text(prompt.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];

        let mut tags = HashMap::new();
        tags.insert("source".to_string(), "jarvis".to_string());
        if let Some(intent) = intent {
            tags.insert("intent".to_string(), intent.to_string());
        }

        let request = ChatCompletionRequest {
            model: "auto".to_string(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: true,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            tools: None,
            tool_choice: None,
            tags: Some(tags),
            omen: None,
        };

        let url = format!("{}/chat/completions", self.base_url);

        let mut req_builder = self.http_client.post(&url).json(&request);

        if let Some(ref key) = self.api_key {
            req_builder = req_builder.bearer_auth(key);
        }

        let response = req_builder.send().await?;

        let stream = response.bytes_stream().map(|chunk| {
            chunk
                .map_err(Into::into)
                .and_then(|bytes| {
                    let text = String::from_utf8(bytes.to_vec())?;
                    Ok(text)
                })
        });

        Ok(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_omen_client_creation() {
        let client = OmenClient::new(
            "http://localhost:8080/v1".to_string(),
            Some("test-key".to_string()),
        );

        assert_eq!(client.base_url, "http://localhost:8080/v1");
        assert!(client.api_key.is_some());
    }
}
