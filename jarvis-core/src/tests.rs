#[cfg(test)]
mod tests {
    use crate::{Config, LLMRouter, MemoryStore, JarvisResult};
    use crate::llm::LLMProvider;
    use tokio;

    #[tokio::test]
    async fn test_config_creation() -> JarvisResult<()> {
        let config = Config::default();
        
        assert_eq!(config.llm.primary_provider, "ollama");
        assert_eq!(config.llm.ollama_url, "http://localhost:11434");
        assert_eq!(config.llm.context_window, 8192);
        assert!(config.llm.default_model.is_some());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_memory_store_creation() -> JarvisResult<()> {
        let temp_db = tempfile::NamedTempFile::new().unwrap();
        let db_path = temp_db.path().to_str().unwrap();
        
        let memory = MemoryStore::new(db_path).await.map_err(|e| crate::error::JarvisError::Database(e.to_string()))?;
        
        // Test basic connection with an actual available method
        let tasks = memory.get_recent_tasks(10).await.map_err(|e| crate::error::JarvisError::Database(e.to_string()))?;
        assert!(tasks.is_empty()); // Should be empty initially
        
        Ok(())
    }

    #[tokio::test]
    async fn test_llm_router_creation() -> JarvisResult<()> {
        let config = Config::default();
        
        // This should work with Ollama as default
        match LLMRouter::new(&config).await {
            Ok(_router) => {
                // Router created successfully
            }
            Err(e) => {
                // Expected if Ollama not running - that's fine for tests
                println!("LLM Router creation failed (expected if Ollama not running): {}", e);
            }
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_openai_provider_creation() -> JarvisResult<()> {
        let mut config = Config::default();
        config.llm.primary_provider = "openai".to_string();
        config.llm.openai_api_key = Some("test-key".to_string());
        
        match LLMRouter::new(&config).await {
            Ok(_router) => {
                // Router created successfully
            }
            Err(e) => {
                // Expected without real API key
                assert!(e.to_string().contains("OpenAI"));
            }
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_claude_provider_creation() -> JarvisResult<()> {
        let mut config = Config::default();
        config.llm.primary_provider = "claude".to_string();
        config.llm.claude_api_key = Some("test-key".to_string());
        
        match LLMRouter::new(&config).await {
            Ok(_router) => {
                // Router created successfully
            }
            Err(e) => {
                // Expected without real API key
                assert!(e.to_string().contains("Claude"));
            }
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_error_handling() {
        use crate::error::{JarvisError, ErrorContext};
        
        // Test error creation
        let config_error = JarvisError::Config("Test config error".to_string());
        assert!(config_error.to_string().contains("Configuration error"));
        
        // Test error context
        let result: Result<(), std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound, 
            "File not found"
        ));
        
        let jarvis_result = result.with_system_context("Loading configuration file");
        assert!(jarvis_result.is_err());
        
        if let Err(e) = jarvis_result {
            assert!(e.to_string().contains("System error"));
            assert!(e.to_string().contains("Loading configuration file"));
        }
    }

    // Integration test helpers
    pub async fn setup_test_environment() -> (Config, String) {
        let temp_db = tempfile::NamedTempFile::new().unwrap();
        let db_path = temp_db.path().to_str().unwrap().to_string();
        
        let mut config = Config::default();
        config.database_path = db_path.clone();
        
        (config, db_path)
    }

    pub async fn create_test_memory_store() -> JarvisResult<MemoryStore> {
        let temp_db = tempfile::NamedTempFile::new().unwrap();
        let db_path = temp_db.path().to_str().unwrap();
        
        MemoryStore::new(db_path).await.map_err(|e| crate::error::JarvisError::Database(e.to_string()))
    }

    // Mock LLM provider for testing
    #[cfg(test)]
    pub struct MockLLMProvider {
        pub responses: Vec<String>,
        pub call_count: std::sync::Arc<std::sync::Mutex<usize>>,
    }

    #[cfg(test)]
    impl MockLLMProvider {
        pub fn new(responses: Vec<String>) -> Self {
            Self {
                responses,
                call_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
            }
        }
    }

    #[cfg(test)]
    #[async_trait::async_trait]
    impl LLMProvider for MockLLMProvider {
        async fn generate(&self, _prompt: &str, _context: Option<&str>) -> anyhow::Result<String> {
            let mut count = self.call_count.lock().unwrap();
            let response = self.responses.get(*count).unwrap_or(&"Default response".to_string()).clone();
            *count += 1;
            Ok(response)
        }

        async fn generate_stream(&self, _prompt: &str, _context: Option<&str>) -> anyhow::Result<Box<dyn futures::Stream<Item = anyhow::Result<String>> + Unpin>> {
            use futures::stream;
            let response = "Streaming response chunk".to_string();
            let stream = stream::iter(vec![Ok(response)]);
            Ok(Box::new(Box::pin(stream)))
        }

        fn model_name(&self) -> &str {
            "mock-model"
        }

        fn context_window(&self) -> usize {
            4096
        }
    }
}