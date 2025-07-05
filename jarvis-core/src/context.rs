use anyhow::Result;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::types::{Message, MessageRole, Conversation, Environment};
use crate::memory::MemoryStore;
// use jarvis_shell::Environment; // Removed to avoid circular dependency

/// Intelligent context manager for conversation flow
pub struct ContextManager {
    memory: MemoryStore,
    max_context_tokens: usize,
    context_relevance_threshold: f32,
}

/// Context-aware conversation state
#[derive(Clone, Debug)]
pub struct ConversationContext {
    pub conversation_id: Uuid,
    pub title: String,
    pub recent_messages: Vec<Message>,
    pub system_context: String,
    pub relevant_history: Vec<Message>,
    pub environment_context: String,
    pub context_summary: Option<String>,
    pub token_count: usize,
}

/// Context relevance scoring
#[derive(Clone, Debug)]
pub struct ContextRelevance {
    pub message_id: Uuid,
    pub relevance_score: f32,
    pub recency_score: f32,
    pub combined_score: f32,
    pub tags: Vec<String>,
}

impl ContextManager {
    pub fn new(memory: MemoryStore, max_context_tokens: usize) -> Self {
        Self {
            memory,
            max_context_tokens,
            context_relevance_threshold: 0.3,
        }
    }

    /// Get intelligent context for a conversation
    pub async fn get_conversation_context(
        &self,
        conversation_id: Uuid,
        environment: &Environment,
        query: Option<&str>,
    ) -> Result<ConversationContext> {
        // Get recent messages (last 20)
        let recent_messages = self.memory.get_conversation_messages(conversation_id)
            .await?
            .into_iter()
            .rev()
            .take(20)
            .collect::<Vec<_>>();

        // Build system context from environment
        let system_context = self.build_system_context(environment).await?;
        
        // Build environment context
        let environment_context = self.build_environment_context(environment).await?;

        // Get relevant historical context if query provided
        let relevant_history = if let Some(q) = query {
            self.find_relevant_messages(conversation_id, q, 10).await?
        } else {
            Vec::new()
        };

        // Get conversation metadata
        let conversation = self.memory.get_conversation(conversation_id)
            .await?
            .unwrap_or_else(|| Conversation {
                id: conversation_id,
                title: "New Conversation".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });

        // Calculate estimated token count
        let token_count = self.estimate_token_count(&recent_messages, &relevant_history, &system_context);

        // Generate context summary if too large
        let context_summary = if token_count > self.max_context_tokens {
            Some(self.generate_context_summary(&recent_messages, &relevant_history).await?)
        } else {
            None
        };

        Ok(ConversationContext {
            conversation_id,
            title: conversation.title,
            recent_messages,
            system_context,
            relevant_history,
            environment_context,
            context_summary,
            token_count,
        })
    }

    /// Build system context from environment
    async fn build_system_context(&self, environment: &Environment) -> Result<String> {
        let mut context = Vec::new();

        // System information
        context.push(format!("System: {} {}", environment.os_info.os_type, environment.os_info.version));
        context.push(format!("Hostname: {}", environment.os_info.hostname));
        context.push(format!("Architecture: {}", environment.os_info.arch));

        // Git context if available
        if let Some(ref git) = environment.git_context {
            context.push(format!("Git Repository: {}", git.repository_name));
            context.push(format!("Current Branch: {}", git.current_branch));
            if git.is_dirty {
                context.push("Working Directory: Has uncommitted changes".to_string());
            }
        }

        // Working directory
        context.push(format!("Working Directory: {}", environment.current_dir.display()));

        // Load averages
        context.push(format!(
            "Load Averages: {:.2}, {:.2}, {:.2}",
            environment.system_stats.load_avg_1min,
            environment.system_stats.load_avg_5min,
            environment.system_stats.load_avg_15min
        ));

        Ok(context.join("\\n"))
    }

    /// Build environment-specific context
    async fn build_environment_context(&self, environment: &Environment) -> Result<String> {
        let mut context = Vec::new();

        // Package manager context
        if environment.arch_info.is_arch_linux {
            context.push(format!("Package Manager: {}", environment.arch_info.aur_helper));
        }

        // Development environment
        if let Some(ref git) = environment.git_context {
            context.push(format!("Repository: {}", git.repository_name));
            
            // Recent commits (if available)
            if !git.recent_commits.is_empty() {
                context.push("Recent commits:".to_string());
                for commit in git.recent_commits.iter().take(3) {
                    context.push(format!("  {} - {}", &commit.hash[..8], commit.message));
                }
            }
        }

        // Current processes (top CPU consumers)
        if !environment.system_stats.top_processes.is_empty() {
            context.push("Top processes:".to_string());
            for process in environment.system_stats.top_processes.iter().take(5) {
                context.push(format!("  {} ({:.1}% CPU)", process.name, process.cpu_percent));
            }
        }

        Ok(context.join("\\n"))
    }

    /// Find relevant messages based on semantic similarity (simplified)
    async fn find_relevant_messages(
        &self,
        conversation_id: Uuid,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Message>> {
        // For now, use simple keyword matching
        // TODO: Implement vector embeddings for semantic search when ZQLite is available
        let all_messages = self.memory.get_conversation_messages(conversation_id).await?;
        
        let query_words: Vec<&str> = query.to_lowercase().split_whitespace().collect();
        
        let mut scored_messages: Vec<(Message, f32)> = all_messages
            .into_iter()
            .map(|msg| {
                let content_lower = msg.content.to_lowercase();
                let score = query_words.iter()
                    .map(|word| {
                        if content_lower.contains(word) { 1.0 } else { 0.0 }
                    })
                    .sum::<f32>() / query_words.len() as f32;
                (msg, score)
            })
            .filter(|(_, score)| *score > self.context_relevance_threshold)
            .collect();

        // Sort by relevance score
        scored_messages.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        Ok(scored_messages.into_iter()
            .take(limit)
            .map(|(msg, _)| msg)
            .collect())
    }

    /// Estimate token count for context
    fn estimate_token_count(
        &self,
        recent_messages: &[Message],
        relevant_history: &[Message],
        system_context: &str,
    ) -> usize {
        let recent_tokens: usize = recent_messages.iter()
            .map(|msg| msg.content.len() / 4) // Rough estimation: 4 chars per token
            .sum();
        
        let history_tokens: usize = relevant_history.iter()
            .map(|msg| msg.content.len() / 4)
            .sum();
        
        let system_tokens = system_context.len() / 4;
        
        recent_tokens + history_tokens + system_tokens
    }

    /// Generate a summary of context when it's too large
    async fn generate_context_summary(
        &self,
        recent_messages: &[Message],
        relevant_history: &[Message],
    ) -> Result<String> {
        let mut summary = Vec::new();

        // Summarize conversation flow
        if !recent_messages.is_empty() {
            summary.push("Recent conversation flow:".to_string());
            
            // Group messages by type
            let user_messages: Vec<_> = recent_messages.iter()
                .filter(|msg| msg.role == MessageRole::User)
                .collect();
            
            let assistant_messages: Vec<_> = recent_messages.iter()
                .filter(|msg| msg.role == MessageRole::Assistant)
                .collect();

            summary.push(format!("- {} user queries", user_messages.len()));
            summary.push(format!("- {} assistant responses", assistant_messages.len()));
            
            // Extract key topics from messages
            let topics = self.extract_key_topics(recent_messages);
            if !topics.is_empty() {
                summary.push(format!("- Key topics: {}", topics.join(", ")));
            }
        }

        // Summarize relevant history
        if !relevant_history.is_empty() {
            summary.push(format!("Relevant historical context from {} messages", relevant_history.len()));
        }

        Ok(summary.join("\\n"))
    }

    /// Extract key topics from messages (simplified keyword extraction)
    fn extract_key_topics(&self, messages: &[Message]) -> Vec<String> {
        use std::collections::HashMap;
        
        let mut word_counts: HashMap<String, usize> = HashMap::new();
        let stop_words = ["the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by", "is", "are", "was", "were", "be", "been", "have", "has", "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "can", "a", "an", "this", "that", "these", "those", "i", "you", "he", "she", "it", "we", "they", "me", "him", "her", "us", "them"];
        
        for message in messages {
            let content_lower = message.content.to_lowercase();
            let words: Vec<&str> = content_lower
                .split_whitespace()
                .filter(|word| word.len() > 3 && !stop_words.contains(word))
                .collect();
            
            for word in words {
                *word_counts.entry(word.to_string()).or_insert(0) += 1;
            }
        }
        
        // Get top 5 most frequent words
        let mut word_vec: Vec<(String, usize)> = word_counts.into_iter().collect();
        word_vec.sort_by(|a, b| b.1.cmp(&a.1));
        
        word_vec.into_iter()
            .take(5)
            .filter(|(_, count)| *count > 1)
            .map(|(word, _)| word)
            .collect()
    }

    /// Optimize context for LLM consumption
    pub fn optimize_context_for_llm(&self, context: &ConversationContext) -> String {
        let mut optimized = Vec::new();

        // System context first
        optimized.push("SYSTEM CONTEXT:".to_string());
        optimized.push(context.system_context.clone());
        optimized.push("".to_string());

        // Environment context
        if !context.environment_context.is_empty() {
            optimized.push("ENVIRONMENT:".to_string());
            optimized.push(context.environment_context.clone());
            optimized.push("".to_string());
        }

        // Context summary if available
        if let Some(ref summary) = context.context_summary {
            optimized.push("CONTEXT SUMMARY:".to_string());
            optimized.push(summary.clone());
            optimized.push("".to_string());
        }

        // Relevant history
        if !context.relevant_history.is_empty() {
            optimized.push("RELEVANT CONTEXT:".to_string());
            for msg in &context.relevant_history {
                optimized.push(format!("{}: {}", msg.role, msg.content));
            }
            optimized.push("".to_string());
        }

        // Recent conversation
        if !context.recent_messages.is_empty() {
            optimized.push("RECENT CONVERSATION:".to_string());
            for msg in &context.recent_messages {
                optimized.push(format!("{}: {}", msg.role, msg.content));
            }
        }

        optimized.join("\\n")
    }
}