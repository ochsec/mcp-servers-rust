use anyhow::Result;
use futures::future::BoxFuture;
use crate::mcp_core::tools::Tool;
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::{debug, error};

use crate::github::GitHubClient;
use super::toolsets::Toolset;
use super::registry::ToolHandlerFunc;

pub async fn create_context_toolset(github_client: Arc<GitHubClient>) -> Result<Toolset> {
    let mut toolset = Toolset::new("context", "Context tools for getting current user information");

    // get_me tool
    let get_me_tool = Tool {
        name: "get_me".to_string(),
        description: "Get details of the authenticated user".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    };

    let github_client_clone = github_client.clone();
    let get_me_handler: ToolHandlerFunc = Box::new(move |_args: Map<String, Value>| {
        let client = github_client_clone.clone();
        
        Box::pin(async move {
            debug!("Getting authenticated user");
            
            match client.get_authenticated_user().await {
                Ok(user) => {
                    debug!("Successfully retrieved authenticated user: {}", user.login);
                    Ok(serde_json::to_value(user)?)
                }
                Err(e) => {
                    error!("Failed to get authenticated user: {}", e);
                    Err(e)
                }
            }
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("get_me".to_string(), get_me_tool, get_me_handler);

    Ok(toolset)
}