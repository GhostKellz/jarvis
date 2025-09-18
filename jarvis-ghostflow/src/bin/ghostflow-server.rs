use anyhow::{Context, Result};
use clap::Parser;
use std::net::SocketAddr;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use jarvis_ghostflow::{
    create_ghostflow_server, IntegrationConfig, JarvisGhostFlowIntegration
};

/// GhostFlow Server - n8n-style workflow automation with Jarvis AI integration
#[derive(Parser)]
#[command(name = "ghostflow-server")]
#[command(about = "Start the GhostFlow workflow automation server")]
#[command(version)]
struct Args {
    /// API server address
    #[arg(long, default_value = "127.0.0.1:8080")]
    api_address: SocketAddr,

    /// QUIC server address (optional)
    #[arg(long)]
    quic_address: Option<SocketAddr>,

    /// Enable QUIC networking
    #[arg(long, default_value = "true")]
    enable_quic: bool,

    /// Enable WebSocket support
    #[arg(long, default_value = "true")]
    enable_websockets: bool,

    /// Enable metrics collection
    #[arg(long, default_value = "true")]
    enable_metrics: bool,

    /// Workflow storage path
    #[arg(long, default_value = "./workflows")]
    workflow_storage_path: String,

    /// Run demo workflow on startup
    #[arg(long)]
    run_demo: bool,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let log_level = args.log_level.parse()
        .unwrap_or(tracing::Level::INFO);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    format!("ghostflow_server={},jarvis_ghostflow={}", log_level, log_level).into()
                })
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting GhostFlow Server");
    info!("API Address: {}", args.api_address);
    info!("QUIC Enabled: {}", args.enable_quic);
    info!("WebSockets Enabled: {}", args.enable_websockets);
    info!("Metrics Enabled: {}", args.enable_metrics);

    // Set demo environment variable if requested
    if args.run_demo {
        std::env::set_var("GHOSTFLOW_RUN_DEMO", "true");
        info!("Demo workflow execution enabled");
    }

    // Create integration config
    let config = IntegrationConfig {
        api_address: args.api_address,
        enable_quic: args.enable_quic,
        quic_address: args.quic_address,
        enable_websockets: args.enable_websockets,
        enable_metrics: args.enable_metrics,
        workflow_storage_path: args.workflow_storage_path,
    };

    // Create and start GhostFlow server
    let integration = create_ghostflow_server(Some(config)).await
        .context("Failed to create GhostFlow server")?;

    info!("🚀 GhostFlow Server started successfully!");
    info!("📡 API available at: http://{}", args.api_address);
    
    if args.enable_quic {
        if let Some(quic_addr) = args.quic_address {
            info!("⚡ QUIC available at: {}", quic_addr);
        }
    }

    info!("📊 Available endpoints:");
    info!("  • GET  /api/health           - Health check");
    info!("  • GET  /api/metrics          - System metrics");
    info!("  • GET  /api/workflows        - List workflows");
    info!("  • POST /api/workflows        - Create workflow");
    info!("  • GET  /api/workflows/:id    - Get workflow");
    info!("  • PUT  /api/workflows/:id    - Update workflow");
    info!("  • POST /api/workflows/:id/execute - Execute workflow");
    info!("  • GET  /api/node-types       - List available node types");

    // Print some usage examples
    print_usage_examples(&args).await;

    // Handle shutdown gracefully
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            info!("Received interrupt signal, shutting down...");
            integration.shutdown().await
                .context("Failed to shutdown integration")?;
            info!("GhostFlow Server shutdown complete");
        }
        Err(err) => {
            warn!("Unable to listen for shutdown signal: {}", err);
        }
    }

    Ok(())
}

async fn print_usage_examples(args: &Args) {
    info!("");
    info!("📝 Usage Examples:");
    info!("");
    info!("1. Check server health:");
    info!("   curl http://{}/api/health", args.api_address);
    info!("");
    info!("2. List available node types:");
    info!("   curl http://{}/api/node-types", args.api_address);
    info!("");
    info!("3. Create a simple workflow:");
    info!("   curl -X POST http://{}/api/workflows \\", args.api_address);
    info!("     -H 'Content-Type: application/json' \\");
    info!("     -d '{{");
    info!("       \"name\": \"My First Workflow\",");
    info!("       \"description\": \"A simple test workflow\",");
    info!("       \"nodes\": {{");
    info!("         \"start\": {{");
    info!("           \"id\": \"start\",");
    info!("           \"node_type\": \"start\",");
    info!("           \"position\": {{\"x\": 100, \"y\": 100}},");
    info!("           \"parameters\": {{}}");
    info!("         }}");
    info!("       }},");
    info!("       \"connections\": []");
    info!("     }}'");
    info!("");
    info!("4. Execute a workflow:");
    info!("   curl -X POST http://{}/api/workflows/{{workflow_id}}/execute \\", args.api_address);
    info!("     -H 'Content-Type: application/json' \\");
    info!("     -d '{{");
    info!("       \"trigger_data\": {{\"message\": \"Hello World!\"}},");
    info!("       \"execution_mode\": \"manual\"");
    info!("     }}'");
    info!("");
    info!("🎯 Advanced Features:");
    info!("  • AI-powered nodes with LLM routing");
    info!("  • Blockchain integration for Web3 workflows");
    info!("  • Memory nodes for persistent context");
    info!("  • Agent orchestration for complex tasks");
    info!("  • Real-time workflow execution monitoring");
    info!("  • QUIC networking for high-performance communication");
    info!("");

    if args.run_demo {
        info!("🎪 Demo workflow will be executed automatically in 2 seconds...");
    } else {
        info!("💡 Tip: Use --run-demo to automatically execute a demo workflow");
    }
    info!("");
}

/// Print a banner with ASCII art
fn _print_banner() {
    println!(r#"
    ╔═══════════════════════════════════════╗
    ║            GhostFlow Server           ║
    ║        n8n-style AI Workflows        ║
    ║     Powered by Jarvis Integration     ║
    ╚═══════════════════════════════════════╝
    "#);
}