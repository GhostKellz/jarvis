use super::{GhostFlowNode, NodeHealth, HealthStatus};
use crate::{Result, WorkflowContext, NodeExecutionResult, ExecutionStatus, LLMProviderConfig};
use async_trait::async_trait;
use jarvis_core::{LLMRouter, Config as JarvisConfig};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Smart LLM Router Node that leverages Jarvis's intelligent provider selection
pub struct LLMRouterNode {
    llm_router: Arc<RwLock<Option<LLMRouter>>>,
    config: LLMRouterConfig,
    health: Arc<RwLock<NodeHealth>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRouterConfig {
    pub providers: Vec<LLMProviderConfig>,
    pub enable_caching: bool,
    pub enable_streaming: bool,
    pub cost_optimization: bool,
    pub auto_fallback: bool,
    pub context_enhancement: bool,
    pub max_retries: u32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRouterInput {
    pub prompt: String,
    pub system_context: Option<String>,
    pub model_preference: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRouterOutput {
    pub response: String,
    pub provider_used: String,
    pub model_used: String,
    pub tokens_consumed: u64,
    pub execution_time_ms: u64,
    pub cost_estimate: f64,
    pub cache_hit: bool,
    pub provider_attempts: Vec<ProviderAttempt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAttempt {
    pub provider: String,
    pub success: bool,
    pub error: Option<String>,
    pub latency_ms: u64,
}

impl LLMRouterNode {
    pub fn new() -> Result<Self> {
        Ok(Self {
            llm_router: Arc::new(RwLock::new(None)),
            config: LLMRouterConfig::default(),
            health: Arc::new(RwLock::new(NodeHealth {
                status: HealthStatus::Unknown,
                message: None,
                last_execution: None,
                error_count: 0,
                success_rate: 0.0,
            })),
        })
    }

    async fn initialize_llm_router(&self, config: &HashMap<String, serde_json::Value>) -> Result<()> {
        // Create Jarvis config from node config
        let jarvis_config = self.create_jarvis_config(config)?;
        let router = LLMRouter::new(&jarvis_config).await?;
        
        *self.llm_router.write().await = Some(router);
        Ok(())
    }

    fn create_jarvis_config(&self, config: &HashMap<String, serde_json::Value>) -> Result<JarvisConfig> {
        // Convert GhostFlow config to Jarvis config format
        let mut jarvis_config = JarvisConfig::default();
        
        if let Some(providers) = config.get("providers") {
            if let Ok(provider_configs) = serde_json::from_value::<Vec<LLMProviderConfig>>(providers.clone()) {
                // Set primary provider
                if let Some(primary) = provider_configs.first() {
                    jarvis_config.llm.primary_provider = primary.provider.clone();
                    jarvis_config.llm.default_model = Some(primary.model.clone());
                    jarvis_config.llm.context_window = primary.context_window;
                    
                    // Set API keys based on provider
                    match primary.provider.as_str() {
                        "openai" => jarvis_config.llm.openai_api_key = primary.api_key.clone(),
                        "claude" => jarvis_config.llm.claude_api_key = primary.api_key.clone(),
                        "ollama" => {
                            if let Some(url) = &primary.base_url {
                                jarvis_config.llm.ollama_url = url.clone();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        Ok(jarvis_config)
    }

    async fn execute_llm_request(&self, input: &LLMRouterInput) -> Result<LLMRouterOutput> {
        let start_time = Instant::now();
        let mut attempts = Vec::new();
        
        let router_guard = self.llm_router.read().await;
        let router = router_guard.as_ref()
            .ok_or_else(|| crate::GhostFlowError::NodeExecution("LLM Router not initialized".to_string()))?;

        // Try generating response
        let response = if input.stream.unwrap_or(false) && self.config.enable_streaming {
            // For streaming, we'd need to handle this differently in a real implementation
            // For now, fall back to regular generation
            router.generate(&input.prompt, input.system_context.as_deref()).await?
        } else {
            router.generate(&input.prompt, input.system_context.as_deref()).await?
        };

        let execution_time = start_time.elapsed().as_millis() as u64;
        let tokens_consumed = self.estimate_tokens(&response);
        let cost_estimate = self.estimate_cost(&response, "primary");

        // Update health metrics
        self.update_health_metrics(true, execution_time).await;

        Ok(LLMRouterOutput {
            response,
            provider_used: "jarvis-router".to_string(), // Would be dynamic in real implementation
            model_used: "auto-selected".to_string(),
            tokens_consumed,
            execution_time_ms: execution_time,
            cost_estimate,
            cache_hit: false, // Would check cache in real implementation
            provider_attempts: attempts,
        })
    }

    fn estimate_tokens(&self, text: &str) -> u64 {
        // Rough estimation: ~4 characters per token
        (text.len() / 4) as u64
    }

    fn estimate_cost(&self, text: &str, provider: &str) -> f64 {
        let tokens = self.estimate_tokens(text) as f64;
        let cost_per_1k = match provider {
            "openai" => 0.002,
            "claude" => 0.008,
            _ => 0.0,
        };
        (tokens / 1000.0) * cost_per_1k
    }

    async fn update_health_metrics(&self, success: bool, execution_time_ms: u64) {
        let mut health = self.health.write().await;
        
        if !success {
            health.error_count += 1;
        }
        
        health.last_execution = Some(chrono::Utc::now());
        health.status = if health.error_count == 0 {
            HealthStatus::Healthy
        } else if health.error_count < 5 {
            HealthStatus::Warning
        } else {
            HealthStatus::Critical
        };
    }
}

#[async_trait]
impl GhostFlowNode for LLMRouterNode {
    fn node_type(&self) -> &'static str {
        "jarvis.llm_router"
    }

    fn display_name(&self) -> &str {
        "Smart LLM Router"
    }

    fn description(&self) -> &str {
        "Intelligent routing to optimal LLM providers with automatic failover, cost optimization, and caching"
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt to send to the LLM",
                    "minLength": 1
                },
                "system_context": {
                    "type": "string",
                    "description": "Optional system context or instructions"
                },
                "model_preference": {
                    "type": "string",
                    "description": "Preferred model or provider",
                    "enum": ["openai", "claude", "ollama", "auto"]
                },
                "temperature": {
                    "type": "number",
                    "description": "Temperature for response generation",
                    "minimum": 0.0,
                    "maximum": 2.0
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens to generate",
                    "minimum": 1,
                    "maximum": 8192
                },
                "stream": {
                    "type": "boolean",
                    "description": "Enable streaming response"
                }
            },
            "required": ["prompt"]
        })
    }

    fn output_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "response": {
                    "type": "string",
                    "description": "The LLM response"
                },
                "provider_used": {
                    "type": "string",
                    "description": "The provider that was used"
                },
                "model_used": {
                    "type": "string",
                    "description": "The specific model that was used"
                },
                "tokens_consumed": {
                    "type": "integer",
                    "description": "Number of tokens consumed"
                },
                "execution_time_ms": {
                    "type": "integer",
                    "description": "Execution time in milliseconds"
                },
                "cost_estimate": {
                    "type": "number",
                    "description": "Estimated cost in USD"
                },
                "cache_hit": {
                    "type": "boolean",
                    "description": "Whether the response was cached"
                }
            }
        })
    }

    fn config_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "providers": {
                    "type": "array",
                    "description": "List of LLM providers to use",
                    "items": {
                        "type": "object",
                        "properties": {
                            "provider": {
                                "type": "string",
                                "enum": ["openai", "claude", "ollama"]
                            },
                            "model": {
                                "type": "string",
                                "description": "Model name"
                            },
                            "api_key": {
                                "type": "string",
                                "description": "API key for the provider"
                            },
                            "base_url": {
                                "type": "string",
                                "description": "Base URL for the provider"
                            },
                            "priority": {
                                "type": "integer",
                                "description": "Priority (1 = highest)"
                            }
                        },
                        "required": ["provider", "model"]
                    }
                },
                "enable_caching": {
                    "type": "boolean",
                    "description": "Enable response caching",
                    "default": true
                },
                "enable_streaming": {
                    "type": "boolean",
                    "description": "Enable streaming responses",
                    "default": false
                },
                "cost_optimization": {
                    "type": "boolean",
                    "description": "Enable cost optimization",
                    "default": true
                },
                "auto_fallback": {
                    "type": "boolean",
                    "description": "Automatically fallback to other providers",
                    "default": true
                },
                "max_retries": {
                    "type": "integer",
                    "description": "Maximum retry attempts",
                    "default": 3,
                    "minimum": 0,
                    "maximum": 10
                }
            }
        })
    }

    async fn execute(
        &self,
        context: &mut WorkflowContext,
        inputs: HashMap<String, serde_json::Value>,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<crate::NodeExecutionResult> {
        let start_time = Instant::now();
        
        // Initialize router if needed
        if self.llm_router.read().await.is_none() {
            self.initialize_llm_router(&config).await?;
        }

        // Parse input
        let input: LLMRouterInput = serde_json::from_value(serde_json::Value::Object(
            inputs.into_iter().collect()
        ))?;

        // Execute LLM request
        match self.execute_llm_request(&input).await {
            Ok(output) => {
                // Store result in workflow context for memory
                if let Some(memory_context) = &mut context.memory_context {
                    memory_context.context_entries.push(crate::ContextEntry {
                        id: Uuid::new_v4(),
                        content: format!("Prompt: {}\nResponse: {}", input.prompt, output.response),
                        entry_type: crate::ContextEntryType::AIResponse,
                        timestamp: chrono::Utc::now(),
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("provider".to_string(), json!(output.provider_used));
                            meta.insert("model".to_string(), json!(output.model_used));
                            meta.insert("tokens".to_string(), json!(output.tokens_consumed));
                            meta.insert("cost".to_string(), json!(output.cost_estimate));
                            meta
                        },
                    });
                }

                Ok(crate::NodeExecutionResult {
                    node_id: "llm_router".to_string(),
                    execution_id: context.execution_id,
                    status: ExecutionStatus::Success,
                    output: serde_json::to_value(output)?,
                    error: None,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    metadata: HashMap::new(),
                    next_nodes: vec![],
                })
            }
            Err(e) => {
                self.update_health_metrics(false, start_time.elapsed().as_millis() as u64).await;
                
                Ok(crate::NodeExecutionResult {
                    node_id: "llm_router".to_string(),
                    execution_id: context.execution_id,
                    status: ExecutionStatus::Failure,
                    output: json!({}),
                    error: Some(e.to_string()),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    metadata: HashMap::new(),
                    next_nodes: vec![],
                })
            }
        }
    }

    fn validate_config(&self, config: &HashMap<String, serde_json::Value>) -> Result<()> {
        // Validate that at least one provider is configured
        if let Some(providers) = config.get("providers") {
            if let Ok(provider_configs) = serde_json::from_value::<Vec<LLMProviderConfig>>(providers.clone()) {
                if provider_configs.is_empty() {
                    return Err(crate::GhostFlowError::Config(
                        "At least one LLM provider must be configured".to_string()
                    ));
                }
                
                // Validate each provider has required fields
                for provider in &provider_configs {
                    if provider.provider.is_empty() || provider.model.is_empty() {
                        return Err(crate::GhostFlowError::Config(
                            "Provider and model are required for each LLM provider".to_string()
                        ));
                    }
                }
            }
        } else {
            return Err(crate::GhostFlowError::Config(
                "No providers configured for LLM Router".to_string()
            ));
        }
        
        Ok(())
    }

    async fn health_check(&self) -> NodeHealth {
        self.health.read().await.clone()
    }
}

impl Default for LLMRouterConfig {
    fn default() -> Self {
        Self {
            providers: vec![
                LLMProviderConfig {
                    provider: "ollama".to_string(),
                    model: "llama3.1:8b".to_string(),
                    api_key: None,
                    base_url: Some("http://localhost:11434".to_string()),
                    max_tokens: Some(4096),
                    temperature: Some(0.7),
                    context_window: 8192,
                    cost_per_token: 0.0,
                    priority: 1,
                }
            ],
            enable_caching: true,
            enable_streaming: false,
            cost_optimization: true,
            auto_fallback: true,
            context_enhancement: true,
            max_retries: 3,
            timeout_seconds: 60,
        }
    }
}