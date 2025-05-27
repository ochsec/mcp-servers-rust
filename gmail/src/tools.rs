use crate::client::GmailClient;
use crate::error::{GmailError, Result};
use crate::label_manager::LabelManager;
use crate::utils::{
    create_email_message, encode_message_for_gmail, extract_attachments, extract_email_content,
    format_email_for_display, get_header_value, SendEmailArgs,
};
use crate::mcp_types::{Content, CallToolResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadEmailArgs {
    #[serde(rename = "messageId")]
    pub message_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchEmailsArgs {
    pub query: String,
    #[serde(rename = "maxResults")]
    pub max_results: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModifyEmailArgs {
    #[serde(rename = "messageId")]
    pub message_id: String,
    #[serde(rename = "labelIds")]
    pub label_ids: Option<Vec<String>>,
    #[serde(rename = "addLabelIds")]
    pub add_label_ids: Option<Vec<String>>,
    #[serde(rename = "removeLabelIds")]
    pub remove_label_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteEmailArgs {
    #[serde(rename = "messageId")]
    pub message_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateLabelArgs {
    pub name: String,
    #[serde(rename = "messageListVisibility")]
    pub message_list_visibility: Option<String>,
    #[serde(rename = "labelListVisibility")]
    pub label_list_visibility: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateLabelArgs {
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "messageListVisibility")]
    pub message_list_visibility: Option<String>,
    #[serde(rename = "labelListVisibility")]
    pub label_list_visibility: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteLabelArgs {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetOrCreateLabelArgs {
    pub name: String,
    #[serde(rename = "messageListVisibility")]
    pub message_list_visibility: Option<String>,
    #[serde(rename = "labelListVisibility")]
    pub label_list_visibility: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchModifyEmailsArgs {
    #[serde(rename = "messageIds")]
    pub message_ids: Vec<String>,
    #[serde(rename = "addLabelIds")]
    pub add_label_ids: Option<Vec<String>>,
    #[serde(rename = "removeLabelIds")]
    pub remove_label_ids: Option<Vec<String>>,
    #[serde(rename = "batchSize")]
    pub batch_size: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchDeleteEmailsArgs {
    #[serde(rename = "messageIds")]
    pub message_ids: Vec<String>,
    #[serde(rename = "batchSize")]
    pub batch_size: Option<usize>,
}

pub struct GmailTools;

impl GmailTools {
    pub async fn send_email(client: &mut GmailClient, args: Value) -> Result<CallToolResult> {
        let args: SendEmailArgs = serde_json::from_value(args)
            .map_err(|e| GmailError::JsonError(e))?;

        let message = create_email_message(&args)?;
        let encoded_message = encode_message_for_gmail(&message);

        let response = client.send_message(&encoded_message, args.thread_id).await?;

        Ok(CallToolResult {
            content: vec![Content::text(format!("Email sent successfully with ID: {}", response.id))],
            is_error: Some(false),
        })
    }

    pub async fn draft_email(client: &mut GmailClient, args: Value) -> Result<CallToolResult> {
        let args: SendEmailArgs = serde_json::from_value(args)
            .map_err(|e| GmailError::JsonError(e))?;

        let message = create_email_message(&args)?;
        let encoded_message = encode_message_for_gmail(&message);

        let response = client.create_draft(&encoded_message, args.thread_id).await?;

        let draft_id = response
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        Ok(CallToolResult {
            content: vec![Content::text(format!("Email draft created successfully with ID: {}", draft_id))],
            is_error: Some(false),
        })
    }

    pub async fn read_email(client: &mut GmailClient, args: Value) -> Result<CallToolResult> {
        let args: ReadEmailArgs = serde_json::from_value(args)
            .map_err(|e| GmailError::JsonError(e))?;

        let message = client.get_message(&args.message_id, Some("full")).await?;

        let content = if let Some(payload) = &message.payload {
            extract_email_content(payload)
        } else {
            crate::utils::EmailContent {
                text: String::new(),
                html: String::new(),
            }
        };

        let attachments = if let Some(payload) = &message.payload {
            extract_attachments(payload)
        } else {
            vec![]
        };

        let formatted_message = format_email_for_display(&message, &content, &attachments);

        Ok(CallToolResult {
            content: vec![Content::text(formatted_message)],
            is_error: Some(false),
        })
    }

    pub async fn search_emails(client: &mut GmailClient, args: Value) -> Result<CallToolResult> {
        let args: SearchEmailsArgs = serde_json::from_value(args)
            .map_err(|e| GmailError::JsonError(e))?;

        let response = client
            .search_messages(&args.query, args.max_results)
            .await?;

        let messages = response.messages.unwrap_or_default();

        let mut results = Vec::new();
        for msg_ref in messages {
            let detail = client
                .get_message(&msg_ref.id, Some("metadata"))
                .await?;

            let empty_headers = vec![];
            let headers = detail
                .payload
                .as_ref()
                .and_then(|p| p.headers.as_ref())
                .unwrap_or(&empty_headers);

            let subject = get_header_value(headers, "Subject").unwrap_or_default();
            let from = get_header_value(headers, "From").unwrap_or_default();
            let date = get_header_value(headers, "Date").unwrap_or_default();

            results.push(format!(
                "ID: {}\nSubject: {}\nFrom: {}\nDate: {}\n",
                msg_ref.id, subject, from, date
            ));
        }

        Ok(CallToolResult {
            content: vec![Content::text(results.join("\n"))],
            is_error: Some(false),
        })
    }

    pub async fn modify_email(client: &mut GmailClient, args: Value) -> Result<CallToolResult> {
        let args: ModifyEmailArgs = serde_json::from_value(args)
            .map_err(|e| GmailError::JsonError(e))?;

        let add_labels = args.label_ids.or(args.add_label_ids);
        let remove_labels = args.remove_label_ids;

        client
            .modify_message(&args.message_id, add_labels, remove_labels)
            .await?;

        Ok(CallToolResult {
            content: vec![Content::text(format!("Email {} labels updated successfully", args.message_id))],
            is_error: Some(false),
        })
    }

    pub async fn delete_email(client: &mut GmailClient, args: Value) -> Result<CallToolResult> {
        let args: DeleteEmailArgs = serde_json::from_value(args)
            .map_err(|e| GmailError::JsonError(e))?;

        client.delete_message(&args.message_id).await?;

        Ok(CallToolResult {
            content: vec![Content::text(format!("Email {} deleted successfully", args.message_id))],
            is_error: Some(false),
        })
    }

    pub async fn list_email_labels(client: &mut GmailClient, _args: Value) -> Result<CallToolResult> {
        let label_results = LabelManager::list_labels(client).await?;

        let text = format!(
            "Found {} labels ({} system, {} user):\n\nSystem Labels:\n{}\n\nUser Labels:\n{}",
            label_results.count.total,
            label_results.count.system,
            label_results.count.user,
            label_results
                .system
                .iter()
                .map(|l| format!("ID: {}\nName: {}\n", l.id, l.name))
                .collect::<Vec<_>>()
                .join("\n"),
            label_results
                .user
                .iter()
                .map(|l| format!("ID: {}\nName: {}\n", l.id, l.name))
                .collect::<Vec<_>>()
                .join("\n")
        );

        Ok(CallToolResult {
            content: vec![Content::text(text)],
            is_error: Some(false),
        })
    }

    pub async fn create_label(client: &mut GmailClient, args: Value) -> Result<CallToolResult> {
        let args: CreateLabelArgs = serde_json::from_value(args)
            .map_err(|e| GmailError::JsonError(e))?;

        let result = LabelManager::create_label(
            client,
            &args.name,
            args.message_list_visibility,
            args.label_list_visibility,
        )
        .await?;

        Ok(CallToolResult {
            content: vec![Content::text(format!(
                "Label created successfully:\nID: {}\nName: {}\nType: {}",
                result.id,
                result.name,
                result.label_type.as_deref().unwrap_or("unknown")
            ))],
            is_error: Some(false),
        })
    }

    pub async fn update_label(client: &mut GmailClient, args: Value) -> Result<CallToolResult> {
        let args: UpdateLabelArgs = serde_json::from_value(args)
            .map_err(|e| GmailError::JsonError(e))?;

        let mut updates = HashMap::new();
        
        if let Some(name) = args.name {
            updates.insert("name".to_string(), json!(name));
        }
        if let Some(visibility) = args.message_list_visibility {
            updates.insert("messageListVisibility".to_string(), json!(visibility));
        }
        if let Some(visibility) = args.label_list_visibility {
            updates.insert("labelListVisibility".to_string(), json!(visibility));
        }

        let result = LabelManager::update_label(client, &args.id, updates).await?;

        Ok(CallToolResult {
            content: vec![Content::text(format!(
                "Label updated successfully:\nID: {}\nName: {}\nType: {}",
                result.id,
                result.name,
                result.label_type.as_deref().unwrap_or("unknown")
            ))],
            is_error: Some(false),
        })
    }

    pub async fn delete_label(client: &mut GmailClient, args: Value) -> Result<CallToolResult> {
        let args: DeleteLabelArgs = serde_json::from_value(args)
            .map_err(|e| GmailError::JsonError(e))?;

        let result = LabelManager::delete_label(client, &args.id).await?;

        Ok(CallToolResult {
            content: vec![Content::text(result.message)],
            is_error: Some(false),
        })
    }

    pub async fn get_or_create_label(client: &mut GmailClient, args: Value) -> Result<CallToolResult> {
        let args: GetOrCreateLabelArgs = serde_json::from_value(args)
            .map_err(|e| GmailError::JsonError(e))?;

        let result = LabelManager::get_or_create_label(
            client,
            &args.name,
            args.message_list_visibility,
            args.label_list_visibility,
        )
        .await?;

        let action = if result.label_type.as_deref() == Some("user") && result.name == args.name {
            "found existing"
        } else {
            "created new"
        };

        Ok(CallToolResult {
            content: vec![Content::text(format!(
                "Successfully {} label:\nID: {}\nName: {}\nType: {}",
                action,
                result.id,
                result.name,
                result.label_type.as_deref().unwrap_or("unknown")
            ))],
            is_error: Some(false),
        })
    }

    pub async fn batch_modify_emails(client: &mut GmailClient, args: Value) -> Result<CallToolResult> {
        let args: BatchModifyEmailsArgs = serde_json::from_value(args)
            .map_err(|e| GmailError::JsonError(e))?;

        let _batch_size = args.batch_size.unwrap_or(50);
        let results = client
            .batch_modify_messages(&args.message_ids, args.add_label_ids, args.remove_label_ids)
            .await?;

        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let failure_count = results.len() - success_count;

        let mut result_text = format!("Batch label modification complete.\nSuccessfully processed: {} messages\n", success_count);

        if failure_count > 0 {
            result_text.push_str(&format!("Failed to process: {} messages\n\nFailed message IDs:\n", failure_count));
            for (i, result) in results.iter().enumerate() {
                if let Err(e) = result {
                    let message_id = args.message_ids.get(i).map(|id| &id[..16.min(id.len())]).unwrap_or("unknown");
                    result_text.push_str(&format!("- {}... ({})\n", message_id, e));
                }
            }
        }

        Ok(CallToolResult {
            content: vec![Content::text(result_text)],
            is_error: Some(false),
        })
    }

    pub async fn batch_delete_emails(client: &mut GmailClient, args: Value) -> Result<CallToolResult> {
        let args: BatchDeleteEmailsArgs = serde_json::from_value(args)
            .map_err(|e| GmailError::JsonError(e))?;

        let _batch_size = args.batch_size.unwrap_or(50);
        let results = client.batch_delete_messages(&args.message_ids).await?;

        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let failure_count = results.len() - success_count;

        let mut result_text = format!("Batch delete operation complete.\nSuccessfully deleted: {} messages\n", success_count);

        if failure_count > 0 {
            result_text.push_str(&format!("Failed to delete: {} messages\n\nFailed message IDs:\n", failure_count));
            for (i, result) in results.iter().enumerate() {
                if let Err(e) = result {
                    let message_id = args.message_ids.get(i).map(|id| &id[..16.min(id.len())]).unwrap_or("unknown");
                    result_text.push_str(&format!("- {}... ({})\n", message_id, e));
                }
            }
        }

        Ok(CallToolResult {
            content: vec![Content::text(result_text)],
            is_error: Some(false),
        })
    }
}