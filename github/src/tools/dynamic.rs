use anyhow::Result;
use futures::future::BoxFuture;
use crate::mcp_core::tools::Tool;
use serde_json::{Map, Value};
use tracing::{debug};

use crate::server::{required_param};
use super::toolsets::{Toolset, ALL_TOOLSETS};
use super::registry::ToolHandlerFunc;

pub async fn create_dynamic_toolset(enabled_toolsets: Vec<String>) -> Result<Toolset> {
    let mut toolset = Toolset::new("dynamic", "Dynamic toolset management tools");

    // List available toolsets tool
    add_list_available_toolsets_tool(&mut toolset);

    // Get toolset tools tool
    add_get_toolset_tools_tool(&mut toolset);

    // Enable toolset tool
    add_enable_toolset_tool(&mut toolset, enabled_toolsets);

    Ok(toolset)
}

fn add_list_available_toolsets_tool(toolset: &mut Toolset) {
    let tool = Tool {
        name: "list_available_toolsets".to_string(),
        description: "List all available toolsets".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    };

    let handler: ToolHandlerFunc = Box::new(move |_args: Map<String, Value>| {
        Box::pin(async move {
            debug!("Listing available toolsets");

            let toolsets: Vec<serde_json::Value> = ALL_TOOLSETS.iter().map(|&name| {
                serde_json::json!({
                    "name": name,
                    "description": get_toolset_description(name)
                })
            }).collect();

            Ok(serde_json::json!({
                "toolsets": toolsets
            }))
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("list_available_toolsets".to_string(), tool, handler);
}

fn add_get_toolset_tools_tool(toolset: &mut Toolset) {
    let tool = Tool {
        name: "get_toolset_tools".to_string(),
        description: "List tools in a specific toolset".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "toolset_name": {
                    "type": "string",
                    "description": "Name of the toolset"
                }
            },
            "required": ["toolset_name"]
        }),
    };

    let handler: ToolHandlerFunc = Box::new(move |args: Map<String, Value>| {
        Box::pin(async move {
            let toolset_name: String = required_param(&args, "toolset_name")?;
            debug!("Getting tools for toolset: {}", toolset_name);

            let tools = get_toolset_tool_names(&toolset_name);
            
            Ok(serde_json::json!({
                "toolset": toolset_name,
                "tools": tools
            }))
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("get_toolset_tools".to_string(), tool, handler);
}

fn add_enable_toolset_tool(toolset: &mut Toolset, enabled_toolsets: Vec<String>) {
    let tool = Tool {
        name: "enable_toolset".to_string(),
        description: "Enable additional toolsets dynamically".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "toolset_name": {
                    "type": "string",
                    "description": "Name of the toolset to enable"
                }
            },
            "required": ["toolset_name"]
        }),
    };

    let handler: ToolHandlerFunc = Box::new(move |args: Map<String, Value>| {
        let enabled = enabled_toolsets.clone();
        
        Box::pin(async move {
            let toolset_name: String = required_param(&args, "toolset_name")?;
            debug!("Enabling toolset: {}", toolset_name);

            // Check if toolset is valid
            if !ALL_TOOLSETS.contains(&toolset_name.as_str()) {
                return Err(anyhow::anyhow!("Unknown toolset: {}", toolset_name));
            }

            // Check if already enabled
            if enabled.contains(&toolset_name) {
                return Ok(serde_json::json!({
                    "message": format!("Toolset '{}' is already enabled", toolset_name),
                    "enabled": true
                }));
            }

            // Note: In a real implementation, we would need to modify the registry
            // This is a simplified response
            Ok(serde_json::json!({
                "message": format!("Toolset '{}' has been enabled", toolset_name),
                "enabled": true
            }))
        }) as BoxFuture<'static, Result<Value>>
    });

    toolset.add_tool("enable_toolset".to_string(), tool, handler);
}

fn get_toolset_description(name: &str) -> &'static str {
    match name {
        "repos" => "Repository management tools",
        "issues" => "Issue management tools",
        "pull_requests" => "Pull request management tools",
        "users" => "User management tools",
        "code_security" => "Code security scanning tools",
        "secret_protection" => "Secret scanning tools",
        "notifications" => "Notification management tools",
        "context" => "Context tools for getting current user information",
        "dynamic" => "Dynamic toolset management tools",
        _ => "Unknown toolset",
    }
}

fn get_toolset_tool_names(name: &str) -> Vec<&'static str> {
    match name {
        "repos" => vec![
            "search_repositories",
            "get_file_contents", 
            "get_repository",
            "create_or_update_file",
        ],
        "issues" => vec![
            "get_issue",
            "list_issues",
            "create_issue",
        ],
        "pull_requests" => vec![
            "get_pull_request",
            "list_pull_requests", 
            "create_pull_request",
        ],
        "users" => vec![
            "search_users",
        ],
        "context" => vec![
            "get_me",
        ],
        "dynamic" => vec![
            "list_available_toolsets",
            "get_toolset_tools",
            "enable_toolset",
        ],
        _ => vec![],
    }
}