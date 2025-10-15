//! Natural Language Processing for Jarvis
//!
//! Parses natural language commands and routes them to appropriate tools/actions.

use crate::llm::{Intent, LLMRouter};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Parsed command with detected intent and parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    pub intent: CommandIntent,
    pub tool: String,
    pub action: String,
    pub parameters: serde_json::Value,
    pub original_query: String,
    pub confidence: f32,
}

/// High-level command intent categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandIntent {
    SystemStatus,
    PackageManagement,
    DockerManagement,
    VMManagement,
    Troubleshooting,
    Information,
    Unknown,
}

/// Natural language command parser
pub struct CommandParser {
    llm_router: Option<LLMRouter>,
}

impl CommandParser {
    pub fn new(llm_router: Option<LLMRouter>) -> Self {
        Self { llm_router }
    }

    /// Parse a natural language command
    pub async fn parse(&self, query: &str) -> Result<ParsedCommand> {
        // First try rule-based parsing (fast, deterministic)
        if let Some(cmd) = self.parse_rules(query) {
            return Ok(cmd);
        }

        // Fall back to LLM-based parsing (smart, context-aware)
        if let Some(router) = &self.llm_router {
            self.parse_llm(query, router).await
        } else {
            // No LLM available, return best-effort parse
            Ok(ParsedCommand {
                intent: CommandIntent::Unknown,
                tool: "unknown".to_string(),
                action: "unknown".to_string(),
                parameters: serde_json::json!({"query": query}),
                original_query: query.to_string(),
                confidence: 0.0,
            })
        }
    }

    /// Rule-based parsing for common patterns
    fn parse_rules(&self, query: &str) -> Option<ParsedCommand> {
        let lower = query.to_lowercase();

        // System status patterns
        if lower.contains("system status")
            || lower.contains("show system")
            || lower.contains("check system")
            || (lower.contains("cpu") && lower.contains("memory"))
        {
            return Some(ParsedCommand {
                intent: CommandIntent::SystemStatus,
                tool: "jarvis_system_status".to_string(),
                action: "check".to_string(),
                parameters: serde_json::json!({
                    "verbose": lower.contains("verbose") || lower.contains("detailed")
                }),
                original_query: query.to_string(),
                confidence: 0.9,
            });
        }

        // Package search
        if lower.starts_with("search for") || lower.starts_with("find package") {
            let package = extract_package_name(&lower);
            return Some(ParsedCommand {
                intent: CommandIntent::PackageManagement,
                tool: "jarvis_package_manager".to_string(),
                action: "search".to_string(),
                parameters: serde_json::json!({
                    "action": "search",
                    "package": package,
                    "manager": "pacman"
                }),
                original_query: query.to_string(),
                confidence: 0.85,
            });
        }

        // Package install
        if lower.starts_with("install") || lower.contains("install package") {
            let package = extract_package_name(&lower);
            return Some(ParsedCommand {
                intent: CommandIntent::PackageManagement,
                tool: "jarvis_package_manager".to_string(),
                action: "install".to_string(),
                parameters: serde_json::json!({
                    "action": "install",
                    "package": package,
                    "manager": "pacman",
                    "confirm": false  // Always require manual confirmation
                }),
                original_query: query.to_string(),
                confidence: 0.9,
            });
        }

        // Package updates
        if lower.contains("check updates")
            || lower.contains("list updates")
            || lower.contains("available updates")
        {
            return Some(ParsedCommand {
                intent: CommandIntent::PackageManagement,
                tool: "jarvis_package_manager".to_string(),
                action: "list-updates".to_string(),
                parameters: serde_json::json!({
                    "action": "list-updates"
                }),
                original_query: query.to_string(),
                confidence: 0.9,
            });
        }

        // Docker list
        if lower.contains("list containers")
            || lower.contains("show containers")
            || lower.contains("docker ps")
        {
            return Some(ParsedCommand {
                intent: CommandIntent::DockerManagement,
                tool: "jarvis_docker".to_string(),
                action: "list".to_string(),
                parameters: serde_json::json!({
                    "action": "list"
                }),
                original_query: query.to_string(),
                confidence: 0.95,
            });
        }

        // Docker logs
        if lower.contains("logs") && (lower.contains("container") || lower.contains("docker")) {
            let container = extract_container_name(&lower);
            return Some(ParsedCommand {
                intent: CommandIntent::DockerManagement,
                tool: "jarvis_docker".to_string(),
                action: "logs".to_string(),
                parameters: serde_json::json!({
                    "action": "logs",
                    "target": container,
                    "tail": 50
                }),
                original_query: query.to_string(),
                confidence: 0.85,
            });
        }

        // Docker diagnose/troubleshoot
        if (lower.contains("diagnose") || lower.contains("troubleshoot") || lower.contains("debug"))
            && lower.contains("container")
        {
            let container = extract_container_name(&lower);
            return Some(ParsedCommand {
                intent: CommandIntent::Troubleshooting,
                tool: "jarvis_docker".to_string(),
                action: "diagnose".to_string(),
                parameters: serde_json::json!({
                    "action": "diagnose",
                    "target": container,
                    "llm_assist": true
                }),
                original_query: query.to_string(),
                confidence: 0.9,
            });
        }

        // Docker health
        if lower.contains("docker health") || lower.contains("container health") {
            return Some(ParsedCommand {
                intent: CommandIntent::DockerManagement,
                tool: "jarvis_docker".to_string(),
                action: "health".to_string(),
                parameters: serde_json::json!({
                    "action": "health",
                    "llm_assist": true
                }),
                original_query: query.to_string(),
                confidence: 0.9,
            });
        }

        // VM list
        if lower.contains("list vms") || lower.contains("show vms") || lower.contains("virtual machines") {
            return Some(ParsedCommand {
                intent: CommandIntent::VMManagement,
                tool: "jarvis_docker".to_string(),
                action: "vm-list".to_string(),
                parameters: serde_json::json!({
                    "action": "vm-list"
                }),
                original_query: query.to_string(),
                confidence: 0.9,
            });
        }

        None
    }

    /// LLM-based parsing for complex queries
    async fn parse_llm(&self, query: &str, router: &LLMRouter) -> Result<ParsedCommand> {
        let prompt = format!(
            r#"Parse this system administration command and return JSON:

Command: "{}"

Available tools:
- jarvis_system_status: Check CPU, memory, disk usage
- jarvis_package_manager: Search, install, remove, update packages
- jarvis_docker: Manage Docker containers (list, logs, start, stop, diagnose)
- jarvis_docker: Manage KVM VMs (vm-list, vm-start, vm-stop, vm-info)

Return JSON in this format:
{{
  "tool": "tool_name",
  "action": "action_name",
  "parameters": {{}},
  "intent": "SystemStatus|PackageManagement|DockerManagement|VMManagement|Troubleshooting|Information",
  "confidence": 0.0-1.0
}}

Examples:
- "show me system status" → {{"tool": "jarvis_system_status", "action": "check", "parameters": {{"verbose": false}}, "intent": "SystemStatus", "confidence": 0.95}}
- "install docker" → {{"tool": "jarvis_package_manager", "action": "install", "parameters": {{"action": "install", "package": "docker", "confirm": false}}, "intent": "PackageManagement", "confidence": 0.9}}
- "why is ollama using so much memory?" → {{"tool": "jarvis_docker", "action": "diagnose", "parameters": {{"action": "diagnose", "target": "ollama", "llm_assist": true}}, "intent": "Troubleshooting", "confidence": 0.85}}

Return only valid JSON, no explanation."#,
            query
        );

        let response = router.generate_with_intent(&prompt, Intent::System).await?;

        // Try to extract JSON from response
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                &response
            }
        } else {
            &response
        };

        match serde_json::from_str::<serde_json::Value>(json_str) {
            Ok(parsed) => {
                let intent_str = parsed.get("intent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                let intent = match intent_str {
                    "SystemStatus" => CommandIntent::SystemStatus,
                    "PackageManagement" => CommandIntent::PackageManagement,
                    "DockerManagement" => CommandIntent::DockerManagement,
                    "VMManagement" => CommandIntent::VMManagement,
                    "Troubleshooting" => CommandIntent::Troubleshooting,
                    "Information" => CommandIntent::Information,
                    _ => CommandIntent::Unknown,
                };

                Ok(ParsedCommand {
                    intent,
                    tool: parsed.get("tool")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    action: parsed.get("action")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    parameters: parsed.get("parameters")
                        .cloned()
                        .unwrap_or(serde_json::json!({})),
                    original_query: query.to_string(),
                    confidence: parsed.get("confidence")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.5) as f32,
                })
            }
            Err(e) => {
                tracing::warn!("Failed to parse LLM response as JSON: {}", e);
                tracing::debug!("LLM response: {}", response);

                // Return unknown command
                Ok(ParsedCommand {
                    intent: CommandIntent::Unknown,
                    tool: "unknown".to_string(),
                    action: "unknown".to_string(),
                    parameters: serde_json::json!({"query": query, "llm_response": response}),
                    original_query: query.to_string(),
                    confidence: 0.1,
                })
            }
        }
    }

    /// Get suggested commands based on intent
    pub fn suggest_commands(&self, intent: CommandIntent) -> Vec<String> {
        match intent {
            CommandIntent::SystemStatus => vec![
                "show system status".to_string(),
                "check system resources".to_string(),
                "show detailed system status".to_string(),
            ],
            CommandIntent::PackageManagement => vec![
                "search for docker".to_string(),
                "install neovim".to_string(),
                "check for updates".to_string(),
                "list installed packages".to_string(),
            ],
            CommandIntent::DockerManagement => vec![
                "list containers".to_string(),
                "show logs for ollama".to_string(),
                "check docker health".to_string(),
                "restart ollama container".to_string(),
            ],
            CommandIntent::VMManagement => vec![
                "list vms".to_string(),
                "start vm windows11".to_string(),
                "show vm info for ubuntu-server".to_string(),
            ],
            CommandIntent::Troubleshooting => vec![
                "diagnose ollama container".to_string(),
                "why is my container failing?".to_string(),
                "troubleshoot high memory usage".to_string(),
            ],
            CommandIntent::Information => vec![
                "what models are available?".to_string(),
                "show jarvis version".to_string(),
                "help with docker commands".to_string(),
            ],
            CommandIntent::Unknown => vec![
                "Try: 'show system status'".to_string(),
                "Try: 'list containers'".to_string(),
                "Try: 'check for updates'".to_string(),
            ],
        }
    }
}

// Helper functions

fn extract_package_name(query: &str) -> String {
    // Remove common words
    let cleaned = query
        .replace("install", "")
        .replace("search for", "")
        .replace("package", "")
        .replace("find", "")
        .trim()
        .to_string();

    // Take first word as package name
    cleaned
        .split_whitespace()
        .next()
        .unwrap_or("unknown")
        .to_string()
}

fn extract_container_name(query: &str) -> String {
    // Look for common patterns
    if let Some(idx) = query.find("container") {
        let after = &query[idx + "container".len()..];
        return after
            .trim()
            .split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_string();
    }

    if let Some(idx) = query.find("for") {
        let after = &query[idx + "for".len()..];
        return after
            .trim()
            .split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_string();
    }

    // Look for quoted names
    if let Some(start) = query.find('"') {
        if let Some(end) = query[start + 1..].find('"') {
            return query[start + 1..start + 1 + end].to_string();
        }
    }

    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_status_parsing() {
        let parser = CommandParser::new(None);

        let cmd = parser.parse_rules("show system status").unwrap();
        assert_eq!(cmd.intent, CommandIntent::SystemStatus);
        assert_eq!(cmd.tool, "jarvis_system_status");
    }

    #[test]
    fn test_package_search_parsing() {
        let parser = CommandParser::new(None);

        let cmd = parser.parse_rules("search for docker").unwrap();
        assert_eq!(cmd.intent, CommandIntent::PackageManagement);
        assert_eq!(cmd.action, "search");
        assert_eq!(cmd.parameters["package"], "docker");
    }

    #[test]
    fn test_docker_list_parsing() {
        let parser = CommandParser::new(None);

        let cmd = parser.parse_rules("list containers").unwrap();
        assert_eq!(cmd.intent, CommandIntent::DockerManagement);
        assert_eq!(cmd.action, "list");
    }

    #[test]
    fn test_container_name_extraction() {
        assert_eq!(extract_container_name("logs for ollama"), "ollama");
        assert_eq!(extract_container_name("diagnose container nginx"), "nginx");
        assert_eq!(extract_container_name("check \"my-app\" logs"), "my-app");
    }
}
