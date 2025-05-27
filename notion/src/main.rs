use anyhow::Result;
use std::env;
use tracing::{info, warn};
use tracing_subscriber;

mod init_server;
mod openapi_mcp_server;
mod mcp;

use init_server::init_proxy;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "notion_mcp_server=info".into()),
        )
        .init();

    info!("Starting Notion MCP Server (Rust)");

    // Get OpenAPI spec path from environment or use default
    let spec_path = env::var("OPENAPI_SPEC_PATH")
        .unwrap_or_else(|_| "scripts/notion-openapi.json".to_string());

    // Get base URL override from environment
    let base_url = env::var("OPENAPI_BASE_URL").ok();

    // Initialize and start the MCP proxy
    let proxy = init_proxy(&spec_path, base_url.as_deref()).await?;
    
    info!("MCP server initialized, connecting to stdio transport");
    
    // Connect to stdio transport for MCP communication  
    let transport = mcp::stdio::StdioServerTransport::new();
    proxy.connect(transport).await?;

    Ok(())
}