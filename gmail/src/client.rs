use crate::auth::GoogleAuth;
use crate::error::{GmailError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error};

const GMAIL_API_BASE: &str = "https://gmail.googleapis.com/gmail/v1";

#[derive(Debug, Serialize, Deserialize)]
pub struct GmailMessage {
    pub id: String,
    #[serde(rename = "threadId")]
    pub thread_id: Option<String>,
    #[serde(rename = "labelIds")]
    pub label_ids: Option<Vec<String>>,
    pub snippet: Option<String>,
    pub payload: Option<MessagePayload>,
    #[serde(rename = "sizeEstimate")]
    pub size_estimate: Option<u64>,
    #[serde(rename = "historyId")]
    pub history_id: Option<String>,
    #[serde(rename = "internalDate")]
    pub internal_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessagePayload {
    #[serde(rename = "partId")]
    pub part_id: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    pub headers: Option<Vec<MessageHeader>>,
    pub body: Option<MessageBody>,
    pub parts: Option<Vec<MessagePayload>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageBody {
    #[serde(rename = "attachmentId")]
    pub attachment_id: Option<String>,
    pub size: Option<u64>,
    pub data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailLabel {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub label_type: Option<String>,
    #[serde(rename = "messageListVisibility")]
    pub message_list_visibility: Option<String>,
    #[serde(rename = "labelListVisibility")]
    pub label_list_visibility: Option<String>,
    #[serde(rename = "messagesTotal")]
    pub messages_total: Option<u32>,
    #[serde(rename = "messagesUnread")]
    pub messages_unread: Option<u32>,
    pub color: Option<LabelColor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelColor {
    #[serde(rename = "textColor")]
    pub text_color: Option<String>,
    #[serde(rename = "backgroundColor")]
    pub background_color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageListResponse {
    pub messages: Option<Vec<MessageRef>>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "resultSizeEstimate")]
    pub result_size_estimate: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageRef {
    pub id: String,
    #[serde(rename = "threadId")]
    pub thread_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LabelListResponse {
    pub labels: Option<Vec<GmailLabel>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub raw: String,
    #[serde(rename = "threadId", skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModifyMessageRequest {
    #[serde(rename = "addLabelIds", skip_serializing_if = "Option::is_none")]
    pub add_label_ids: Option<Vec<String>>,
    #[serde(rename = "removeLabelIds", skip_serializing_if = "Option::is_none")]
    pub remove_label_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateLabelRequest {
    pub name: String,
    #[serde(rename = "messageListVisibility", skip_serializing_if = "Option::is_none")]
    pub message_list_visibility: Option<String>,
    #[serde(rename = "labelListVisibility", skip_serializing_if = "Option::is_none")]
    pub label_list_visibility: Option<String>,
}

pub struct GmailClient {
    client: Client,
    auth: GoogleAuth,
}

impl GmailClient {
    pub async fn new() -> Result<Self> {
        let auth = GoogleAuth::new().await?;
        let client = Client::new();
        
        Ok(Self { client, auth })
    }

    pub async fn authenticate(&mut self, callback_url: &str) -> Result<()> {
        self.auth.authenticate(callback_url).await
    }

    async fn make_request<T>(&mut self, method: &str, endpoint: &str, body: Option<Value>) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.auth.refresh_token_if_needed().await?;
        let token = self.auth.get_access_token()?;
        
        let url = format!("{}/{}", GMAIL_API_BASE, endpoint);
        debug!("Making {} request to: {}", method, url);

        let mut request = match method {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => return Err(GmailError::ApiError(format!("Unsupported HTTP method: {}", method))),
        };

        request = request.bearer_auth(token);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("API request failed with status {}: {}", status, error_text);
            return Err(GmailError::ApiError(format!("HTTP {}: {}", status, error_text)));
        }

        let json_response: T = response.json().await?;
        Ok(json_response)
    }

    // Message operations
    pub async fn send_message(&mut self, raw_message: &str, thread_id: Option<String>) -> Result<GmailMessage> {
        let request = SendMessageRequest {
            raw: raw_message.to_string(),
            thread_id,
        };
        
        self.make_request("POST", "users/me/messages/send", Some(serde_json::to_value(request)?)).await
    }

    pub async fn create_draft(&mut self, raw_message: &str, thread_id: Option<String>) -> Result<Value> {
        let message_request = SendMessageRequest {
            raw: raw_message.to_string(),
            thread_id,
        };
        
        let draft_request = serde_json::json!({
            "message": message_request
        });
        
        self.make_request("POST", "users/me/drafts", Some(draft_request)).await
    }

    pub async fn get_message(&mut self, message_id: &str, format: Option<&str>) -> Result<GmailMessage> {
        let endpoint = match format {
            Some(fmt) => format!("users/me/messages/{}?format={}", message_id, fmt),
            None => format!("users/me/messages/{}", message_id),
        };
        
        self.make_request("GET", &endpoint, None).await
    }

    pub async fn search_messages(&mut self, query: &str, max_results: Option<u32>) -> Result<MessageListResponse> {
        let mut endpoint = format!("users/me/messages?q={}", urlencoding::encode(query));
        
        if let Some(max) = max_results {
            endpoint.push_str(&format!("&maxResults={}", max));
        }
        
        self.make_request("GET", &endpoint, None).await
    }

    pub async fn modify_message(&mut self, message_id: &str, add_labels: Option<Vec<String>>, remove_labels: Option<Vec<String>>) -> Result<GmailMessage> {
        let request = ModifyMessageRequest {
            add_label_ids: add_labels,
            remove_label_ids: remove_labels,
        };
        
        let endpoint = format!("users/me/messages/{}/modify", message_id);
        self.make_request("POST", &endpoint, Some(serde_json::to_value(request)?)).await
    }

    pub async fn delete_message(&mut self, message_id: &str) -> Result<()> {
        let endpoint = format!("users/me/messages/{}", message_id);
        let _: Value = self.make_request("DELETE", &endpoint, None).await?;
        Ok(())
    }

    // Label operations
    pub async fn list_labels(&mut self) -> Result<LabelListResponse> {
        self.make_request("GET", "users/me/labels", None).await
    }

    pub async fn create_label(&mut self, name: &str, message_list_visibility: Option<String>, label_list_visibility: Option<String>) -> Result<GmailLabel> {
        let request = CreateLabelRequest {
            name: name.to_string(),
            message_list_visibility,
            label_list_visibility,
        };
        
        self.make_request("POST", "users/me/labels", Some(serde_json::to_value(request)?)).await
    }

    pub async fn get_label(&mut self, label_id: &str) -> Result<GmailLabel> {
        let endpoint = format!("users/me/labels/{}", label_id);
        self.make_request("GET", &endpoint, None).await
    }

    pub async fn update_label(&mut self, label_id: &str, updates: HashMap<String, Value>) -> Result<GmailLabel> {
        let endpoint = format!("users/me/labels/{}", label_id);
        self.make_request("PUT", &endpoint, Some(serde_json::to_value(updates)?)).await
    }

    pub async fn delete_label(&mut self, label_id: &str) -> Result<()> {
        let endpoint = format!("users/me/labels/{}", label_id);
        let _: Value = self.make_request("DELETE", &endpoint, None).await?;
        Ok(())
    }

    // Batch operations
    pub async fn batch_modify_messages(&mut self, message_ids: &[String], add_labels: Option<Vec<String>>, remove_labels: Option<Vec<String>>) -> Result<Vec<std::result::Result<GmailMessage, GmailError>>> {
        let mut results = Vec::new();
        
        for message_id in message_ids {
            let result = self.modify_message(message_id, add_labels.clone(), remove_labels.clone()).await;
            results.push(result);
        }
        
        Ok(results)
    }

    pub async fn batch_delete_messages(&mut self, message_ids: &[String]) -> Result<Vec<std::result::Result<(), GmailError>>> {
        let mut results = Vec::new();
        
        for message_id in message_ids {
            let result = self.delete_message(message_id).await;
            results.push(result);
        }
        
        Ok(results)
    }
}