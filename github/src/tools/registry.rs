use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::future::BoxFuture;
use crate::mcp_core::tools::{Tool, ToolHandler};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::github::GitHubClient;
use super::toolsets::*;

pub type ToolHandlerFunc = Box<dyn Fn(Map<String, Value>) -> BoxFuture<'static, Result<Value>> + Send + Sync>;

pub struct ToolRegistry {
    tools: HashMap<String, Tool>,
    handlers: HashMap<String, ToolHandlerFunc>,
    toolsets: ToolsetGroup,
    enabled_toolsets: Vec<String>,
    read_only: bool,
    dynamic_toolsets: bool,
    github_client: Arc<GitHubClient>,
}

impl ToolRegistry {
    pub fn new(
        enabled_toolsets: Vec<String>,
        read_only: bool,
        dynamic_toolsets: bool,
        github_client: Arc<GitHubClient>,
    ) -> Self {
        Self {
            tools: HashMap::new(),
            handlers: HashMap::new(),
            toolsets: ToolsetGroup::new(),
            enabled_toolsets,
            read_only,
            dynamic_toolsets,
            github_client,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing tool registry with toolsets: {:?}", self.enabled_toolsets);

        // Initialize context tools (always enabled)
        let context_toolset = super::context::create_context_toolset(
            self.github_client.clone(),
        ).await?;
        self.register_toolset("context", context_toolset);

        // Initialize dynamic toolset if enabled
        if self.dynamic_toolsets {
            let dynamic_toolset = super::dynamic::create_dynamic_toolset(
                self.enabled_toolsets.clone(),
            ).await?;
            self.register_toolset("dynamic", dynamic_toolset);
        }

        // Initialize other toolsets based on configuration
        let enabled_toolsets = self.enabled_toolsets.clone();
        for toolset_name in &enabled_toolsets {
            match toolset_name.as_str() {
                "all" => {
                    // Enable all toolsets
                    self.enable_all_toolsets().await?;
                    break;
                }
                "repos" => {
                    let repos_toolset = super::repos::create_repos_toolset(
                        self.github_client.clone(),
                        self.read_only,
                    ).await?;
                    self.register_toolset("repos", repos_toolset);
                }
                "issues" => {
                    let issues_toolset = super::issues::create_issues_toolset(
                        self.github_client.clone(),
                        self.read_only,
                    ).await?;
                    self.register_toolset("issues", issues_toolset);
                }
                "pull_requests" => {
                    let pr_toolset = super::pull_requests::create_pull_requests_toolset(
                        self.github_client.clone(),
                        self.read_only,
                    ).await?;
                    self.register_toolset("pull_requests", pr_toolset);
                }
                "users" => {
                    let users_toolset = super::users::create_users_toolset(
                        self.github_client.clone(),
                    ).await?;
                    self.register_toolset("users", users_toolset);
                }
                _ => {
                    debug!("Unknown toolset: {}", toolset_name);
                }
            }
        }

        info!("Tool registry initialized with {} tools", self.tools.len());
        Ok(())
    }

    async fn enable_all_toolsets(&mut self) -> Result<()> {
        // Repository tools
        let repos_toolset = super::repos::create_repos_toolset(
            self.github_client.clone(),
            self.read_only,
        ).await?;
        self.register_toolset("repos", repos_toolset);

        // Issues tools
        let issues_toolset = super::issues::create_issues_toolset(
            self.github_client.clone(),
            self.read_only,
        ).await?;
        self.register_toolset("issues", issues_toolset);

        // Pull requests tools
        let pr_toolset = super::pull_requests::create_pull_requests_toolset(
            self.github_client.clone(),
            self.read_only,
        ).await?;
        self.register_toolset("pull_requests", pr_toolset);

        // Users tools
        let users_toolset = super::users::create_users_toolset(
            self.github_client.clone(),
        ).await?;
        self.register_toolset("users", users_toolset);

        Ok(())
    }

    fn register_toolset(&mut self, name: &str, mut toolset: Toolset) {
        debug!("Registering toolset: {}", name);
        
        for (tool_name, tool, handler) in toolset.tools.drain(..) {
            if self.tools.contains_key(&tool_name) {
                debug!("Tool {} already registered, skipping", tool_name);
                continue;
            }

            debug!("Registering tool: {}", tool_name);
            self.tools.insert(tool_name.clone(), tool);
            self.handlers.insert(tool_name, handler);
        }

        self.toolsets.add_toolset(name.to_string(), toolset);
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        Ok(self.tools.values().cloned().collect())
    }

    pub async fn call_tool(&self, name: &str, arguments: Map<String, Value>) -> Result<Value> {
        debug!("Calling tool: {} with args: {:?}", name, arguments);

        let handler = self.handlers.get(name)
            .ok_or_else(|| anyhow!("Tool not found: {}", name))?;

        match handler(arguments).await {
            Ok(result) => {
                debug!("Tool {} executed successfully", name);
                Ok(result)
            }
            Err(e) => {
                error!("Tool {} execution failed: {}", name, e);
                Err(e)
            }
        }
    }

    // Note: handlers cannot be cloned, so we manage them internally

    pub fn get_toolset_names(&self) -> Vec<String> {
        self.toolsets.get_toolset_names()
    }

    pub fn get_toolset_tools(&self, toolset_name: &str) -> Option<Vec<String>> {
        self.toolsets.get_toolset_tools(toolset_name)
    }

    pub async fn enable_toolset(&mut self, toolset_name: &str) -> Result<()> {
        if self.toolsets.has_toolset(toolset_name) {
            return Ok(()); // Already enabled
        }

        match toolset_name {
            "repos" => {
                let repos_toolset = super::repos::create_repos_toolset(
                    self.github_client.clone(),
                    self.read_only,
                ).await?;
                self.register_toolset("repos", repos_toolset);
            }
            "issues" => {
                let issues_toolset = super::issues::create_issues_toolset(
                    self.github_client.clone(),
                    self.read_only,
                ).await?;
                self.register_toolset("issues", issues_toolset);
            }
            "pull_requests" => {
                let pr_toolset = super::pull_requests::create_pull_requests_toolset(
                    self.github_client.clone(),
                    self.read_only,
                ).await?;
                self.register_toolset("pull_requests", pr_toolset);
            }
            "users" => {
                let users_toolset = super::users::create_users_toolset(
                    self.github_client.clone(),
                ).await?;
                self.register_toolset("users", users_toolset);
            }
            _ => {
                return Err(anyhow!("Unknown toolset: {}", toolset_name));
            }
        }

        Ok(())
    }
}