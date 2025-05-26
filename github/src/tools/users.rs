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

pub async fn create_users_toolset(github_client: Arc<GitHubClient>) -> Result<Toolset> {
    let mut toolset = Toolset::new("users", "User management tools");

    // Search users tool
    add_search_users_tool(&mut toolset, github_client.clone());

    Ok(toolset)
}

fn add_search_users_tool(toolset: &mut Toolset, github_client: Arc<GitHubClient>) {
    let tool = Tool {
        name: "search_users".to_string(),
        description: "Search for GitHub users".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "sort": {
                    "type": "string",
                    "description": "Sort field (followers, repositories, joined)",
                    "enum": ["followers", "repositories", "joined"]
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

            debug!("Searching users with query: {}", query);

            match client.search_users(
                &query,
                sort.as_deref(),
                order.as_deref(),
                Some(pagination.per_page as u8),
                Some(pagination.page),
            ).await {
                Ok(results) => {
                    debug!("Found {} users", results.items.len());
                    Ok(serde_json::to_value(results)?)
                }
                Err(e) => {
                    error!("Failed to search users: {}", e);
                    Err(e)
                }
            }
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("search_users".to_string(), tool, handler);
}