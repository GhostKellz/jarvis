use anyhow::Result;
use jarvis_core::{LLMRouter, MemoryStore, types::*};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AIIntegration {
    llm: Arc<LLMRouter>,
    memory: Arc<MemoryStore>,
    current_conversation: Arc<RwLock<Option<Conversation>>>,
}

impl AIIntegration {
    pub fn new(llm: Arc<LLMRouter>, memory: Arc<MemoryStore>) -> Self {
        Self {
            llm,
            memory,
            current_conversation: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start_conversation(&self, title: &str) -> Result<()> {
        let conversation = self.memory.create_conversation(title).await?;
        *self.current_conversation.write().await = Some(conversation);
        Ok(())
    }

    pub async fn send_message(&self, content: &str, context: Option<&str>) -> Result<String> {
        // Ensure we have a conversation
        if self.current_conversation.read().await.is_none() {
            self.start_conversation("Neovim Session").await?;
        }

        let conversation_id = self.current_conversation
            .read()
            .await
            .as_ref()
            .unwrap()
            .id;

        // Add user message
        let user_metadata = MessageMetadata {
            model_used: None,
            tokens_used: None,
            execution_time_ms: None,
            system_context: None,
        };

        self.memory.add_message(
            conversation_id,
            MessageRole::User,
            content,
            user_metadata,
        ).await?;

        // Generate AI response
        let start_time = std::time::Instant::now();
        let response = if let Some(ctx) = context {
            self.llm.generate_with_system_context(content, ctx).await?
        } else {
            self.llm.generate(content, None).await?
        };
        let execution_time = start_time.elapsed().as_millis() as u64;

        // Add assistant message
        let assistant_metadata = MessageMetadata {
            model_used: Some("jarvis".to_string()),
            tokens_used: None,
            execution_time_ms: Some(execution_time),
            system_context: None,
        };

        self.memory.add_message(
            conversation_id,
            MessageRole::Assistant,
            &response,
            assistant_metadata,
        ).await?;

        Ok(response)
    }

    pub async fn explain_code(&self, code: &str, language: &str, context: &str) -> Result<String> {
        let prompt = format!(
            "Explain this {} code in detail. Focus on what it does, how it works, and any potential issues:\n\n```{}\n{}\n```\n\nContext: {}",
            language, language, code, context
        );

        self.send_message(&prompt, Some("Code explanation request")).await
    }

    pub async fn suggest_improvements(&self, code: &str, language: &str) -> Result<String> {
        let prompt = format!(
            "Suggest improvements for this {} code. Focus on performance, readability, best practices, and potential bugs:\n\n```{}\n{}\n```",
            language, language, code
        );

        self.send_message(&prompt, Some("Code improvement request")).await
    }

    pub async fn fix_errors(&self, code: &str, errors: &[String], language: &str) -> Result<String> {
        let errors_text = errors.join("\n");
        let prompt = format!(
            "Fix the following errors in this {} code:\n\nErrors:\n{}\n\nCode:\n```{}\n{}\n```\n\nProvide the corrected code with explanations.",
            language, errors_text, language, code
        );

        self.send_message(&prompt, Some("Error fixing request")).await
    }

    pub async fn generate_code(&self, description: &str, language: &str, context: &str) -> Result<String> {
        let prompt = format!(
            "Generate {} code for: {}\n\nContext: {}\n\nProvide clean, well-commented code that follows best practices.",
            language, description, context
        );

        self.send_message(&prompt, Some("Code generation request")).await
    }

    pub async fn refactor_code(&self, code: &str, language: &str, goal: &str) -> Result<String> {
        let prompt = format!(
            "Refactor this {} code to {}:\n\n```{}\n{}\n```\n\nProvide the refactored code with explanations of changes.",
            language, goal, language, code
        );

        self.send_message(&prompt, Some("Code refactoring request")).await
    }

    pub async fn add_comments(&self, code: &str, language: &str) -> Result<String> {
        let prompt = format!(
            "Add comprehensive comments to this {} code. Explain complex logic, function purposes, and parameter meanings:\n\n```{}\n{}\n```",
            language, language, code
        );

        self.send_message(&prompt, Some("Code documentation request")).await
    }

    pub async fn convert_language(&self, code: &str, from_lang: &str, to_lang: &str) -> Result<String> {
        let prompt = format!(
            "Convert this {} code to {}. Maintain the same functionality and logic:\n\n```{}\n{}\n```",
            from_lang, to_lang, from_lang, code
        );

        self.send_message(&prompt, Some("Language conversion request")).await
    }

    pub async fn analyze_performance(&self, code: &str, language: &str) -> Result<String> {
        let prompt = format!(
            "Analyze the performance characteristics of this {} code. Identify bottlenecks, suggest optimizations, and estimate complexity:\n\n```{}\n{}\n```",
            language, language, code
        );

        self.send_message(&prompt, Some("Performance analysis request")).await
    }

    pub async fn generate_tests(&self, code: &str, language: &str) -> Result<String> {
        let prompt = format!(
            "Generate comprehensive unit tests for this {} code. Include edge cases, error conditions, and normal usage:\n\n```{}\n{}\n```",
            language, language, code
        );

        self.send_message(&prompt, Some("Test generation request")).await
    }

    pub async fn system_prompt(&self, query: &str, system_info: &str) -> Result<String> {
        let prompt = format!(
            "System query: {}\n\nSystem context:\n{}",
            query, system_info
        );

        self.send_message(&prompt, Some("System administration request")).await
    }
}
