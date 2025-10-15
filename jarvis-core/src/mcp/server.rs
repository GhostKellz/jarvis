//! Jarvis MCP Server

use anyhow::Result;
use glyph::server::ServerBuilder;
use crate::mcp::tools::*;

/// Run Jarvis MCP server
pub async fn run_mcp_server(transport: &str, address: Option<&str>, llm_router: Option<crate::llm::LLMRouter>) -> Result<()> {
    tracing::info!("Starting Jarvis MCP server with transport: {}", transport);

    let builder = ServerBuilder::new()
        .with_server_info("jarvis", env!("CARGO_PKG_VERSION"));

    // Configure transport and run server
    match transport {
        "stdio" => {
            tracing::info!("Using stdio transport");
            let mut server_with_transport = builder.for_stdio();

            // Register tools
            tracing::info!("Registering Jarvis tools");
            server_with_transport.server().register_tool(SystemStatusTool).await?;
            server_with_transport.server().register_tool(PackageManagerTool).await?;
            server_with_transport.server().register_tool(DockerTool::new(llm_router.clone())).await?;

            tracing::info!("Jarvis MCP server ready");
            server_with_transport.run().await?;
        },
        "ws" | "websocket" => {
            let addr = address.unwrap_or("127.0.0.1:7332");
            tracing::info!("Using WebSocket transport on {}", addr);
            let mut server_with_transport = builder.for_websocket(addr).await?;

            // Register tools
            tracing::info!("Registering Jarvis tools");
            server_with_transport.server().register_tool(SystemStatusTool).await?;
            server_with_transport.server().register_tool(PackageManagerTool).await?;
            server_with_transport.server().register_tool(DockerTool::new(llm_router)).await?;

            tracing::info!("Jarvis MCP server ready");
            server_with_transport.run().await?;
        },
        _ => return Err(anyhow::anyhow!("Unsupported transport: {} (supported: stdio, ws, websocket)", transport)),
    };

    Ok(())
}
