use anyhow::Result;
use futures::future::BoxFuture;
use crate::mcp_core::tools::Tool;
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::{debug, error};

use crate::github::GitHubClient;
use crate::server::{required_param, optional_param, extract_pagination_params};
use super::toolsets::Toolset;
use super::registry::ToolHandlerFunc;

pub async fn create_repos_toolset(github_client: Arc<GitHubClient>, read_only: bool) -> Result<Toolset> {
    let mut toolset = Toolset::new("repos", "Repository management tools");

    // Search repositories tool
    add_search_repositories_tool(&mut toolset, github_client.clone());

    // Get file contents tool
    add_get_file_contents_tool(&mut toolset, github_client.clone());

    // Get repository tool
    add_get_repository_tool(&mut toolset, github_client.clone());

    if !read_only {
        // Create or update file tool
        add_create_or_update_file_tool(&mut toolset, github_client.clone());
    }

    Ok(toolset)
}

fn add_search_repositories_tool(toolset: &mut Toolset, github_client: Arc<GitHubClient>) {
    let tool = Tool {
        name: "search_repositories".to_string(),
        description: "Search for GitHub repositories".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "sort": {
                    "type": "string",
                    "description": "Sort field (stars, forks, help-wanted-issues, updated)",
                    "enum": ["stars", "forks", "help-wanted-issues", "updated"]
                },
                "order": {
                    "type": "string", 
                    "description": "Sort order (asc, desc)",
                    "enum": ["asc", "desc"]
                },
                "page": {
                    "type": "number",
                    "description": "Page number for pagination (min 1)"
                },
                "perPage": {
                    "type": "number",
                    "description": "Results per page for pagination (min 1, max 100)"
                }
            },
            "required": ["query"]
        }),
    };

    let handler: ToolHandlerFunc = Box::new(move |args: Map<String, Value>| {
        let client = github_client.clone();
        
        Box::pin(async move {
            let query: String = required_param(&args, "query")?;
            let sort: Option<String> = optional_param(&args, "sort")?;
            let order: Option<String> = optional_param(&args, "order")?;
            let pagination = extract_pagination_params(&args)?;

            debug!("Searching repositories with query: {}", query);

            match client.search_repositories(
                &query,
                sort.as_deref(),
                order.as_deref(),
                Some(pagination.per_page as u8),
                Some(pagination.page),
            ).await {
                Ok(results) => {
                    debug!("Found {} repositories", results.items.len());
                    Ok(serde_json::to_value(results)?)
                }
                Err(e) => {
                    error!("Failed to search repositories: {}", e);
                    Err(e)
                }
            }
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("search_repositories".to_string(), tool, handler);
}

fn add_get_file_contents_tool(toolset: &mut Toolset, github_client: Arc<GitHubClient>) {
    let tool = Tool {
        name: "get_file_contents".to_string(),
        description: "Get contents of a file or directory".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {
                    "type": "string",
                    "description": "Repository owner"
                },
                "repo": {
                    "type": "string", 
                    "description": "Repository name"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory path"
                },
                "ref": {
                    "type": "string",
                    "description": "Git reference (branch, tag, or commit SHA)"
                }
            },
            "required": ["owner", "repo", "path"]
        }),
    };

    let handler: ToolHandlerFunc = Box::new(move |args: Map<String, Value>| {
        let client = github_client.clone();
        
        Box::pin(async move {
            let owner: String = required_param(&args, "owner")?;
            let repo: String = required_param(&args, "repo")?;
            let path: String = required_param(&args, "path")?;
            let reference: Option<String> = optional_param(&args, "ref")?;

            debug!("Getting file contents for {}/{} path: {}", owner, repo, path);

            match client.get_file_contents(&owner, &repo, &path, reference.as_deref()).await {
                Ok(contents) => {
                    debug!("Successfully retrieved file contents");
                    Ok(serde_json::to_value(contents)?)
                }
                Err(e) => {
                    error!("Failed to get file contents: {}", e);
                    Err(e)
                }
            }
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("get_file_contents".to_string(), tool, handler);
}

fn add_get_repository_tool(toolset: &mut Toolset, github_client: Arc<GitHubClient>) {
    let tool = Tool {
        name: "get_repository".to_string(),
        description: "Get repository information".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {
                    "type": "string",
                    "description": "Repository owner"
                },
                "repo": {
                    "type": "string",
                    "description": "Repository name"
                }
            },
            "required": ["owner", "repo"]
        }),
    };

    let handler: ToolHandlerFunc = Box::new(move |args: Map<String, Value>| {
        let client = github_client.clone();
        
        Box::pin(async move {
            let owner: String = required_param(&args, "owner")?;
            let repo: String = required_param(&args, "repo")?;

            debug!("Getting repository {}/{}", owner, repo);

            match client.get_repository(&owner, &repo).await {
                Ok(repository) => {
                    debug!("Successfully retrieved repository");
                    Ok(serde_json::to_value(repository)?)
                }
                Err(e) => {
                    error!("Failed to get repository: {}", e);
                    Err(e)
                }
            }
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("get_repository".to_string(), tool, handler);
}

fn add_create_or_update_file_tool(toolset: &mut Toolset, github_client: Arc<GitHubClient>) {
    let tool = Tool {
        name: "create_or_update_file".to_string(),
        description: "Create or update a single file".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {
                    "type": "string",
                    "description": "Repository owner"
                },
                "repo": {
                    "type": "string",
                    "description": "Repository name"
                },
                "path": {
                    "type": "string",
                    "description": "File path"
                },
                "content": {
                    "type": "string",
                    "description": "File content"
                },
                "message": {
                    "type": "string",
                    "description": "Commit message"
                },
                "sha": {
                    "type": "string",
                    "description": "SHA of the file being replaced (required for updates)"
                },
                "branch": {
                    "type": "string",
                    "description": "Branch name"
                }
            },
            "required": ["owner", "repo", "path", "content", "message"]
        }),
    };

    let handler: ToolHandlerFunc = Box::new(move |args: Map<String, Value>| {
        let client = github_client.clone();
        
        Box::pin(async move {
            let owner: String = required_param(&args, "owner")?;
            let repo: String = required_param(&args, "repo")?;
            let path: String = required_param(&args, "path")?;
            let content: String = required_param(&args, "content")?;
            let message: String = required_param(&args, "message")?;
            let sha: Option<String> = optional_param(&args, "sha")?;
            let branch: Option<String> = optional_param(&args, "branch")?;

            debug!("Creating/updating file {}/{} path: {}", owner, repo, path);

            match client.create_or_update_file(
                &owner,
                &repo,
                &path,
                &content,
                &message,
                sha.as_deref(),
                branch.as_deref(),
            ).await {
                Ok(commit) => {
                    debug!("Successfully created/updated file");
                    Ok(serde_json::to_value(commit)?)
                }
                Err(e) => {
                    error!("Failed to create/update file: {}", e);
                    Err(e)
                }
            }
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("create_or_update_file".to_string(), tool, handler);
}