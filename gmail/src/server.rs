use crate::client::GmailClient;
use crate::error::{GmailError, Result};
use crate::mcp_types::*;
use crate::tools::GmailTools;
use serde_json::json;
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

pub struct GmailMcpServer {
    client: Arc<Mutex<GmailClient>>,
}

impl GmailMcpServer {
    pub async fn new() -> Result<Self> {
        let client = Arc::new(Mutex::new(GmailClient::new().await?));
        Ok(Self { client })
    }

    pub async fn authenticate(&mut self, callback_url: &str) -> Result<()> {
        let mut client = self.client.lock().await;
        client.authenticate(callback_url).await
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting Gmail MCP server...");

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = line.map_err(|e| GmailError::IoError(e))?;
            
            if line.trim().is_empty() {
                continue;
            }

            debug!("Received request: {}", line);

            let request: McpRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    error!("Failed to parse request: {}", e);
                    let error_response = McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(McpError {
                            code: -32700,
                            message: "Parse error".to_string(),
                            data: None,
                        }),
                    };
                    let response_json = serde_json::to_string(&error_response)?;
                    writeln!(stdout, "{}", response_json)?;
                    stdout.flush()?;
                    continue;
                }
            };

            let response = self.handle_request(request).await;
            let response_json = serde_json::to_string(&response)?;
            writeln!(stdout, "{}", response_json)?;
            stdout.flush()?;
        }

        Ok(())
    }

    async fn handle_request(&self, request: McpRequest) -> McpResponse {
        debug!("Handling request: {}", request.method);

        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "initialized" => {
                // Acknowledge the initialized notification
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({})),
                    error: None,
                }
            }
            "tools/list" => self.handle_list_tools(request).await,
            "tools/call" => self.handle_call_tool(request).await,
            _ => {
                error!("Unknown method: {}", request.method);
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32601,
                        message: "Method not found".to_string(),
                        data: None,
                    }),
                }
            }
        }
    }

    async fn handle_initialize(&self, request: McpRequest) -> McpResponse {
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            server_info: ServerInfo {
                name: "gmail".to_string(),
                version: "0.1.0".to_string(),
            },
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {}),
            },
        };

        McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result).unwrap()),
            error: None,
        }
    }

    async fn handle_list_tools(&self, request: McpRequest) -> McpResponse {
        let tools = vec![
            Tool {
                name: "send_email".to_string(),
                description: Some("Sends a new email".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "to": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of recipient email addresses"
                        },
                        "subject": {
                            "type": "string",
                            "description": "Email subject"
                        },
                        "body": {
                            "type": "string",
                            "description": "Email body content"
                        },
                        "htmlBody": {
                            "type": "string",
                            "description": "HTML version of the email body"
                        },
                        "mimeType": {
                            "type": "string",
                            "enum": ["text/plain", "text/html", "multipart/alternative"],
                            "description": "Email content type"
                        },
                        "cc": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of CC recipients"
                        },
                        "bcc": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of BCC recipients"
                        },
                        "threadId": {
                            "type": "string",
                            "description": "Thread ID to reply to"
                        },
                        "inReplyTo": {
                            "type": "string",
                            "description": "Message ID being replied to"
                        }
                    },
                    "required": ["to", "subject", "body"]
                }),
            },
            Tool {
                name: "draft_email".to_string(),
                description: Some("Create an email draft".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "to": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of recipient email addresses"
                        },
                        "subject": {
                            "type": "string",
                            "description": "Email subject"
                        },
                        "body": {
                            "type": "string",
                            "description": "Email body content"
                        },
                        "htmlBody": {
                            "type": "string",
                            "description": "HTML version of the email body"
                        },
                        "mimeType": {
                            "type": "string",
                            "enum": ["text/plain", "text/html", "multipart/alternative"],
                            "description": "Email content type"
                        },
                        "cc": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of CC recipients"
                        },
                        "bcc": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of BCC recipients"
                        },
                        "threadId": {
                            "type": "string",
                            "description": "Thread ID to reply to"
                        },
                        "inReplyTo": {
                            "type": "string",
                            "description": "Message ID being replied to"
                        }
                    },
                    "required": ["to", "subject", "body"]
                }),
            },
            Tool {
                name: "read_email".to_string(),
                description: Some("Retrieves the content of a specific email".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "messageId": {
                            "type": "string",
                            "description": "ID of the email message to retrieve"
                        }
                    },
                    "required": ["messageId"]
                }),
            },
            Tool {
                name: "search_emails".to_string(),
                description: Some("Searches for emails using Gmail search syntax".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Gmail search query (e.g., 'from:example@gmail.com')"
                        },
                        "maxResults": {
                            "type": "number",
                            "description": "Maximum number of results to return"
                        }
                    },
                    "required": ["query"]
                }),
            },
            Tool {
                name: "modify_email".to_string(),
                description: Some("Modifies email labels (move to different folders)".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "messageId": {
                            "type": "string",
                            "description": "ID of the email message to modify"
                        },
                        "labelIds": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of label IDs to apply"
                        },
                        "addLabelIds": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of label IDs to add to the message"
                        },
                        "removeLabelIds": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of label IDs to remove from the message"
                        }
                    },
                    "required": ["messageId"]
                }),
            },
            Tool {
                name: "delete_email".to_string(),
                description: Some("Permanently deletes an email".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "messageId": {
                            "type": "string",
                            "description": "ID of the email message to delete"
                        }
                    },
                    "required": ["messageId"]
                }),
            },
            Tool {
                name: "list_email_labels".to_string(),
                description: Some("Retrieves all available Gmail labels".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "create_label".to_string(),
                description: Some("Creates a new Gmail label".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name for the new label"
                        },
                        "messageListVisibility": {
                            "type": "string",
                            "enum": ["show", "hide"],
                            "description": "Whether to show or hide the label in the message list"
                        },
                        "labelListVisibility": {
                            "type": "string",
                            "enum": ["labelShow", "labelShowIfUnread", "labelHide"],
                            "description": "Visibility of the label in the label list"
                        }
                    },
                    "required": ["name"]
                }),
            },
            Tool {
                name: "update_label".to_string(),
                description: Some("Updates an existing Gmail label".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "ID of the label to update"
                        },
                        "name": {
                            "type": "string",
                            "description": "New name for the label"
                        },
                        "messageListVisibility": {
                            "type": "string",
                            "enum": ["show", "hide"],
                            "description": "Whether to show or hide the label in the message list"
                        },
                        "labelListVisibility": {
                            "type": "string",
                            "enum": ["labelShow", "labelShowIfUnread", "labelHide"],
                            "description": "Visibility of the label in the label list"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "delete_label".to_string(),
                description: Some("Deletes a Gmail label".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "ID of the label to delete"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "get_or_create_label".to_string(),
                description: Some("Gets an existing label by name or creates it if it doesn't exist".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the label to get or create"
                        },
                        "messageListVisibility": {
                            "type": "string",
                            "enum": ["show", "hide"],
                            "description": "Whether to show or hide the label in the message list"
                        },
                        "labelListVisibility": {
                            "type": "string",
                            "enum": ["labelShow", "labelShowIfUnread", "labelHide"],
                            "description": "Visibility of the label in the label list"
                        }
                    },
                    "required": ["name"]
                }),
            },
            Tool {
                name: "batch_modify_emails".to_string(),
                description: Some("Modifies labels for multiple emails in batches".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "messageIds": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of message IDs to modify"
                        },
                        "addLabelIds": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of label IDs to add to all messages"
                        },
                        "removeLabelIds": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of label IDs to remove from all messages"
                        },
                        "batchSize": {
                            "type": "number",
                            "description": "Number of messages to process in each batch (default: 50)"
                        }
                    },
                    "required": ["messageIds"]
                }),
            },
            Tool {
                name: "batch_delete_emails".to_string(),
                description: Some("Permanently deletes multiple emails in batches".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "messageIds": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of message IDs to delete"
                        },
                        "batchSize": {
                            "type": "number",
                            "description": "Number of messages to process in each batch (default: 50)"
                        }
                    },
                    "required": ["messageIds"]
                }),
            },
        ];

        let result = ListToolsResult { tools };

        McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result).unwrap()),
            error: None,
        }
    }

    async fn handle_call_tool(&self, request: McpRequest) -> McpResponse {
        let params = request.params.unwrap_or(json!({}));
        
        let call_request: CallToolRequest = match serde_json::from_value(params) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to parse tool call request: {}", e);
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Invalid params".to_string(),
                        data: Some(json!({"error": e.to_string()})),
                    }),
                };
            }
        };

        debug!("Calling tool: {}", call_request.name);

        let mut client_guard = self.client.lock().await;
        let result = match call_request.name.as_str() {
            "send_email" => GmailTools::send_email(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            "draft_email" => GmailTools::draft_email(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            "read_email" => GmailTools::read_email(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            "search_emails" => GmailTools::search_emails(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            "modify_email" => GmailTools::modify_email(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            "delete_email" => GmailTools::delete_email(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            "list_email_labels" => GmailTools::list_email_labels(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            "create_label" => GmailTools::create_label(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            "update_label" => GmailTools::update_label(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            "delete_label" => GmailTools::delete_label(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            "get_or_create_label" => GmailTools::get_or_create_label(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            "batch_modify_emails" => GmailTools::batch_modify_emails(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            "batch_delete_emails" => GmailTools::batch_delete_emails(&mut *client_guard, call_request.arguments.unwrap_or(json!({}))).await,
            _ => {
                error!("Unknown tool: {}", call_request.name);
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32601,
                        message: format!("Unknown tool: {}", call_request.name),
                        data: None,
                    }),
                };
            }
        };

        match result {
            Ok(tool_result) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::to_value(tool_result).unwrap()),
                error: None,
            },
            Err(e) => {
                error!("Tool call error: {}", e);
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32603,
                        message: "Internal error".to_string(),
                        data: Some(json!({"error": e.to_string()})),
                    }),
                }
            }
        }
    }
}