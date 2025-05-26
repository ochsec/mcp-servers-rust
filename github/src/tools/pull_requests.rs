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

pub async fn create_pull_requests_toolset(github_client: Arc<GitHubClient>, read_only: bool) -> Result<Toolset> {
    let mut toolset = Toolset::new("pull_requests", "Pull request management tools");

    // Get pull request tool
    add_get_pull_request_tool(&mut toolset, github_client.clone());

    // List pull requests tool
    add_list_pull_requests_tool(&mut toolset, github_client.clone());

    if !read_only {
        // Create pull request tool
        add_create_pull_request_tool(&mut toolset, github_client.clone());
    }

    Ok(toolset)
}

fn add_get_pull_request_tool(toolset: &mut Toolset, github_client: Arc<GitHubClient>) {
    let tool = Tool {
        name: "get_pull_request".to_string(),
        description: "Get details of a specific pull request".to_string(),
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
                "pull_number": {
                    "type": "number",
                    "description": "Pull request number"
                }
            },
            "required": ["owner", "repo", "pull_number"]
        }),
    };

    let handler: ToolHandlerFunc = Box::new(move |args: Map<String, Value>| {
        let client = github_client.clone();
        
        Box::pin(async move {
            let owner: String = required_param(&args, "owner")?;
            let repo: String = required_param(&args, "repo")?;
            let pull_number: u64 = required_param::<f64>(&args, "pull_number")? as u64;

            debug!("Getting pull request #{} for {}/{}", pull_number, owner, repo);

            match client.get_pull_request(&owner, &repo, pull_number).await {
                Ok(pr) => {
                    debug!("Successfully retrieved pull request");
                    Ok(serde_json::to_value(pr)?)
                }
                Err(e) => {
                    error!("Failed to get pull request: {}", e);
                    Err(e)
                }
            }
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("get_pull_request".to_string(), tool, handler);
}

fn add_list_pull_requests_tool(toolset: &mut Toolset, github_client: Arc<GitHubClient>) {
    let tool = Tool {
        name: "list_pull_requests".to_string(),
        description: "List and filter repository pull requests".to_string(),
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
                "state": {
                    "type": "string",
                    "description": "Pull request state",
                    "enum": ["open", "closed", "all"]
                },
                "head": {
                    "type": "string",
                    "description": "Filter by head branch"
                },
                "base": {
                    "type": "string",
                    "description": "Filter by base branch"
                },
                "sort": {
                    "type": "string",
                    "description": "Sort field",
                    "enum": ["created", "updated", "popularity", "long-running"]
                },
                "direction": {
                    "type": "string",
                    "description": "Sort direction",
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
            "required": ["owner", "repo"]
        }),
    };

    let handler: ToolHandlerFunc = Box::new(move |args: Map<String, Value>| {
        let client = github_client.clone();
        
        Box::pin(async move {
            let owner: String = required_param(&args, "owner")?;
            let repo: String = required_param(&args, "repo")?;
            let state: Option<String> = optional_param(&args, "state")?;
            let head: Option<String> = optional_param(&args, "head")?;
            let base: Option<String> = optional_param(&args, "base")?;
            let sort: Option<String> = optional_param(&args, "sort")?;
            let direction: Option<String> = optional_param(&args, "direction")?;
            let pagination = extract_pagination_params(&args)?;

            debug!("Listing pull requests for {}/{}", owner, repo);

            match client.list_pull_requests(
                &owner,
                &repo,
                state.as_deref(),
                head.as_deref(),
                base.as_deref(),
                sort.as_deref(),
                direction.as_deref(),
                Some(pagination.per_page as u8),
                Some(pagination.page),
            ).await {
                Ok(prs) => {
                    debug!("Successfully retrieved {} pull requests", prs.len());
                    Ok(serde_json::to_value(prs)?)
                }
                Err(e) => {
                    error!("Failed to list pull requests: {}", e);
                    Err(e)
                }
            }
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("list_pull_requests".to_string(), tool, handler);
}

fn add_create_pull_request_tool(toolset: &mut Toolset, github_client: Arc<GitHubClient>) {
    let tool = Tool {
        name: "create_pull_request".to_string(),
        description: "Create a new pull request".to_string(),
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
                "title": {
                    "type": "string",
                    "description": "Pull request title"
                },
                "head": {
                    "type": "string",
                    "description": "The name of the branch where your changes are implemented"
                },
                "base": {
                    "type": "string",
                    "description": "The name of the branch you want the changes pulled into"
                },
                "body": {
                    "type": "string",
                    "description": "Pull request body"
                },
                "draft": {
                    "type": "boolean",
                    "description": "Create as draft pull request"
                }
            },
            "required": ["owner", "repo", "title", "head", "base"]
        }),
    };

    let handler: ToolHandlerFunc = Box::new(move |args: Map<String, Value>| {
        let client = github_client.clone();
        
        Box::pin(async move {
            let owner: String = required_param(&args, "owner")?;
            let repo: String = required_param(&args, "repo")?;
            let title: String = required_param(&args, "title")?;
            let head: String = required_param(&args, "head")?;
            let base: String = required_param(&args, "base")?;
            let body: Option<String> = optional_param(&args, "body")?;
            let draft: Option<bool> = optional_param(&args, "draft")?;

            debug!("Creating pull request for {}/{}: {} -> {}", owner, repo, head, base);

            match client.create_pull_request(
                &owner,
                &repo,
                &title,
                &head,
                &base,
                body.as_deref(),
                draft,
            ).await {
                Ok(pr) => {
                    debug!("Successfully created pull request #{}", pr.number);
                    Ok(serde_json::to_value(pr)?)
                }
                Err(e) => {
                    error!("Failed to create pull request: {}", e);
                    Err(e)
                }
            }
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("create_pull_request".to_string(), tool, handler);
}