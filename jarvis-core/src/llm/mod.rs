pub mod ollama_client;
pub mod omen_client;

pub use ollama_client::OllamaClient;
pub use omen_client::OmenClient;

/// LLMRouter routes LLM requests to appropriate backends
#[derive(Clone)]
pub struct LLMRouter {
    omen_client: Option<OmenClient>,
    ollama_client: Option<OllamaClient>,
    default_model: String,
    primary_provider: String,
}

/// Intent type for routing decisions
#[derive(Debug, Clone, Copy)]
pub enum Intent {
    Code,
    System,
    DevOps,
    Reason,
}

impl LLMRouter {
    pub async fn new(config: &crate::config::Config) -> anyhow::Result<Self> {
        let omen_client = if config.llm.omen_enabled.unwrap_or(false) {
            tracing::info!("Initializing Omen client at {}", config.llm.omen_url());
            Some(OmenClient::new(
                config.llm.omen_base_url.clone().unwrap_or_else(|| "http://localhost:8080/v1".to_string()),
                config.llm.omen_api_key.clone(),
            ))
        } else {
            None
        };

        let ollama_client = if config.llm.primary_provider == "ollama" || omen_client.is_none() {
            tracing::info!("Initializing Ollama client at {}", config.llm.ollama_url);
            Some(OllamaClient::new(config.llm.ollama_url.clone()))
        } else {
            None
        };

        let default_model = config.llm.default_model.clone()
            .unwrap_or_else(|| "llama3.1:8b".to_string());

        Ok(Self {
            omen_client,
            ollama_client,
            default_model,
            primary_provider: config.llm.primary_provider.clone(),
        })
    }

    /// Generate a response using the configured LLM backend
    pub async fn generate(&self, prompt: &str, _options: Option<serde_json::Value>) -> anyhow::Result<String> {
        // Try Omen first if available (intelligent routing)
        if let Some(omen) = &self.omen_client {
            tracing::debug!("Routing through Omen (auto-intent)");
            return omen.code(prompt).await;
        }

        // Fallback to direct Ollama
        if let Some(ollama) = &self.ollama_client {
            tracing::debug!("Using direct Ollama: {}", self.default_model);
            return ollama.complete(&self.default_model, prompt, Some(0.7)).await;
        }

        anyhow::bail!("No LLM backend configured. Enable Omen or Ollama in jarvis.toml")
    }

    /// Generate with specific intent routing
    pub async fn generate_with_intent(&self, prompt: &str, intent: Intent) -> anyhow::Result<String> {
        match (&self.omen_client, &self.ollama_client, intent) {
            // Omen available - use intelligent routing
            (Some(omen), _, Intent::Code) => {
                tracing::debug!("Routing code intent through Omen");
                omen.code(prompt).await
            }
            (Some(omen), _, Intent::System) => {
                tracing::debug!("Routing system intent through Omen");
                omen.system(prompt).await
            }
            (Some(omen), _, Intent::DevOps) => {
                tracing::debug!("Routing devops intent through Omen");
                omen.devops(prompt).await
            }
            (Some(omen), _, Intent::Reason) => {
                tracing::debug!("Routing reason intent through Omen");
                omen.reason(prompt).await
            }

            // Ollama fallback with specialized prompts
            (None, Some(ollama), Intent::Code) => {
                tracing::debug!("Using Ollama for code intent: {}", self.default_model);
                ollama.code(&self.default_model, prompt, Some(0.7)).await
            }
            (None, Some(ollama), Intent::System) => {
                tracing::debug!("Using Ollama for system intent: {}", self.default_model);
                ollama.system(&self.default_model, prompt, Some(0.7)).await
            }
            (None, Some(ollama), Intent::DevOps) => {
                tracing::debug!("Using Ollama for devops intent: {}", self.default_model);
                ollama.devops(&self.default_model, prompt, Some(0.7)).await
            }
            (None, Some(ollama), Intent::Reason) => {
                tracing::debug!("Using Ollama for reasoning: {}", self.default_model);
                ollama.complete(&self.default_model, prompt, Some(0.8)).await
            }

            // No backend available
            _ => anyhow::bail!("No LLM backend available for intent: {:?}", intent),
        }
    }

    /// Check if Ollama is available and healthy
    pub async fn check_ollama_health(&self) -> bool {
        if let Some(ollama) = &self.ollama_client {
            ollama.health_check().await.unwrap_or(false)
        } else {
            false
        }
    }

    /// List available Ollama models
    pub async fn list_ollama_models(&self) -> anyhow::Result<Vec<String>> {
        if let Some(ollama) = &self.ollama_client {
            let models = ollama.list_models().await?;
            Ok(models.into_iter().map(|m| m.name).collect())
        } else {
            Ok(vec![])
        }
    }

    /// Get the primary provider name
    pub fn primary_provider(&self) -> &str {
        &self.primary_provider
    }

    /// Check if Omen is enabled
    pub fn has_omen(&self) -> bool {
        self.omen_client.is_some()
    }

    /// Check if Ollama is enabled
    pub fn has_ollama(&self) -> bool {
        self.ollama_client.is_some()
    }
}
