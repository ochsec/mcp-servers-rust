use anyhow::Result;
use clap::Parser;
use serde_json::Value;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info};
use tracing_subscriber;

mod atlassian;
mod config;
mod mcp_types;

use atlassian::AtlassianClient;
use config::Config;
use mcp_types::*;

#[derive(Parser)]
#[command(name = "atlassian-mcp-server")]
#[command(about = "MCP server for Atlassian JIRA and Confluence integration")]
struct Cli {}

#[derive(Debug, Clone)]
pub struct AtlassianMcpServer {
    client: AtlassianClient,
    config: Config,
}

impl AtlassianMcpServer {
    fn new(config: Config) -> Self {
        let client = AtlassianClient::new(config.atlassian.clone());
        Self { client, config }
    }

    fn get_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "get_jira_ticket".to_string(),
                description: "Get details of a JIRA ticket by key".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "ticket_key": {
                            "type": "string",
                            "description": "The JIRA ticket key (e.g., PROJ-123)"
                        }
                    },
                    "required": ["ticket_key"]
                }),
            },
            Tool {
                name: "search_jira_tickets".to_string(),
                description: "Search for JIRA tickets using JQL".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "jql": {
                            "type": "string",
                            "description": "JQL query string"
                        },
                        "max_results": {
                            "type": "integer",
                            "description": "Maximum number of results to return",
                            "default": 10
                        }
                    },
                    "required": ["jql"]
                }),
            },
            Tool {
                name: "create_jira_ticket".to_string(),
                description: "Create a new JIRA ticket".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_key": {
                            "type": "string",
                            "description": "The project key where the ticket will be created"
                        },
                        "summary": {
                            "type": "string",
                            "description": "Summary/title of the ticket"
                        },
                        "description": {
                            "type": "string",
                            "description": "Description of the ticket"
                        },
                        "issue_type": {
                            "type": "string",
                            "description": "Type of issue (e.g., Task, Bug, Story)",
                            "default": "Task"
                        }
                    },
                    "required": ["project_key", "summary", "description"]
                }),
            },
            Tool {
                name: "add_comment_to_jira_ticket".to_string(),
                description: "Add a comment to a JIRA ticket".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "ticket_key": {
                            "type": "string",
                            "description": "The JIRA ticket key"
                        },
                        "comment": {
                            "type": "string",
                            "description": "Comment text to add"
                        }
                    },
                    "required": ["ticket_key", "comment"]
                }),
            },
            Tool {
                name: "get_confluence_page".to_string(),
                description: "Get a Confluence page by ID".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_id": {
                            "type": "string",
                            "description": "The Confluence page ID"
                        }
                    },
                    "required": ["page_id"]
                }),
            },
            Tool {
                name: "search_confluence".to_string(),
                description: "Search for content in Confluence".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query text"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results to return",
                            "default": 10
                        }
                    },
                    "required": ["query"]
                }),
            },
        ]
    }

    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "tools/list" => self.handle_list_tools(request).await,
            "tools/call" => self.handle_call_tool(request).await,
            _ => error_response(
                request.id,
                -32601,
                "Method not found",
                None,
            ),
        }
    }

    async fn handle_initialize(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: None,
            },
            server_info: ServerInfo {
                name: "atlassian-mcp-server".to_string(),
                version: "0.1.0".to_string(),
                instructions: Some(format!(
                    "Atlassian MCP server for JIRA and Confluence integration. Connected to {}",
                    self.config.atlassian.base_url
                )),
            },
            instructions: Some(format!(
                "This server provides tools to interact with Atlassian JIRA and Confluence at {}. Use the available tools to manage JIRA tickets and access Confluence content.",
                self.config.atlassian.base_url
            )),
        };

        success_response(request.id, serde_json::to_value(result).unwrap())
    }

    async fn handle_list_tools(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let tools = self.get_tools();
        let result = serde_json::json!({
            "tools": tools
        });

        success_response(request.id, result)
    }

    async fn handle_call_tool(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params: ToolCallParams = match request.params {
            Some(params) => match serde_json::from_value(params) {
                Ok(p) => p,
                Err(e) => {
                    return error_response(
                        request.id,
                        -32602,
                        &format!("Invalid params: {}", e),
                        None,
                    );
                }
            },
            None => {
                return error_response(
                    request.id,
                    -32602,
                    "Missing params",
                    None,
                );
            }
        };

        let args = params.arguments.unwrap_or_default();

        let result = match params.name.as_str() {
            "get_jira_ticket" => self.call_get_jira_ticket(args).await,
            "search_jira_tickets" => self.call_search_jira_tickets(args).await,
            "create_jira_ticket" => self.call_create_jira_ticket(args).await,
            "add_comment_to_jira_ticket" => self.call_add_comment_to_jira_ticket(args).await,
            "get_confluence_page" => self.call_get_confluence_page(args).await,
            "search_confluence" => self.call_search_confluence(args).await,
            _ => {
                return error_response(
                    request.id,
                    -32601,
                    &format!("Unknown tool: {}", params.name),
                    None,
                );
            }
        };

        match result {
            Ok(content) => {
                let tool_result = ToolCallResult {
                    content: vec![ToolCallContent {
                        content_type: "text".to_string(),
                        text: content,
                    }],
                    is_error: Some(false),
                };
                success_response(request.id, serde_json::to_value(tool_result).unwrap())
            }
            Err(e) => {
                let tool_result = ToolCallResult {
                    content: vec![ToolCallContent {
                        content_type: "text".to_string(),
                        text: format!("Error: {}", e),
                    }],
                    is_error: Some(true),
                };
                success_response(request.id, serde_json::to_value(tool_result).unwrap())
            }
        }
    }

    async fn call_get_jira_ticket(&self, args: HashMap<String, Value>) -> Result<String, String> {
        let ticket_key: String = required_param(&args, "ticket_key")?;
        
        match self.client.get_jira_ticket(&ticket_key).await {
            Ok(ticket) => Ok(serde_json::to_string_pretty(&ticket).unwrap_or_else(|_| ticket.to_string())),
            Err(e) => Err(format!("Error getting JIRA ticket: {}", e)),
        }
    }

    async fn call_search_jira_tickets(&self, args: HashMap<String, Value>) -> Result<String, String> {
        let jql: String = required_param(&args, "jql")?;
        let max_results: Option<u32> = optional_param(&args, "max_results")?;

        match self.client.search_jira_tickets(&jql, max_results).await {
            Ok(results) => Ok(serde_json::to_string_pretty(&results).unwrap_or_else(|_| results.to_string())),
            Err(e) => Err(format!("Error searching JIRA tickets: {}", e)),
        }
    }

    async fn call_create_jira_ticket(&self, args: HashMap<String, Value>) -> Result<String, String> {
        let project_key: String = required_param(&args, "project_key")?;
        let summary: String = required_param(&args, "summary")?;
        let description: String = required_param(&args, "description")?;
        let issue_type: Option<String> = optional_param(&args, "issue_type")?;

        match self
            .client
            .create_jira_ticket(
                &project_key,
                &summary,
                &description,
                issue_type.as_deref(),
            )
            .await
        {
            Ok(ticket) => {
                let ticket_key = ticket
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                Ok(format!("Created JIRA ticket: {}\n\n{}", ticket_key, serde_json::to_string_pretty(&ticket).unwrap_or_else(|_| ticket.to_string())))
            }
            Err(e) => Err(format!("Error creating JIRA ticket: {}", e)),
        }
    }

    async fn call_add_comment_to_jira_ticket(&self, args: HashMap<String, Value>) -> Result<String, String> {
        let ticket_key: String = required_param(&args, "ticket_key")?;
        let comment: String = required_param(&args, "comment")?;

        match self
            .client
            .add_comment_to_jira_ticket(&ticket_key, &comment)
            .await
        {
            Ok(_) => Ok(format!("Added comment to {}", ticket_key)),
            Err(e) => Err(format!("Error adding comment to JIRA ticket: {}", e)),
        }
    }

    async fn call_get_confluence_page(&self, args: HashMap<String, Value>) -> Result<String, String> {
        let page_id: String = required_param(&args, "page_id")?;

        match self.client.get_confluence_page(&page_id).await {
            Ok(page) => Ok(serde_json::to_string_pretty(&page).unwrap_or_else(|_| page.to_string())),
            Err(e) => Err(format!("Error getting Confluence page: {}", e)),
        }
    }

    async fn call_search_confluence(&self, args: HashMap<String, Value>) -> Result<String, String> {
        let query: String = required_param(&args, "query")?;
        let limit: Option<u32> = optional_param(&args, "limit")?;

        match self.client.search_confluence(&query, limit).await {
            Ok(results) => Ok(serde_json::to_string_pretty(&results).unwrap_or_else(|_| results.to_string())),
            Err(e) => Err(format!("Error searching Confluence: {}", e)),
        }
    }

    pub async fn run_stdio(&mut self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        eprintln!(
            "Atlassian MCP server running on stdio (connected to {})",
            self.config.atlassian.base_url
        );
        eprintln!("Available tools: get_jira_ticket, search_jira_tickets, create_jira_ticket, add_comment_to_jira_ticket, get_confluence_page, search_confluence");

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
                        Ok(req) => req,
                        Err(e) => {
                            error!("Failed to parse JSON-RPC request: {}", e);
                            let error_resp = error_response(
                                None,
                                -32700,
                                "Parse error",
                                Some(serde_json::json!({"error": e.to_string()})),
                            );
                            let response_json = serde_json::to_string(&error_resp)?;
                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                            continue;
                        }
                    };

                    let response = self.handle_request(request).await;
                    let response_json = serde_json::to_string(&response)?;
                    stdout.write_all(response_json.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                }
                Err(e) => {
                    error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let _cli = Cli::parse();

    let config = match Config::load() {
        Ok(config) => {
            info!("Loaded configuration for {}", config.atlassian.base_url);
            config
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            eprintln!("Make sure to set the required environment variables:");
            eprintln!("  ATLASSIAN_BASE_URL=https://your-instance.atlassian.net");
            eprintln!("  ATLASSIAN_EMAIL=your-email@example.com");
            eprintln!("  ATLASSIAN_TOKEN=your-api-token");
            std::process::exit(1);
        }
    };

    let mut atlassian_server = AtlassianMcpServer::new(config);
    atlassian_server.run_stdio().await?;

    Ok(())
}