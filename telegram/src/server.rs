use crate::config::TelegramConfig;
use crate::error::TelegramError;
use crate::telegram::TelegramClient;
use crate::types::{Dialog, DownloadedMedia, Message, Messages};
use crate::utils::parse_entity;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_mcp_sdk::{McpServer, Tool, ToolHandler, ToolResult};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

pub struct McpTelegramServer {
    telegram: Arc<RwLock<TelegramClient>>,
}

impl McpTelegramServer {
    pub async fn new() -> Result<Self> {
        let config = TelegramConfig::from_env()?;
        let mut telegram_client = TelegramClient::new(config)?;
        telegram_client.connect().await?;

        Ok(Self {
            telegram: Arc::new(RwLock::new(telegram_client)),
        })
    }
}

// Tool handlers
struct SendMessageHandler {
    telegram: Arc<RwLock<TelegramClient>>,
}

#[async_trait]
impl ToolHandler for SendMessageHandler {
    async fn execute(&self, params: Value) -> ToolResult {
        let entity = params["entity"].as_str().ok_or("Missing entity")?;
        let message = params["message"].as_str().unwrap_or("");
        let file_paths = params["file_path"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect::<Vec<_>>()
            });
        let reply_to = params["reply_to"].as_i64().map(|id| id as i32);

        let telegram = self.telegram.read().await;
        if !telegram.is_authorized().await {
            return Err("Not authorized. Please run login first.".into());
        }

        telegram
            .send_message(
                entity,
                message,
                file_paths.as_ref().map(|v| v.as_slice()),
                reply_to,
            )
            .await
            .map_err(|e| format!("Failed to send message: {}", e))?;

        Ok(json!(format!("Message sent to {}", entity)))
    }
}

struct EditMessageHandler {
    telegram: Arc<RwLock<TelegramClient>>,
}

#[async_trait]
impl ToolHandler for EditMessageHandler {
    async fn execute(&self, params: Value) -> ToolResult {
        let entity = params["entity"].as_str().ok_or("Missing entity")?;
        let message_id = params["message_id"].as_i64().ok_or("Missing message_id")? as i32;
        let message = params["message"].as_str().ok_or("Missing message")?;

        let telegram = self.telegram.read().await;
        if !telegram.is_authorized().await {
            return Err("Not authorized. Please run login first.".into());
        }

        telegram
            .edit_message(entity, message_id, message)
            .await
            .map_err(|e| format!("Failed to edit message: {}", e))?;

        Ok(json!(format!("Message edited in {}", entity)))
    }
}

struct DeleteMessageHandler {
    telegram: Arc<RwLock<TelegramClient>>,
}

#[async_trait]
impl ToolHandler for DeleteMessageHandler {
    async fn execute(&self, params: Value) -> ToolResult {
        let entity = params["entity"].as_str().ok_or("Missing entity")?;
        let message_ids = params["message_ids"]
            .as_array()
            .ok_or("Missing message_ids")?
            .iter()
            .filter_map(|v| v.as_i64())
            .map(|id| id as i32)
            .collect::<Vec<_>>();

        let telegram = self.telegram.read().await;
        if !telegram.is_authorized().await {
            return Err("Not authorized. Please run login first.".into());
        }

        telegram
            .delete_messages(entity, &message_ids)
            .await
            .map_err(|e| format!("Failed to delete messages: {}", e))?;

        Ok(json!(format!("Messages deleted from {}", entity)))
    }
}

struct SearchDialogsHandler {
    telegram: Arc<RwLock<TelegramClient>>,
}

#[async_trait]
impl ToolHandler for SearchDialogsHandler {
    async fn execute(&self, params: Value) -> ToolResult {
        let query = params["query"].as_str().ok_or("Missing query")?;
        let limit = params["limit"].as_i64().unwrap_or(10) as usize;
        let global_search = params["global_search"].as_bool().unwrap_or(false);

        let telegram = self.telegram.read().await;
        if !telegram.is_authorized().await {
            return Err("Not authorized. Please run login first.".into());
        }

        let dialogs = telegram
            .search_dialogs(query, limit, global_search)
            .await
            .map_err(|e| format!("Failed to search dialogs: {}", e))?;

        Ok(serde_json::to_value(dialogs)?)
    }
}

struct GetDraftHandler {
    telegram: Arc<RwLock<TelegramClient>>,
}

#[async_trait]
impl ToolHandler for GetDraftHandler {
    async fn execute(&self, params: Value) -> ToolResult {
        let entity = params["entity"].as_str().ok_or("Missing entity")?;

        let telegram = self.telegram.read().await;
        if !telegram.is_authorized().await {
            return Err("Not authorized. Please run login first.".into());
        }

        let draft = telegram
            .get_draft(entity)
            .await
            .map_err(|e| format!("Failed to get draft: {}", e))?;

        Ok(json!(draft))
    }
}

struct SetDraftHandler {
    telegram: Arc<RwLock<TelegramClient>>,
}

#[async_trait]
impl ToolHandler for SetDraftHandler {
    async fn execute(&self, params: Value) -> ToolResult {
        let entity = params["entity"].as_str().ok_or("Missing entity")?;
        let message = params["message"].as_str().ok_or("Missing message")?;

        let telegram = self.telegram.read().await;
        if !telegram.is_authorized().await {
            return Err("Not authorized. Please run login first.".into());
        }

        telegram
            .set_draft(entity, message)
            .await
            .map_err(|e| format!("Failed to set draft: {}", e))?;

        Ok(json!(format!("Draft saved for {}", entity)))
    }
}

struct GetMessagesHandler {
    telegram: Arc<RwLock<TelegramClient>>,
}

#[async_trait]
impl ToolHandler for GetMessagesHandler {
    async fn execute(&self, params: Value) -> ToolResult {
        let entity = params["entity"].as_str().ok_or("Missing entity")?;
        let limit = params["limit"].as_i64().unwrap_or(10) as usize;
        let start_date = params["start_date"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let end_date = params["end_date"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let unread = params["unread"].as_bool().unwrap_or(false);
        let mark_as_read = params["mark_as_read"].as_bool().unwrap_or(false);

        let telegram = self.telegram.read().await;
        if !telegram.is_authorized().await {
            return Err("Not authorized. Please run login first.".into());
        }

        let messages = telegram
            .get_messages(entity, limit, start_date, end_date, unread, mark_as_read)
            .await
            .map_err(|e| format!("Failed to get messages: {}", e))?;

        Ok(serde_json::to_value(messages)?)
    }
}

struct MediaDownloadHandler {
    telegram: Arc<RwLock<TelegramClient>>,
}

#[async_trait]
impl ToolHandler for MediaDownloadHandler {
    async fn execute(&self, params: Value) -> ToolResult {
        let entity = params["entity"].as_str().ok_or("Missing entity")?;
        let message_id = params["message_id"].as_i64().ok_or("Missing message_id")? as i32;
        let path = params["path"].as_str();

        let telegram = self.telegram.read().await;
        if !telegram.is_authorized().await {
            return Err("Not authorized. Please run login first.".into());
        }

        let downloaded_media = telegram
            .download_media(entity, message_id, path)
            .await
            .map_err(|e| format!("Failed to download media: {}", e))?;

        Ok(serde_json::to_value(downloaded_media)?)
    }
}

struct MessageFromLinkHandler {
    telegram: Arc<RwLock<TelegramClient>>,
}

#[async_trait]
impl ToolHandler for MessageFromLinkHandler {
    async fn execute(&self, params: Value) -> ToolResult {
        let link = params["link"].as_str().ok_or("Missing link")?;

        let telegram = self.telegram.read().await;
        if !telegram.is_authorized().await {
            return Err("Not authorized. Please run login first.".into());
        }

        let message = telegram
            .message_from_link(link)
            .await
            .map_err(|e| format!("Failed to get message from link: {}", e))?;

        Ok(serde_json::to_value(message)?)
    }
}

pub async fn run() -> Result<()> {
    let telegram_server = McpTelegramServer::new().await?;
    let telegram = telegram_server.telegram;

    let mut server = McpServer::new("mcp-telegram", "0.1.0");

    // Create tools
    let tools = vec![
        Tool::new(
            "send_message",
            "Send a message to a Telegram user, group, or channel",
            json!({
                "type": "object",
                "properties": {
                    "entity": {
                        "type": "string",
                        "description": "The identifier of where to send the message"
                    },
                    "message": {
                        "type": "string",
                        "description": "The text message to be sent",
                        "default": ""
                    },
                    "file_path": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "The list of paths to the files to be sent"
                    },
                    "reply_to": {
                        "type": "integer",
                        "description": "The message ID to reply to"
                    }
                },
                "required": ["entity"]
            }),
            Box::new(SendMessageHandler {
                telegram: Arc::clone(&telegram),
            }),
        ),
        Tool::new(
            "edit_message",
            "Edit a message from a specific entity",
            json!({
                "type": "object",
                "properties": {
                    "entity": {"type": "string"},
                    "message_id": {"type": "integer"},
                    "message": {"type": "string"}
                },
                "required": ["entity", "message_id", "message"]
            }),
            Box::new(EditMessageHandler {
                telegram: Arc::clone(&telegram),
            }),
        ),
        Tool::new(
            "delete_message",
            "Delete messages from a specific entity",
            json!({
                "type": "object",
                "properties": {
                    "entity": {"type": "string"},
                    "message_ids": {
                        "type": "array",
                        "items": {"type": "integer"}
                    }
                },
                "required": ["entity", "message_ids"]
            }),
            Box::new(DeleteMessageHandler {
                telegram: Arc::clone(&telegram),
            }),
        ),
        Tool::new(
            "search_dialogs",
            "Search for users, groups, and channels",
            json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "default": 10},
                    "global_search": {"type": "boolean", "default": false}
                },
                "required": ["query"]
            }),
            Box::new(SearchDialogsHandler {
                telegram: Arc::clone(&telegram),
            }),
        ),
        Tool::new(
            "get_draft",
            "Get the draft message for a specific entity",
            json!({
                "type": "object",
                "properties": {
                    "entity": {"type": "string"}
                },
                "required": ["entity"]
            }),
            Box::new(GetDraftHandler {
                telegram: Arc::clone(&telegram),
            }),
        ),
        Tool::new(
            "set_draft",
            "Set a draft message for a specific entity",
            json!({
                "type": "object",
                "properties": {
                    "entity": {"type": "string"},
                    "message": {"type": "string"}
                },
                "required": ["entity", "message"]
            }),
            Box::new(SetDraftHandler {
                telegram: Arc::clone(&telegram),
            }),
        ),
        Tool::new(
            "get_messages",
            "Get messages from a specific entity",
            json!({
                "type": "object",
                "properties": {
                    "entity": {"type": "string"},
                    "limit": {"type": "integer", "default": 10},
                    "start_date": {"type": "string", "format": "date-time"},
                    "end_date": {"type": "string", "format": "date-time"},
                    "unread": {"type": "boolean", "default": false},
                    "mark_as_read": {"type": "boolean", "default": false}
                },
                "required": ["entity"]
            }),
            Box::new(GetMessagesHandler {
                telegram: Arc::clone(&telegram),
            }),
        ),
        Tool::new(
            "media_download",
            "Download media from a specific message to a unique local file",
            json!({
                "type": "object",
                "properties": {
                    "entity": {"type": "string"},
                    "message_id": {"type": "integer"},
                    "path": {"type": "string"}
                },
                "required": ["entity", "message_id"]
            }),
            Box::new(MediaDownloadHandler {
                telegram: Arc::clone(&telegram),
            }),
        ),
        Tool::new(
            "message_from_link",
            "Get a message from a link",
            json!({
                "type": "object",
                "properties": {
                    "link": {"type": "string"}
                },
                "required": ["link"]
            }),
            Box::new(MessageFromLinkHandler {
                telegram: Arc::clone(&telegram),
            }),
        ),
    ];

    // Register all tools
    for tool in tools {
        server.add_tool(tool);
    }

    info!("Starting MCP Telegram server");
    server.run().await?;

    Ok(())
}