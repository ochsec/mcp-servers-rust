use anyhow::{Context, Result};
use openapiv3::OpenAPI;
use std::fs;
use std::path::Path;
use tracing::{error, info};

use crate::openapi_mcp_server::mcp_proxy::proxy::MCPProxy;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("OpenAPI validation failed: {0:?}")]
    ValidationFailed(Vec<String>),
}

async fn load_openapi_spec(spec_path: &str, base_url: Option<&str>) -> Result<OpenAPI> {
    let path = Path::new(spec_path);
    
    if !path.exists() {
        return Err(anyhow::anyhow!(
            "OpenAPI specification file not found: {}",
            spec_path
        ));
    }

    let raw_spec = fs::read_to_string(path)
        .with_context(|| format!("Failed to read OpenAPI specification file: {}", spec_path))?;

    // Parse the OpenAPI spec
    let mut parsed: OpenAPI = serde_json::from_str(&raw_spec)
        .with_context(|| "Failed to parse OpenAPI specification as JSON")?;

    // Override base URL if specified
    if let Some(url) = base_url {
        info!("Overriding base URL with: {}", url);
        if !parsed.servers.is_empty() {
            parsed.servers[0].url = url.to_string();
        }
    }

    Ok(parsed)
}

pub async fn init_proxy(spec_path: &str, base_url: Option<&str>) -> Result<MCPProxy> {
    info!("Loading OpenAPI specification from: {}", spec_path);
    
    let openapi_spec = load_openapi_spec(spec_path, base_url).await?;
    
    info!("Creating MCP proxy for Notion API");
    let proxy = MCPProxy::new("Notion API".to_string(), openapi_spec)?;

    info!("MCP proxy initialized successfully");
    Ok(proxy)
}