use crate::mcp_core::tools::Tool;
use std::collections::HashMap;

use super::registry::ToolHandlerFunc;

pub struct Toolset {
    pub name: String,
    pub description: String,
    pub tools: Vec<(String, Tool, ToolHandlerFunc)>,
}

impl Toolset {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            tools: Vec::new(),
        }
    }

    pub fn add_tool(&mut self, name: String, tool: Tool, handler: ToolHandlerFunc) {
        self.tools.push((name, tool, handler));
    }

    pub fn get_tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|(name, _, _)| name.clone()).collect()
    }
}

pub struct ToolsetGroup {
    toolsets: HashMap<String, Toolset>,
}

impl ToolsetGroup {
    pub fn new() -> Self {
        Self {
            toolsets: HashMap::new(),
        }
    }

    pub fn add_toolset(&mut self, name: String, toolset: Toolset) {
        self.toolsets.insert(name, toolset);
    }

    pub fn get_toolset(&self, name: &str) -> Option<&Toolset> {
        self.toolsets.get(name)
    }

    pub fn has_toolset(&self, name: &str) -> bool {
        self.toolsets.contains_key(name)
    }

    pub fn get_toolset_names(&self) -> Vec<String> {
        self.toolsets.keys().cloned().collect()
    }

    pub fn get_toolset_tools(&self, toolset_name: &str) -> Option<Vec<String>> {
        self.toolsets.get(toolset_name).map(|ts| ts.get_tool_names())
    }
}

// Default toolsets available
pub const DEFAULT_TOOLSETS: &[&str] = &[
    "repos",
    "issues", 
    "pull_requests",
    "users",
    "context",
];

pub const ALL_TOOLSETS: &[&str] = &[
    "repos",
    "issues",
    "pull_requests", 
    "users",
    "code_security",
    "secret_protection",
    "notifications",
    "context",
    "dynamic",
];