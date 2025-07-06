use anyhow::Result;
use jarvis_core::{Config, LLMRouter, MemoryStore};
use jarvis_nvim::{AIIntegration, JarvisNvim, lsp::JarvisLspServer};
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: jarvis-nvim <mode> [options]");
        eprintln!("Modes:");
        eprintln!("  lsp       - Start LSP server");
        eprintln!("  client    - Start Neovim client");
        eprintln!("  socket    - Start socket server");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "lsp" => {
            start_lsp_server().await?;
        }
        "client" => {
            let socket_path = args.get(2).map(|s| s.as_str()).unwrap_or("/tmp/nvim.sock");
            start_nvim_client(socket_path).await?;
        }
        "socket" => {
            let socket_path = args
                .get(2)
                .map(|s| s.as_str())
                .unwrap_or("/tmp/jarvis.sock");
            start_socket_server(socket_path).await?;
        }
        _ => {
            eprintln!("Unknown mode: {}", args[1]);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn start_lsp_server() -> Result<()> {
    println!("Starting Jarvis LSP server...");

    // Initialize core components
    let config = Config::load(None).await?;
    let memory = Arc::new(MemoryStore::new(&config.database_path).await?);
    let llm = Arc::new(LLMRouter::new(&config).await?);
    let ai = Arc::new(AIIntegration::new(llm, memory));

    // Start LSP server
    JarvisLspServer::start(ai).await?;

    Ok(())
}

async fn start_nvim_client(socket_path: &str) -> Result<()> {
    println!("Connecting to Neovim at: {}", socket_path);

    let jarvis = JarvisNvim::new(socket_path).await?;

    // Keep the client running
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

async fn start_socket_server(socket_path: &str) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{UnixListener, UnixStream};

    println!("Starting Jarvis socket server at: {}", socket_path);

    // Remove existing socket file
    let _ = tokio::fs::remove_file(socket_path).await;

    let listener = UnixListener::bind(socket_path)?;

    // Initialize core components
    let config = Config::load(None).await?;
    let memory = Arc::new(MemoryStore::new(&config.database_path).await?);
    let llm = Arc::new(LLMRouter::new(&config).await?);
    let ai = Arc::new(AIIntegration::new(llm, memory));

    println!("Jarvis socket server listening on {}", socket_path);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let ai_clone = ai.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, ai_clone).await {
                        eprintln!("Error handling client: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_client(mut stream: UnixStream, ai: Arc<AIIntegration>) -> Result<()> {
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // Send welcome message
    writer.write_all(b"Connected to Jarvis\n").await?;

    loop {
        line.clear();
        match reader.read_line(&mut line).await? {
            0 => break, // EOF
            _ => {
                let command = line.trim();
                if command.is_empty() {
                    continue;
                }

                // Parse command
                let parts: Vec<&str> = command.splitn(2, ' ').collect();
                let response = match parts[0] {
                    "explain" => {
                        let code = parts.get(1).unwrap_or(&"");
                        ai.explain_code(code, "rust", "")
                            .await
                            .unwrap_or_else(|e| format!("Error: {}", e))
                    }
                    "improve" => {
                        let code = parts.get(1).unwrap_or(&"");
                        ai.suggest_improvements(code, "rust")
                            .await
                            .unwrap_or_else(|e| format!("Error: {}", e))
                    }
                    "fix" => {
                        let code = parts.get(1).unwrap_or(&"");
                        ai.fix_errors(code, &[], "rust")
                            .await
                            .unwrap_or_else(|e| format!("Error: {}", e))
                    }
                    "generate" => {
                        let description = parts.get(1).unwrap_or(&"");
                        ai.generate_code(description, "rust", "")
                            .await
                            .unwrap_or_else(|e| format!("Error: {}", e))
                    }
                    "chat" => {
                        let message = parts.get(1).unwrap_or(&"");
                        ai.send_message(message, None)
                            .await
                            .unwrap_or_else(|e| format!("Error: {}", e))
                    }
                    "quit" | "exit" => break,
                    _ => format!("Unknown command: {}", parts[0]),
                };

                writer.write_all(response.as_bytes()).await?;
                writer.write_all(b"\n---\n").await?;
            }
        }
    }

    Ok(())
}
