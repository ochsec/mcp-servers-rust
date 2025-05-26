use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::future::BoxFuture;
use crate::mcp_core::resources::Resource;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};
use url::Url;

use crate::github::GitHubClient;

pub type ResourceHandlerFunc = Box<dyn Fn(String) -> BoxFuture<'static, Result<Value>> + Send + Sync>;

pub struct ResourceRegistry {
    handlers: HashMap<String, ResourceHandlerFunc>,
    github_client: Arc<GitHubClient>,
}

impl ResourceRegistry {
    pub fn new(github_client: Arc<GitHubClient>) -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
            github_client: github_client.clone(),
        };

        registry.register_handlers();
        registry
    }

    fn register_handlers(&mut self) {
        info!("Registering resource handlers");

        // Repository content handler
        let github_client = self.github_client.clone();
        let repo_handler: ResourceHandlerFunc = Box::new(move |uri: String| {
            let client = github_client.clone();
            
            Box::pin(async move {
                debug!("Handling repository resource: {}", uri);
                handle_repository_resource(&client, &uri).await
            })
        });

        self.handlers.insert("repo://{owner}/{repo}/contents{/path*}".to_string(), repo_handler);

        // Branch-specific content handler
        let github_client = self.github_client.clone();
        let branch_handler: ResourceHandlerFunc = Box::new(move |uri: String| {
            let client = github_client.clone();
            
            Box::pin(async move {
                debug!("Handling branch resource: {}", uri);
                handle_branch_resource(&client, &uri).await
            })
        });

        self.handlers.insert("repo://{owner}/{repo}/refs/heads/{branch}/contents{/path*}".to_string(), branch_handler);

        // Commit-specific content handler
        let github_client = self.github_client.clone();
        let commit_handler: ResourceHandlerFunc = Box::new(move |uri: String| {
            let client = github_client.clone();
            
            Box::pin(async move {
                debug!("Handling commit resource: {}", uri);
                handle_commit_resource(&client, &uri).await
            })
        });

        self.handlers.insert("repo://{owner}/{repo}/sha/{sha}/contents{/path*}".to_string(), commit_handler);

        // Tag-specific content handler
        let github_client = self.github_client.clone();
        let tag_handler: ResourceHandlerFunc = Box::new(move |uri: String| {
            let client = github_client.clone();
            
            Box::pin(async move {
                debug!("Handling tag resource: {}", uri);
                handle_tag_resource(&client, &uri).await
            })
        });

        self.handlers.insert("repo://{owner}/{repo}/refs/tags/{tag}/contents{/path*}".to_string(), tag_handler);

        // Pull request content handler
        let github_client = self.github_client.clone();
        let pr_handler: ResourceHandlerFunc = Box::new(move |uri: String| {
            let client = github_client.clone();
            
            Box::pin(async move {
                debug!("Handling pull request resource: {}", uri);
                handle_pull_request_resource(&client, &uri).await
            })
        });

        self.handlers.insert("repo://{owner}/{repo}/refs/pull/{prNumber}/head/contents{/path*}".to_string(), pr_handler);

        info!("Registered {} resource handlers", self.handlers.len());
    }

    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        let resources = vec![
            Resource {
                uri: "repo://{owner}/{repo}/contents{/path*}".to_string(),
                name: "Repository content".to_string(),
                description: Some("Repository content and file structure".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            Resource {
                uri: "repo://{owner}/{repo}/refs/heads/{branch}/contents{/path*}".to_string(),
                name: "Branch content".to_string(),
                description: Some("Content from a specific branch".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            Resource {
                uri: "repo://{owner}/{repo}/sha/{sha}/contents{/path*}".to_string(),
                name: "Commit content".to_string(),
                description: Some("Content from a specific commit".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            Resource {
                uri: "repo://{owner}/{repo}/refs/tags/{tag}/contents{/path*}".to_string(),
                name: "Tag content".to_string(),
                description: Some("Content from a specific tag".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            Resource {
                uri: "repo://{owner}/{repo}/refs/pull/{prNumber}/head/contents{/path*}".to_string(),
                name: "Pull request content".to_string(),
                description: Some("Content from a pull request head".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
        ];

        Ok(resources)
    }

    pub async fn read_resource(&self, uri: &str) -> Result<Value> {
        debug!("Reading resource: {}", uri);

        // Find matching handler by checking URI patterns
        for (pattern, handler) in &self.handlers {
            if uri_matches_pattern(uri, pattern) {
                return handler(uri.to_string()).await;
            }
        }

        Err(anyhow!("No handler found for resource URI: {}", uri))
    }

    // Note: handlers cannot be cloned, so we handle resources directly
}

async fn handle_repository_resource(client: &GitHubClient, uri: &str) -> Result<Value> {
    let parts = parse_repo_uri(uri)?;
    let owner = parts.get("owner").ok_or_else(|| anyhow!("Missing owner"))?;
    let repo = parts.get("repo").ok_or_else(|| anyhow!("Missing repo"))?;
    let path = parts.get("path").map(|s| s.as_str()).unwrap_or("");

    match client.get_file_contents(owner, repo, path, None).await {
        Ok(content) => {
            if content.r#type == "file" {
                let text = if let Some(encoded_content) = &content.content {
                    match content.encoding.as_deref() {
                        Some("base64") => {
                            match base64::decode(encoded_content.replace('\n', "")) {
                                Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                                Err(_) => format!("Binary file: {}", content.name),
                            }
                        }
                        _ => encoded_content.clone(),
                    }
                } else {
                    format!("File: {}", content.name)
                };

                Ok(serde_json::json!({
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": text
                        }
                    ]
                }))
            } else {
                Ok(serde_json::json!({
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": format!("Directory: {}", content.name)
                        }
                    ]
                }))
            }
        }
        Err(e) => {
            error!("Failed to get file contents: {}", e);
            Err(e)
        }
    }
}

async fn handle_branch_resource(client: &GitHubClient, uri: &str) -> Result<Value> {
    let parts = parse_branch_uri(uri)?;
    let owner = parts.get("owner").ok_or_else(|| anyhow!("Missing owner"))?;
    let repo = parts.get("repo").ok_or_else(|| anyhow!("Missing repo"))?;
    let branch = parts.get("branch").ok_or_else(|| anyhow!("Missing branch"))?;
    let path = parts.get("path").map(|s| s.as_str()).unwrap_or("");

    match client.get_file_contents(owner, repo, path, Some(branch)).await {
        Ok(content) => {
            if content.r#type == "file" {
                let text = if let Some(encoded_content) = &content.content {
                    match content.encoding.as_deref() {
                        Some("base64") => {
                            match base64::decode(encoded_content.replace('\n', "")) {
                                Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                                Err(_) => format!("Binary file: {}", content.name),
                            }
                        }
                        _ => encoded_content.clone(),
                    }
                } else {
                    format!("File: {}", content.name)
                };

                Ok(serde_json::json!({
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": text
                        }
                    ]
                }))
            } else {
                Ok(serde_json::json!({
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": format!("Directory: {}", content.name)
                        }
                    ]
                }))
            }
        }
        Err(e) => {
            error!("Failed to get file contents: {}", e);
            Err(e)
        }
    }
}

async fn handle_commit_resource(client: &GitHubClient, uri: &str) -> Result<Value> {
    let parts = parse_commit_uri(uri)?;
    let owner = parts.get("owner").ok_or_else(|| anyhow!("Missing owner"))?;
    let repo = parts.get("repo").ok_or_else(|| anyhow!("Missing repo"))?;
    let sha = parts.get("sha").ok_or_else(|| anyhow!("Missing sha"))?;
    let path = parts.get("path").map(|s| s.as_str()).unwrap_or("");

    match client.get_file_contents(owner, repo, path, Some(sha)).await {
        Ok(content) => {
            if content.r#type == "file" {
                let text = if let Some(encoded_content) = &content.content {
                    match content.encoding.as_deref() {
                        Some("base64") => {
                            match base64::decode(encoded_content.replace('\n', "")) {
                                Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                                Err(_) => format!("Binary file: {}", content.name),
                            }
                        }
                        _ => encoded_content.clone(),
                    }
                } else {
                    format!("File: {}", content.name)
                };

                Ok(serde_json::json!({
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": text
                        }
                    ]
                }))
            } else {
                Ok(serde_json::json!({
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": format!("Directory: {}", content.name)
                        }
                    ]
                }))
            }
        }
        Err(e) => {
            error!("Failed to get file contents: {}", e);
            Err(e)
        }
    }
}

async fn handle_tag_resource(client: &GitHubClient, uri: &str) -> Result<Value> {
    let parts = parse_tag_uri(uri)?;
    let owner = parts.get("owner").ok_or_else(|| anyhow!("Missing owner"))?;
    let repo = parts.get("repo").ok_or_else(|| anyhow!("Missing repo"))?;
    let tag = parts.get("tag").ok_or_else(|| anyhow!("Missing tag"))?;
    let path = parts.get("path").map(|s| s.as_str()).unwrap_or("");

    match client.get_file_contents(owner, repo, path, Some(tag)).await {
        Ok(content) => {
            if content.r#type == "file" {
                let text = if let Some(encoded_content) = &content.content {
                    match content.encoding.as_deref() {
                        Some("base64") => {
                            match base64::decode(encoded_content.replace('\n', "")) {
                                Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                                Err(_) => format!("Binary file: {}", content.name),
                            }
                        }
                        _ => encoded_content.clone(),
                    }
                } else {
                    format!("File: {}", content.name)
                };

                Ok(serde_json::json!({
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": text
                        }
                    ]
                }))
            } else {
                Ok(serde_json::json!({
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": format!("Directory: {}", content.name)
                        }
                    ]
                }))
            }
        }
        Err(e) => {
            error!("Failed to get file contents: {}", e);
            Err(e)
        }
    }
}

async fn handle_pull_request_resource(client: &GitHubClient, uri: &str) -> Result<Value> {
    let parts = parse_pr_uri(uri)?;
    let owner = parts.get("owner").ok_or_else(|| anyhow!("Missing owner"))?;
    let repo = parts.get("repo").ok_or_else(|| anyhow!("Missing repo"))?;
    let pr_number = parts.get("prNumber").ok_or_else(|| anyhow!("Missing prNumber"))?;
    let path = parts.get("path").map(|s| s.as_str()).unwrap_or("");

    // First get the PR to get the head SHA
    match client.get_pull_request(owner, repo, pr_number.parse()?).await {
        Ok(pr) => {
            let head_sha = pr.head.sha;
            match client.get_file_contents(owner, repo, path, Some(&head_sha)).await {
                Ok(content) => {
                    if content.r#type == "file" {
                        let text = if let Some(encoded_content) = &content.content {
                            match content.encoding.as_deref() {
                                Some("base64") => {
                                    match base64::decode(encoded_content.replace('\n', "")) {
                                        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                                        Err(_) => format!("Binary file: {}", content.name),
                                    }
                                }
                                _ => encoded_content.clone(),
                            }
                        } else {
                            format!("File: {}", content.name)
                        };

                        Ok(serde_json::json!({
                            "contents": [
                                {
                                    "uri": uri,
                                    "mimeType": "text/plain",
                                    "text": text
                                }
                            ]
                        }))
                    } else {
                        Ok(serde_json::json!({
                            "contents": [
                                {
                                    "uri": uri,
                                    "mimeType": "text/plain",
                                    "text": format!("Directory: {}", content.name)
                                }
                            ]
                        }))
                    }
                }
                Err(e) => {
                    error!("Failed to get file contents: {}", e);
                    Err(e)
                }
            }
        }
        Err(e) => {
            error!("Failed to get pull request: {}", e);
            Err(e)
        }
    }
}

fn uri_matches_pattern(uri: &str, pattern: &str) -> bool {
    // Simple pattern matching - in a real implementation you'd want more sophisticated matching
    if pattern.contains("{owner}") && pattern.contains("{repo}") {
        return uri.starts_with("repo://");
    }
    false
}

fn parse_repo_uri(uri: &str) -> Result<HashMap<String, String>> {
    // Parse repo://owner/repo/contents/path
    let mut parts = HashMap::new();
    
    if let Some(stripped) = uri.strip_prefix("repo://") {
        let segments: Vec<&str> = stripped.split('/').collect();
        if segments.len() >= 3 {
            parts.insert("owner".to_string(), segments[0].to_string());
            parts.insert("repo".to_string(), segments[1].to_string());
            
            if segments.len() > 3 && segments[2] == "contents" {
                let path = segments[3..].join("/");
                if !path.is_empty() {
                    parts.insert("path".to_string(), path);
                }
            }
        }
    }
    
    Ok(parts)
}

fn parse_branch_uri(uri: &str) -> Result<HashMap<String, String>> {
    // Parse repo://owner/repo/refs/heads/branch/contents/path
    let mut parts = HashMap::new();
    
    if let Some(stripped) = uri.strip_prefix("repo://") {
        let segments: Vec<&str> = stripped.split('/').collect();
        if segments.len() >= 6 {
            parts.insert("owner".to_string(), segments[0].to_string());
            parts.insert("repo".to_string(), segments[1].to_string());
            parts.insert("branch".to_string(), segments[4].to_string());
            
            if segments.len() > 6 && segments[5] == "contents" {
                let path = segments[6..].join("/");
                if !path.is_empty() {
                    parts.insert("path".to_string(), path);
                }
            }
        }
    }
    
    Ok(parts)
}

fn parse_commit_uri(uri: &str) -> Result<HashMap<String, String>> {
    // Parse repo://owner/repo/sha/commit_sha/contents/path
    let mut parts = HashMap::new();
    
    if let Some(stripped) = uri.strip_prefix("repo://") {
        let segments: Vec<&str> = stripped.split('/').collect();
        if segments.len() >= 5 {
            parts.insert("owner".to_string(), segments[0].to_string());
            parts.insert("repo".to_string(), segments[1].to_string());
            parts.insert("sha".to_string(), segments[3].to_string());
            
            if segments.len() > 5 && segments[4] == "contents" {
                let path = segments[5..].join("/");
                if !path.is_empty() {
                    parts.insert("path".to_string(), path);
                }
            }
        }
    }
    
    Ok(parts)
}

fn parse_tag_uri(uri: &str) -> Result<HashMap<String, String>> {
    // Parse repo://owner/repo/refs/tags/tag/contents/path
    let mut parts = HashMap::new();
    
    if let Some(stripped) = uri.strip_prefix("repo://") {
        let segments: Vec<&str> = stripped.split('/').collect();
        if segments.len() >= 6 {
            parts.insert("owner".to_string(), segments[0].to_string());
            parts.insert("repo".to_string(), segments[1].to_string());
            parts.insert("tag".to_string(), segments[4].to_string());
            
            if segments.len() > 6 && segments[5] == "contents" {
                let path = segments[6..].join("/");
                if !path.is_empty() {
                    parts.insert("path".to_string(), path);
                }
            }
        }
    }
    
    Ok(parts)
}

fn parse_pr_uri(uri: &str) -> Result<HashMap<String, String>> {
    // Parse repo://owner/repo/refs/pull/123/head/contents/path
    let mut parts = HashMap::new();
    
    if let Some(stripped) = uri.strip_prefix("repo://") {
        let segments: Vec<&str> = stripped.split('/').collect();
        if segments.len() >= 7 {
            parts.insert("owner".to_string(), segments[0].to_string());
            parts.insert("repo".to_string(), segments[1].to_string());
            parts.insert("prNumber".to_string(), segments[4].to_string());
            
            if segments.len() > 7 && segments[6] == "contents" {
                let path = segments[7..].join("/");
                if !path.is_empty() {
                    parts.insert("path".to_string(), path);
                }
            }
        }
    }
    
    Ok(parts)
}