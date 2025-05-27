use crate::client::{GmailClient, GmailLabel};
use crate::error::{GmailError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct LabelManagerResult {
    pub system: Vec<GmailLabel>,
    pub user: Vec<GmailLabel>,
    pub count: LabelCount,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LabelCount {
    pub total: usize,
    pub system: usize,
    pub user: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteLabelResult {
    pub success: bool,
    pub message: String,
}

pub struct LabelManager;

impl LabelManager {
    /// Creates a new Gmail label
    pub async fn create_label(
        client: &mut GmailClient,
        label_name: &str,
        message_list_visibility: Option<String>,
        label_list_visibility: Option<String>,
    ) -> Result<GmailLabel> {
        let message_visibility = message_list_visibility.unwrap_or_else(|| "show".to_string());
        let label_visibility = label_list_visibility.unwrap_or_else(|| "labelShow".to_string());

        match client.create_label(label_name, Some(message_visibility), Some(label_visibility)).await {
            Ok(label) => Ok(label),
            Err(GmailError::ApiError(msg)) if msg.contains("already exists") => {
                Err(GmailError::ApiError(format!("Label \"{}\" already exists. Please use a different name.", label_name)))
            }
            Err(e) => Err(GmailError::ApiError(format!("Failed to create label: {}", e))),
        }
    }

    /// Updates an existing Gmail label
    pub async fn update_label(
        client: &mut GmailClient,
        label_id: &str,
        updates: HashMap<String, serde_json::Value>,
    ) -> Result<GmailLabel> {
        // Verify the label exists before updating
        match client.get_label(label_id).await {
            Ok(_) => {},
            Err(GmailError::ApiError(msg)) if msg.contains("404") => {
                return Err(GmailError::LabelNotFound(format!("Label with ID \"{}\" not found.", label_id)));
            }
            Err(e) => return Err(e),
        }

        client.update_label(label_id, updates).await
            .map_err(|e| GmailError::ApiError(format!("Failed to update label: {}", e)))
    }

    /// Deletes a Gmail label
    pub async fn delete_label(client: &mut GmailClient, label_id: &str) -> Result<DeleteLabelResult> {
        // Get the label to check if it's a system label and get its name
        let label = match client.get_label(label_id).await {
            Ok(label) => label,
            Err(GmailError::ApiError(msg)) if msg.contains("404") => {
                return Err(GmailError::LabelNotFound(format!("Label with ID \"{}\" not found.", label_id)));
            }
            Err(e) => return Err(e),
        };

        if label.label_type.as_deref() == Some("system") {
            return Err(GmailError::ApiError(format!("Cannot delete system label with ID \"{}\".", label_id)));
        }

        client.delete_label(label_id).await?;

        Ok(DeleteLabelResult {
            success: true,
            message: format!("Label \"{}\" deleted successfully.", label.name),
        })
    }

    /// Gets a detailed list of all Gmail labels
    pub async fn list_labels(client: &mut GmailClient) -> Result<LabelManagerResult> {
        let response = client.list_labels().await
            .map_err(|e| GmailError::ApiError(format!("Failed to list labels: {}", e)))?;

        let labels = response.labels.unwrap_or_default();

        // Group labels by type for better organization
        let system_labels: Vec<GmailLabel> = labels
            .iter()
            .filter(|label| label.label_type.as_deref() == Some("system"))
            .cloned()
            .collect();

        let user_labels: Vec<GmailLabel> = labels
            .iter()
            .filter(|label| label.label_type.as_deref() == Some("user"))
            .cloned()
            .collect();

        Ok(LabelManagerResult {
            count: LabelCount {
                total: labels.len(),
                system: system_labels.len(),
                user: user_labels.len(),
            },
            system: system_labels,
            user: user_labels,
        })
    }

    /// Finds a label by name
    pub async fn find_label_by_name(client: &mut GmailClient, label_name: &str) -> Result<Option<GmailLabel>> {
        let labels_result = Self::list_labels(client).await?;
        
        // Case-insensitive match across both system and user labels
        let mut all_labels = labels_result.system;
        all_labels.extend(labels_result.user);

        let found_label = all_labels
            .into_iter()
            .find(|label| label.name.to_lowercase() == label_name.to_lowercase());

        Ok(found_label)
    }

    /// Creates label if it doesn't exist or returns existing label
    pub async fn get_or_create_label(
        client: &mut GmailClient,
        label_name: &str,
        message_list_visibility: Option<String>,
        label_list_visibility: Option<String>,
    ) -> Result<GmailLabel> {
        // First try to find an existing label
        if let Some(existing_label) = Self::find_label_by_name(client, label_name).await? {
            return Ok(existing_label);
        }

        // If not found, create a new one
        Self::create_label(client, label_name, message_list_visibility, label_list_visibility).await
    }
}