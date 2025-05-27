use anyhow::Result;
use crate::mcp::protocol::{
    CallToolRequest, CallToolResult, Content, ListToolsRequest, ListToolsResult, 
    TextContent, Tool,
};
use crate::mcp::server::{Server, ServerOptions};
use crate::mcp::transport::Transport;
use openapiv3::OpenAPI;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::env;
use tracing::{error, info, warn};

use crate::openapi_mcp_server::client::{HttpClient, HttpClientConfig, HttpClientError};
use crate::openapi_mcp_server::openapi::parser::{ConversionResult, OpenAPIToMCPConverter, OperationInfo};

pub struct MCPProxy {
    server: Server,
    http_client: HttpClient,
    tools: HashMap<String, crate::openapi_mcp_server::openapi::parser::MCPTool>,
    openapi_lookup: HashMap<String, OperationInfo>,
}

impl MCPProxy {
    pub fn new(name: String, openapi_spec: OpenAPI) -> Result<Self> {
        // Get base URL from the OpenAPI spec
        let base_url = openapi_spec.servers
            .first()
            .map(|server| server.url.clone())
            .ok_or_else(|| anyhow::anyhow!("No base URL found in OpenAPI spec"))?;

        info!("Using base URL: {}", base_url);

        // Parse headers from environment
        let headers = Self::parse_headers_from_env();
        
        // Create HTTP client
        let http_client_config = HttpClientConfig { base_url, headers };
        let http_client = HttpClient::new(http_client_config, openapi_spec.clone())?;

        // Convert OpenAPI spec to MCP tools
        let mut converter = OpenAPIToMCPConverter::new(openapi_spec);
        let ConversionResult { tools, openapi_lookup } = converter.convert_to_mcp_tools()?;

        // Create MCP server
        let server_options = ServerOptions {
            name,
            version: "1.8.1".to_string(),
        };
        let server = Server::new(server_options);

        info!("Created MCP proxy with {} tools", tools.len());

        Ok(Self {
            server,
            http_client,
            tools,
            openapi_lookup,
        })
    }

    pub async fn connect<T: Transport + Send + 'static>(&self, transport: T) -> Result<()> {
        let server = self.server.clone();
        
        // Set up tool handlers
        self.setup_handlers().await;
        
        // Connect to transport
        server.connect(transport).await?;
        
        Ok(())
    }

    async fn setup_handlers(&self) {
        let tools = self.tools.clone();
        let openapi_lookup = self.openapi_lookup.clone();
        let http_client = self.http_client.clone();

        // Handle list tools request
        self.server.add_handler(
            "tools/list",
            move |_request: ListToolsRequest| {
                let tools = tools.clone();
                async move {
                    let mut mcp_tools = Vec::new();

                    // Convert each method to a separate tool
                    for (tool_name, tool_def) in &tools {
                        for method in &tool_def.methods {
                            let tool_name_with_method = format!("{}-{}", tool_name, method.name);
                            let truncated_name = Self::truncate_tool_name(&tool_name_with_method);
                            
                            mcp_tools.push(Tool {
                                name: truncated_name,
                                description: method.description.clone(),
                                input_schema: method.input_schema.clone(),
                            });
                        }
                    }

                    Ok(ListToolsResult { tools: mcp_tools })
                }
            },
        );

        // Handle call tool request
        self.server.add_handler(
            "tools/call",
            move |request: CallToolRequest| {
                let openapi_lookup = openapi_lookup.clone();
                let http_client = http_client.clone();
                
                async move {
                    let tool_name = &request.params.name;
                    let arguments = &request.params.arguments;

                    // Find the operation in OpenAPI spec
                    let operation_info = openapi_lookup.get(tool_name)
                        .ok_or_else(|| anyhow::anyhow!("Method {} not found", tool_name))?;

                    // Convert arguments to HashMap<String, Value>
                    let params = Self::extract_params_from_arguments(arguments)?;

                    // Execute the operation
                    match http_client.execute_operation(operation_info, params).await {
                        Ok(response) => {
                            let content = Content::Text(TextContent {
                                text: serde_json::to_string(&response.data)?,
                            });
                            
                            Ok(CallToolResult {
                                content: vec![content],
                                is_error: false,
                            })
                        }
                        Err(HttpClientError::RequestFailed { status, data, .. }) => {
                            error!("HTTP request failed with status {}: {:?}", status, data);
                            
                            let error_data = data.unwrap_or_else(|| {
                                Value::Object({
                                    let mut map = Map::new();
                                    map.insert("status".to_string(), Value::String("error".to_string()));
                                    map.insert("message".to_string(), Value::String(format!("HTTP {} error", status)));
                                    map
                                })
                            });
                            
                            let content = Content::Text(TextContent {
                                text: serde_json::to_string(&error_data)?,
                            });
                            
                            Ok(CallToolResult {
                                content: vec![content],
                                is_error: true,
                            })
                        }
                        Err(e) => {
                            error!("Error executing operation: {}", e);
                            Err(anyhow::anyhow!("Failed to execute operation: {}", e))
                        }
                    }
                }
            },
        );
    }

    fn parse_headers_from_env() -> HashMap<String, String> {
        let headers_json = match env::var("OPENAPI_MCP_HEADERS") {
            Ok(json) => json,
            Err(_) => {
                info!("No OPENAPI_MCP_HEADERS environment variable found");
                return HashMap::new();
            }
        };

        match serde_json::from_str::<Value>(&headers_json) {
            Ok(Value::Object(map)) => {
                let mut headers = HashMap::new();
                for (key, value) in map {
                    if let Value::String(value_str) = value {
                        headers.insert(key, value_str);
                    } else {
                        warn!("Header value for '{}' is not a string, skipping", key);
                    }
                }
                info!("Parsed {} headers from environment", headers.len());
                headers
            }
            Ok(_) => {
                warn!("OPENAPI_MCP_HEADERS must be a JSON object, got other type");
                HashMap::new()
            }
            Err(e) => {
                warn!("Failed to parse OPENAPI_MCP_HEADERS: {}", e);
                HashMap::new()
            }
        }
    }

    fn extract_params_from_arguments(arguments: &Value) -> Result<HashMap<String, Value>> {
        match arguments {
            Value::Object(map) => Ok(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            _ => Err(anyhow::anyhow!("Tool arguments must be an object")),
        }
    }

    fn truncate_tool_name(name: &str) -> String {
        if name.len() <= 64 {
            name.to_string()
        } else {
            name[..64].to_string()
        }
    }
}