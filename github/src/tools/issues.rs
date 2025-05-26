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

pub async fn create_issues_toolset(github_client: Arc<GitHubClient>, read_only: bool) -> Result<Toolset> {
    let mut toolset = Toolset::new("issues", "Issue management tools");

    // Get issue tool
    add_get_issue_tool(&mut toolset, github_client.clone());

    // List issues tool
    add_list_issues_tool(&mut toolset, github_client.clone());

    if !read_only {
        // Create issue tool
        add_create_issue_tool(&mut toolset, github_client.clone());
    }

    Ok(toolset)
}

fn add_get_issue_tool(toolset: &mut Toolset, github_client: Arc<GitHubClient>) {
    let tool = Tool {
        name: "get_issue".to_string(),
        description: "Get details of a specific issue".to_string(),
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
                "issue_number": {
                    "type": "number",
                    "description": "Issue number"
                }
            },
            "required": ["owner", "repo", "issue_number"]
        }),
    };

    let handler: ToolHandlerFunc = Box::new(move |args: Map<String, Value>| {
        let client = github_client.clone();
        
        Box::pin(async move {
            let owner: String = required_param(&args, "owner")?;
            let repo: String = required_param(&args, "repo")?;
            let issue_number: u64 = required_param::<f64>(&args, "issue_number")? as u64;

            debug!("Getting issue #{} for {}/{}", issue_number, owner, repo);

            match client.get_issue(&owner, &repo, issue_number).await {
                Ok(issue) => {
                    debug!("Successfully retrieved issue");
                    Ok(serde_json::to_value(issue)?)
                }
                Err(e) => {
                    error!("Failed to get issue: {}", e);
                    Err(e)
                }
            }
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("get_issue".to_string(), tool, handler);
}

fn add_list_issues_tool(toolset: &mut Toolset, github_client: Arc<GitHubClient>) {
    let tool = Tool {
        name: "list_issues".to_string(),
        description: "List and filter repository issues".to_string(),
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
                    "description": "Issue state",
                    "enum": ["open", "closed", "all"]
                },
                "labels": {
                    "type": "array",
                    "description": "Filter by labels",
                    "items": {"type": "string"}
                },
                "assignee": {
                    "type": "string",
                    "description": "Filter by assignee"
                },
                "creator": {
                    "type": "string",
                    "description": "Filter by creator"
                },
                "sort": {
                    "type": "string",
                    "description": "Sort field",
                    "enum": ["created", "updated", "comments"]
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
            let labels: Option<Vec<String>> = optional_param(&args, "labels")?;
            let assignee: Option<String> = optional_param(&args, "assignee")?;
            let creator: Option<String> = optional_param(&args, "creator")?;
            let sort: Option<String> = optional_param(&args, "sort")?;
            let direction: Option<String> = optional_param(&args, "direction")?;
            let pagination = extract_pagination_params(&args)?;

            debug!("Listing issues for {}/{}", owner, repo);

            match client.list_issues(
                &owner,
                &repo,
                state.as_deref(),
                labels,
                assignee.as_deref(),
                creator.as_deref(),
                None, // mentioned
                None, // milestone
                sort.as_deref(),
                direction.as_deref(),
                None, // since
                Some(pagination.per_page as u8),
                Some(pagination.page),
            ).await {
                Ok(issues) => {
                    debug!("Successfully retrieved {} issues", issues.len());
                    Ok(serde_json::to_value(issues)?)
                }
                Err(e) => {
                    error!("Failed to list issues: {}", e);
                    Err(e)
                }
            }
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("list_issues".to_string(), tool, handler);
}

fn add_create_issue_tool(toolset: &mut Toolset, github_client: Arc<GitHubClient>) {
    let tool = Tool {
        name: "create_issue".to_string(),
        description: "Create a new issue".to_string(),
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
                    "description": "Issue title"
                },
                "body": {
                    "type": "string",
                    "description": "Issue body"
                },
                "assignees": {
                    "type": "array",
                    "description": "Usernames to assign",
                    "items": {"type": "string"}
                },
                "milestone": {
                    "type": "number",
                    "description": "Milestone number"
                },
                "labels": {
                    "type": "array",
                    "description": "Labels to add",
                    "items": {"type": "string"}
                }
            },
            "required": ["owner", "repo", "title"]
        }),
    };

    let handler: ToolHandlerFunc = Box::new(move |args: Map<String, Value>| {
        let client = github_client.clone();
        
        Box::pin(async move {
            let owner: String = required_param(&args, "owner")?;
            let repo: String = required_param(&args, "repo")?;
            let title: String = required_param(&args, "title")?;
            let body: Option<String> = optional_param(&args, "body")?;
            let assignees: Option<Vec<String>> = optional_param(&args, "assignees")?;
            let milestone: Option<u64> = optional_param::<f64>(&args, "milestone")?.map(|m| m as u64);
            let labels: Option<Vec<String>> = optional_param(&args, "labels")?;

            debug!("Creating issue for {}/{}: {}", owner, repo, title);

            match client.create_issue(
                &owner,
                &repo,
                &title,
                body.as_deref(),
                assignees,
                milestone,
                labels,
            ).await {
                Ok(issue) => {
                    debug!("Successfully created issue #{}", issue.number);
                    Ok(serde_json::to_value(issue)?)
                }
                Err(e) => {
                    error!("Failed to create issue: {}", e);
                    Err(e)
                }
            }
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("create_issue".to_string(), tool, handler);
}