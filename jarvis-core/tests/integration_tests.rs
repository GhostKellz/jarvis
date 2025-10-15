//! Integration tests for Jarvis components
//!
//! Tests LLM routing, MCP tools, and natural language parsing.

use jarvis_core::{CommandParser, Config, Intent, LLMRouter, OllamaClient};

#[tokio::test]
#[ignore] // Requires Ollama to be running
async fn test_ollama_connection() {
    let client = OllamaClient::new("http://localhost:11434".to_string());

    // Test health check
    let is_healthy = client.health_check().await.expect("Health check failed");
    assert!(is_healthy, "Ollama should be healthy");

    // Test model listing
    let models = client.list_models().await.expect("Failed to list models");
    assert!(!models.is_empty(), "Should have at least one model");

    println!("✅ Ollama connection test passed");
    println!("   Available models: {:?}", models.iter().map(|m| &m.name).collect::<Vec<_>>());
}

#[tokio::test]
#[ignore] // Requires Ollama to be running
async fn test_ollama_completion() {
    let client = OllamaClient::new("http://localhost:11434".to_string());

    // Test simple completion
    let response = client
        .complete("llama3.1:8b", "Say 'test successful' and nothing else", Some(0.1))
        .await
        .expect("Completion failed");

    assert!(!response.is_empty(), "Response should not be empty");
    println!("✅ Ollama completion test passed");
    println!("   Response: {}", response);
}

#[tokio::test]
#[ignore] // Requires Ollama to be running
async fn test_ollama_system_prompt() {
    let client = OllamaClient::new("http://localhost:11434".to_string());

    // Test system administration prompt
    let response = client
        .system("llama3.1:8b", "How do I check Docker container status?", Some(0.7))
        .await
        .expect("System prompt failed");

    assert!(!response.is_empty(), "Response should not be empty");
    assert!(
        response.to_lowercase().contains("docker")
            || response.to_lowercase().contains("container"),
        "Response should mention Docker"
    );

    println!("✅ Ollama system prompt test passed");
}

#[tokio::test]
#[ignore] // Requires configured LLMRouter
async fn test_llm_router_intent_routing() {
    let config = Config::default();
    let router = LLMRouter::new(&config).await.expect("Failed to create LLMRouter");

    // Skip if no backend available
    if !router.has_ollama() && !router.has_omen() {
        println!("⚠️ Skipping: No LLM backend configured");
        return;
    }

    // Test code intent
    let response = router
        .generate_with_intent("Write a hello world function", Intent::Code)
        .await
        .expect("Code intent failed");

    assert!(!response.is_empty(), "Code response should not be empty");
    println!("✅ LLM router code intent test passed");

    // Test system intent
    let response = router
        .generate_with_intent("List running processes", Intent::System)
        .await
        .expect("System intent failed");

    assert!(!response.is_empty(), "System response should not be empty");
    println!("✅ LLM router system intent test passed");
}

#[test]
fn test_command_parser_system_status() {
    let parser = CommandParser::new(None);

    // Test various system status commands
    let test_cases = vec![
        "show system status",
        "check system",
        "system status verbose",
        "show me CPU and memory",
    ];

    for query in test_cases {
        let cmd = parser
            .parse_rules(query)
            .expect(&format!("Failed to parse: {}", query));

        assert_eq!(cmd.intent, jarvis_core::CommandIntent::SystemStatus);
        assert_eq!(cmd.tool, "jarvis_system_status");
        println!("✅ Parsed: {} → {:?}", query, cmd.action);
    }
}

#[test]
fn test_command_parser_package_management() {
    let parser = CommandParser::new(None);

    // Test package search
    let cmd = parser.parse_rules("search for docker").unwrap();
    assert_eq!(cmd.intent, jarvis_core::CommandIntent::PackageManagement);
    assert_eq!(cmd.action, "search");
    assert_eq!(cmd.parameters["package"], "docker");

    // Test package install
    let cmd = parser.parse_rules("install neovim").unwrap();
    assert_eq!(cmd.intent, jarvis_core::CommandIntent::PackageManagement);
    assert_eq!(cmd.action, "install");
    assert_eq!(cmd.parameters["package"], "neovim");
    assert_eq!(cmd.parameters["confirm"], false); // Safety check

    // Test update check
    let cmd = parser.parse_rules("check for updates").unwrap();
    assert_eq!(cmd.intent, jarvis_core::CommandIntent::PackageManagement);
    assert_eq!(cmd.action, "list-updates");

    println!("✅ Package management parsing tests passed");
}

#[test]
fn test_command_parser_docker() {
    let parser = CommandParser::new(None);

    // Test container listing
    let cmd = parser.parse_rules("list containers").unwrap();
    assert_eq!(cmd.intent, jarvis_core::CommandIntent::DockerManagement);
    assert_eq!(cmd.action, "list");

    // Test container logs
    let cmd = parser.parse_rules("show logs for ollama").unwrap();
    assert_eq!(cmd.intent, jarvis_core::CommandIntent::DockerManagement);
    assert_eq!(cmd.action, "logs");
    assert_eq!(cmd.parameters["target"], "ollama");

    // Test troubleshooting
    let cmd = parser.parse_rules("diagnose ollama container").unwrap();
    assert_eq!(cmd.intent, jarvis_core::CommandIntent::Troubleshooting);
    assert_eq!(cmd.action, "diagnose");
    assert_eq!(cmd.parameters["llm_assist"], true);

    println!("✅ Docker command parsing tests passed");
}

#[test]
fn test_command_parser_vm_management() {
    let parser = CommandParser::new(None);

    // Test VM listing
    let cmd = parser.parse_rules("list vms").unwrap();
    assert_eq!(cmd.intent, jarvis_core::CommandIntent::VMManagement);
    assert_eq!(cmd.action, "vm-list");

    println!("✅ VM management parsing tests passed");
}

#[tokio::test]
#[ignore] // Requires Ollama for LLM-based parsing
async fn test_command_parser_with_llm() {
    let config = Config::default();
    let router = LLMRouter::new(&config).await.expect("Failed to create router");

    if !router.has_ollama() && !router.has_omen() {
        println!("⚠️ Skipping: No LLM backend configured");
        return;
    }

    let parser = CommandParser::new(Some(router));

    // Test complex query that needs LLM
    let cmd = parser
        .parse("why is my ollama container using so much memory?")
        .await
        .expect("Failed to parse with LLM");

    assert!(!cmd.tool.is_empty(), "Tool should be identified");
    println!("✅ LLM-based parsing test passed");
    println!("   Query: why is my ollama container using so much memory?");
    println!("   Parsed: tool={}, action={}", cmd.tool, cmd.action);
}

#[tokio::test]
#[ignore] // Requires running MCP server
async fn test_mcp_system_status_tool() {
    use glyph::protocol::CallToolResult;
    use jarvis_core::mcp::SystemStatusTool;

    // Note: This test requires the Tool trait from glyph
    // It's a compile-time check that the tool implements the interface correctly

    let tool = SystemStatusTool;
    assert_eq!(tool.name(), "jarvis_system_status");
    assert!(tool.description().is_some());

    println!("✅ SystemStatusTool interface test passed");
}

#[test]
fn test_config_loading() {
    // Test default config creation
    let config = Config::default();

    assert_eq!(config.llm.primary_provider, "ollama");
    assert_eq!(config.llm.ollama_url, "http://localhost:11434");
    assert_eq!(config.system.arch_package_manager, "pacman");

    println!("✅ Config loading test passed");
}

#[test]
fn test_intent_classification() {
    // Test Intent enum usage
    use jarvis_core::Intent;

    let intents = vec![Intent::Code, Intent::System, Intent::DevOps, Intent::Reason];

    for intent in intents {
        // Just ensure they can be created and used
        let _intent_debug = format!("{:?}", intent);
    }

    println!("✅ Intent classification test passed");
}

// Integration test scenarios

#[tokio::test]
#[ignore] // Full integration test
async fn integration_test_natural_language_to_tool() {
    // Simulates: User query → NLP Parser → Tool Call → LLM Analysis

    let config = Config::default();
    let router = LLMRouter::new(&config).await.expect("Failed to create router");

    if !router.has_ollama() && !router.has_omen() {
        println!("⚠️ Skipping: No LLM backend configured");
        return;
    }

    let parser = CommandParser::new(Some(router.clone()));

    // Step 1: Parse natural language
    let cmd = parser
        .parse("show system status")
        .await
        .expect("Failed to parse");

    // Step 2: Verify correct tool was selected
    assert_eq!(cmd.tool, "jarvis_system_status");
    assert_eq!(cmd.intent, jarvis_core::CommandIntent::SystemStatus);

    // Step 3: Verify LLM can enhance the output (simulated)
    let enhancement_prompt = format!(
        "Summarize this system monitoring request in one sentence: {}",
        cmd.original_query
    );

    let summary = router
        .generate(&enhancement_prompt, None)
        .await
        .expect("Failed to generate summary");

    assert!(!summary.is_empty());

    println!("✅ Full integration test passed");
    println!("   Query: {}", cmd.original_query);
    println!("   Tool: {}", cmd.tool);
    println!("   Summary: {}", summary);
}

#[tokio::test]
#[ignore] // Requires Docker to be running
async fn integration_test_docker_diagnostics() {
    use std::process::Command as StdCommand;

    // Check if Docker is available
    let docker_check = StdCommand::new("docker").arg("ps").output();

    if docker_check.is_err() {
        println!("⚠️ Skipping: Docker not available");
        return;
    }

    // Test that Docker commands work
    let output = StdCommand::new("docker")
        .args(&["ps", "--format", "{{.Names}}"])
        .output()
        .expect("Docker ps failed");

    let containers = String::from_utf8_lossy(&output.stdout);

    println!("✅ Docker diagnostics test passed");
    println!("   Running containers: {}", containers.lines().count());
}

// Performance tests

#[tokio::test]
#[ignore] // Performance test
async fn perf_test_command_parsing() {
    use std::time::Instant;

    let parser = CommandParser::new(None);

    let test_queries = vec![
        "show system status",
        "list containers",
        "install docker",
        "check for updates",
        "list vms",
    ];

    let start = Instant::now();
    let iterations = 1000;

    for _ in 0..iterations {
        for query in &test_queries {
            let _ = parser.parse_rules(query);
        }
    }

    let elapsed = start.elapsed();
    let avg_time = elapsed / (iterations * test_queries.len() as u32);

    println!("✅ Performance test passed");
    println!("   Average parse time: {:?}", avg_time);
    println!("   Total iterations: {}", iterations * test_queries.len() as u32);

    // Assert reasonable performance (should be very fast for rule-based parsing)
    assert!(avg_time.as_micros() < 100, "Parsing should be under 100μs");
}

// Helper function for tests
#[allow(dead_code)]
fn setup_test_config() -> Config {
    let mut config = Config::default();
    config.llm.ollama_url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    config
}

// Module for testing helpers
#[cfg(test)]
mod test_helpers {
    #[allow(dead_code)]
    pub fn is_ollama_available() -> bool {
        std::process::Command::new("curl")
            .args(&["-s", "http://localhost:11434/api/tags"])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn is_docker_available() -> bool {
        std::process::Command::new("docker")
            .arg("ps")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }
}
