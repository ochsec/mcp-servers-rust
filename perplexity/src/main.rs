use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use tracing::{debug, error};

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolInput {
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MCPRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MCPResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PerplexityRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PerplexityChoice {
    message: Message,
}

#[derive(Debug, Serialize, Deserialize)]
struct PerplexityResponse {
    choices: Vec<PerplexityChoice>,
    #[serde(default)]
    citations: Vec<String>,
}

struct MCPServer {
    client: reqwest::Client,
    api_key: String,
}

impl MCPServer {
    fn new() -> Result<Self> {
        let api_key = env::var("PERPLEXITY_API_KEY")
            .map_err(|_| anyhow!("PERPLEXITY_API_KEY environment variable is required"))?;
        
        let client = reqwest::Client::new();
        
        Ok(MCPServer { client, api_key })
    }

    async fn perform_chat_completion(&self, messages: Vec<Message>, model: &str) -> Result<String> {
        let url = "https://api.perplexity.ai/chat/completions";
        let request_body = PerplexityRequest {
            model: model.to_string(),
            messages,
        };

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| anyhow!("Network error while calling Perplexity API: {}", e))?;

        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            let status_text = response.status().canonical_reason().unwrap_or("Unknown");
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Perplexity API error: {} {}\n{}",
                status_code,
                status_text,
                error_text
            ));
        }

        let data: PerplexityResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse JSON response from Perplexity API: {}", e))?;

        let mut message_content = data
            .choices
            .first()
            .ok_or_else(|| anyhow!("No choices in response"))?
            .message
            .content
            .clone();

        if !data.citations.is_empty() {
            message_content.push_str("\n\nCitations:\n");
            for (index, citation) in data.citations.iter().enumerate() {
                message_content.push_str(&format!("[{}] {}\n", index + 1, citation));
            }
        }

        Ok(message_content)
    }

    fn get_tools(&self) -> Value {
        json!({
            "tools": [
                {
                    "name": "perplexity_ask",
                    "description": "Engages in a conversation using the Sonar API. Accepts an array of messages (each with a role and content) and returns a ask completion response from the Perplexity model.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "messages": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "role": {
                                            "type": "string",
                                            "description": "Role of the message (e.g., system, user, assistant)"
                                        },
                                        "content": {
                                            "type": "string",
                                            "description": "The content of the message"
                                        }
                                    },
                                    "required": ["role", "content"]
                                },
                                "description": "Array of conversation messages"
                            }
                        },
                        "required": ["messages"]
                    }
                },
                {
                    "name": "perplexity_research",
                    "description": "Performs deep research using the Perplexity API. Accepts an array of messages (each with a role and content) and returns a comprehensive research response with citations.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "messages": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "role": {
                                            "type": "string",
                                            "description": "Role of the message (e.g., system, user, assistant)"
                                        },
                                        "content": {
                                            "type": "string",
                                            "description": "The content of the message"
                                        }
                                    },
                                    "required": ["role", "content"]
                                },
                                "description": "Array of conversation messages"
                            }
                        },
                        "required": ["messages"]
                    }
                },
                {
                    "name": "perplexity_reason",
                    "description": "Performs reasoning tasks using the Perplexity API. Accepts an array of messages (each with a role and content) and returns a well-reasoned response using the sonar-reasoning-pro model.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "messages": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "role": {
                                            "type": "string",
                                            "description": "Role of the message (e.g., system, user, assistant)"
                                        },
                                        "content": {
                                            "type": "string",
                                            "description": "The content of the message"
                                        }
                                    },
                                    "required": ["role", "content"]
                                },
                                "description": "Array of conversation messages"
                            }
                        },
                        "required": ["messages"]
                    }
                }
            ]
        })
    }

    async fn handle_tool_call(&self, name: &str, arguments: &Value) -> Result<Value> {
        let tool_input: ToolInput = serde_json::from_value(arguments.clone())
            .map_err(|_| anyhow!("Invalid arguments: 'messages' must be an array"))?;

        let result = match name {
            "perplexity_ask" => {
                self.perform_chat_completion(tool_input.messages, "sonar-pro")
                    .await?
            }
            "perplexity_research" => {
                self.perform_chat_completion(tool_input.messages, "sonar-deep-research")
                    .await?
            }
            "perplexity_reason" => {
                self.perform_chat_completion(tool_input.messages, "sonar-reasoning-pro")
                    .await?
            }
            _ => return Err(anyhow!("Unknown tool: {}", name)),
        };

        Ok(json!({
            "content": [{"type": "text", "text": result}],
            "isError": false
        }))
    }

    async fn handle_request(&self, request: MCPRequest) -> MCPResponse {
        let id = request.id.clone();

        match request.method.as_str() {
            "tools/list" => MCPResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(self.get_tools()),
                error: None,
            },
            "tools/call" => {
                if let Some(params) = request.params {
                    if let (Some(name), Some(arguments)) = (
                        params.get("name").and_then(|v| v.as_str()),
                        params.get("arguments"),
                    ) {
                        match self.handle_tool_call(name, arguments).await {
                            Ok(result) => MCPResponse {
                                jsonrpc: "2.0".to_string(),
                                id,
                                result: Some(result),
                                error: None,
                            },
                            Err(e) => MCPResponse {
                                jsonrpc: "2.0".to_string(),
                                id,
                                result: Some(json!({
                                    "content": [{"type": "text", "text": format!("Error: {}", e)}],
                                    "isError": true
                                })),
                                error: None,
                            },
                        }
                    } else {
                        MCPResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(json!({
                                "code": -32602,
                                "message": "Invalid params"
                            })),
                        }
                    }
                } else {
                    MCPResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(json!({
                            "code": -32602,
                            "message": "Missing params"
                        })),
                    }
                }
            }
            "initialize" => MCPResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "mcp-perplexity-ask",
                        "version": "0.1.0"
                    }
                })),
                error: None,
            },
            _ => MCPResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(json!({
                    "code": -32601,
                    "message": "Method not found"
                })),
            },
        }
    }

    async fn run(&self) -> Result<()> {
        eprintln!("Perplexity MCP Server running on stdio with Ask, Research, and Reason tools");
        
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let reader = BufReader::new(stdin);

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            debug!("Received: {}", line);

            match serde_json::from_str::<MCPRequest>(&line) {
                Ok(request) => {
                    let response = self.handle_request(request).await;
                    let response_json = serde_json::to_string(&response)?;
                    println!("{}", response_json);
                    stdout.flush()?;
                }
                Err(e) => {
                    error!("Failed to parse request: {}", e);
                    let error_response = MCPResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(json!({
                            "code": -32700,
                            "message": "Parse error"
                        })),
                    };
                    let response_json = serde_json::to_string(&error_response)?;
                    println!("{}", response_json);
                    stdout.flush()?;
                }
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter("mcp_perplexity_ask=debug")
        .init();

    let server = MCPServer::new()?;
    server.run().await
}