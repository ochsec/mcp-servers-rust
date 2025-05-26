use anyhow::{anyhow, Result};
use crate::mcp_core::{
    protocol::{JsonRpcRequest, JsonRpcResponse, McpMessage},
    server::{McpServer, RequestHandler},
    tools::{Tool, ToolHandler},
    resources::{Resource, ResourceHandler},
};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::github::{GitHubClient, GitHubConfig};
use crate::tools::{ToolRegistry, ToolsetGroup};
use crate::resources::ResourceRegistry;

pub struct GitHubMcpServer {
    server: McpServer,
    github_client: Arc<GitHubClient>,
    tools: Arc<RwLock<ToolRegistry>>,
    resources: Arc<ResourceRegistry>,
    config: GitHubServerConfig,
}

#[derive(Debug, Clone)]
pub struct GitHubServerConfig {
    pub version: String,
    pub host: Option<String>,
    pub token: String,
    pub enabled_toolsets: Vec<String>,
    pub dynamic_toolsets: bool,
    pub read_only: bool,
    pub enable_command_logging: bool,
}

impl GitHubMcpServer {
    pub async fn new(config: GitHubServerConfig) -> Result<Self> {
        let github_config = GitHubConfig {
            token: config.token.clone(),
            host: config.host.clone(),
            user_agent: format!("github-mcp-server/{}", config.version),
        };

        let github_client = Arc::new(GitHubClient::new(github_config).await?);
        
        let tools = Arc::new(RwLock::new(ToolRegistry::new(
            config.enabled_toolsets.clone(),
            config.read_only,
            config.dynamic_toolsets,
            github_client.clone(),
        )));

        let resources = Arc::new(ResourceRegistry::new(github_client.clone()));

        let server = McpServer::new(
            "github-mcp-server",
            &config.version,
        );

        Ok(Self {
            server,
            github_client,
            tools,
            resources,
            config,
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing GitHub MCP Server v{}", self.config.version);

        {
            let mut tools = self.tools.write().await;
            tools.initialize().await?;
            
            // Tool handlers are managed internally by ToolRegistry
        }

        // Resource handlers are managed internally by ResourceRegistry

        info!("GitHub MCP Server initialized successfully");
        Ok(())
    }

    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling request: {:?}", request);
        
        let response = match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "tools/list" => self.handle_list_tools(request).await,
            "tools/call" => self.handle_call_tool(request).await,
            "resources/list" => self.handle_list_resources(request).await,
            "resources/read" => self.handle_read_resource(request).await,
            _ => {
                warn!("Unknown method: {}", request.method);
                JsonRpcResponse::error(
                    request.id,
                    -32601,
                    "Method not found",
                    None,
                )
            }
        };

        if self.config.enable_command_logging {
            debug!("Response: {:?}", response);
        }

        response
    }

    async fn handle_initialize(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let capabilities = serde_json::json!({
            "tools": {
                "list": true,
                "call": true
            },
            "resources": {
                "list": true,
                "read": true
            }
        });

        let result = serde_json::json!({
            "protocolVersion": "0.1.0",
            "capabilities": capabilities,
            "serverInfo": {
                "name": "github-mcp-server",
                "version": self.config.version
            }
        });

        JsonRpcResponse::success(request.id, result)
    }

    async fn handle_list_tools(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match self.tools.read().await.list_tools().await {
            Ok(tools) => {
                let result = serde_json::json!({
                    "tools": tools
                });
                JsonRpcResponse::success(request.id, result)
            }
            Err(e) => {
                error!("Failed to list tools: {}", e);
                JsonRpcResponse::error(
                    request.id,
                    -32603,
                    "Internal error",
                    Some(serde_json::json!({"error": e.to_string()})),
                )
            }
        }
    }

    async fn handle_call_tool(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params = request.params.unwrap_or_default();
        
        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                return JsonRpcResponse::error(
                    request.id,
                    -32602,
                    "Invalid params: missing tool name",
                    None,
                );
            }
        };

        let arguments = params.get("arguments")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        match self.tools.read().await.call_tool(tool_name, arguments).await {
            Ok(result) => JsonRpcResponse::success(request.id, result),
            Err(e) => {
                error!("Tool call failed: {}", e);
                JsonRpcResponse::error(
                    request.id,
                    -32603,
                    "Tool execution failed",
                    Some(serde_json::json!({"error": e.to_string()})),
                )
            }
        }
    }

    async fn handle_list_resources(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match self.resources.list_resources().await {
            Ok(resources) => {
                let result = serde_json::json!({
                    "resources": resources
                });
                JsonRpcResponse::success(request.id, result)
            }
            Err(e) => {
                error!("Failed to list resources: {}", e);
                JsonRpcResponse::error(
                    request.id,
                    -32603,
                    "Internal error",
                    Some(serde_json::json!({"error": e.to_string()})),
                )
            }
        }
    }

    async fn handle_read_resource(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params = request.params.unwrap_or_default();
        
        let uri = match params.get("uri").and_then(|v| v.as_str()) {
            Some(uri) => uri,
            None => {
                return JsonRpcResponse::error(
                    request.id,
                    -32602,
                    "Invalid params: missing resource URI",
                    None,
                );
            }
        };

        match self.resources.read_resource(uri).await {
            Ok(result) => JsonRpcResponse::success(request.id, result),
            Err(e) => {
                error!("Resource read failed: {}", e);
                JsonRpcResponse::error(
                    request.id,
                    -32603,
                    "Resource read failed",
                    Some(serde_json::json!({"error": e.to_string()})),
                )
            }
        }
    }

    pub async fn run_stdio(&mut self) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        self.initialize().await?;

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        info!("GitHub MCP Server running on stdio");

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                        Ok(request) => {
                            let response = self.handle_request(request).await;
                            let response_json = serde_json::to_string(&response)?;
                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                        Err(e) => {
                            error!("Failed to parse JSON-RPC request: {}", e);
                            let error_response = JsonRpcResponse::error(
                                None,
                                -32700,
                                "Parse error",
                                Some(serde_json::json!({"error": e.to_string()})),
                            );
                            let response_json = serde_json::to_string(&error_response)?;
                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from stdin: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}

pub fn required_param<T>(args: &Map<String, Value>, name: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let value = args.get(name)
        .ok_or_else(|| anyhow!("Missing required parameter: {}", name))?;
    
    serde_json::from_value(value.clone())
        .map_err(|_| anyhow!("Parameter '{}' has invalid type", name))
}

pub fn optional_param<T>(args: &Map<String, Value>, name: &str) -> Result<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    match args.get(name) {
        Some(value) => Ok(Some(
            serde_json::from_value(value.clone())
                .map_err(|_| anyhow!("Parameter '{}' has invalid type", name))?
        )),
        None => Ok(None),
    }
}

pub fn optional_param_with_default<T>(args: &Map<String, Value>, name: &str, default: T) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    match optional_param(args, name)? {
        Some(value) => Ok(value),
        None => Ok(default),
    }
}

#[derive(Debug, Clone)]
pub struct PaginationParams {
    pub page: u32,
    pub per_page: u32,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 30,
        }
    }
}

pub fn extract_pagination_params(args: &Map<String, Value>) -> Result<PaginationParams> {
    let page = optional_param_with_default(args, "page", 1u32)?;
    let per_page = optional_param_with_default(args, "perPage", 30u32)?;
    
    if page < 1 {
        return Err(anyhow!("Page must be at least 1"));
    }
    
    if per_page < 1 || per_page > 100 {
        return Err(anyhow!("Per page must be between 1 and 100"));
    }
    
    Ok(PaginationParams { page, per_page })
}