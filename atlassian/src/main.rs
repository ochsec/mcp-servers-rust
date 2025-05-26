use anyhow::Result;
use clap::Parser;
use rmcp::{tool, ServerHandler, ServiceExt, model::ServerInfo};
use rmcp::transport::stdio;
use serde_json::Value;
use tracing::info;
use tracing_subscriber;

mod atlassian;
mod config;

use atlassian::AtlassianClient;
use config::Config;

#[derive(Parser)]
#[command(name = "atlassian-mcp-server")]
#[command(about = "MCP server for Atlassian JIRA and Confluence integration")]
struct Cli {}

#[derive(Debug, Clone)]
pub struct AtlassianMcpServer {
    client: AtlassianClient,
    config: Config,
}

impl AtlassianMcpServer {
    fn new(config: Config) -> Self {
        let client = AtlassianClient::new(config.atlassian.clone());
        Self { client, config }
    }
}

#[tool(tool_box)]
impl AtlassianMcpServer {
    /// Get details of a JIRA ticket by key
    pub async fn get_jira_ticket(&self, ticket_key: String) -> Result<Value, String> {
        match self.client.get_jira_ticket(&ticket_key).await {
            Ok(ticket) => Ok(ticket),
            Err(e) => Err(format!("Error getting JIRA ticket: {}", e)),
        }
    }

    /// Search for JIRA tickets using JQL
    pub async fn search_jira_tickets(
        &self,
        jql: String,
        max_results: Option<u32>,
    ) -> Result<Value, String> {
        match self.client.search_jira_tickets(&jql, max_results).await {
            Ok(results) => Ok(results),
            Err(e) => Err(format!("Error searching JIRA tickets: {}", e)),
        }
    }

    /// Create a new JIRA ticket
    pub async fn create_jira_ticket(
        &self,
        project_key: String,
        summary: String,
        description: String,
        issue_type: Option<String>,
    ) -> Result<String, String> {
        match self
            .client
            .create_jira_ticket(
                &project_key,
                &summary,
                &description,
                issue_type.as_deref(),
            )
            .await
        {
            Ok(ticket) => {
                let ticket_key = ticket
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                Ok(format!("Created JIRA ticket: {}", ticket_key))
            }
            Err(e) => Err(format!("Error creating JIRA ticket: {}", e)),
        }
    }

    /// Add a comment to a JIRA ticket
    pub async fn add_comment_to_jira_ticket(
        &self,
        ticket_key: String,
        comment: String,
    ) -> Result<String, String> {
        match self
            .client
            .add_comment_to_jira_ticket(&ticket_key, &comment)
            .await
        {
            Ok(_) => Ok(format!("Added comment to {}", ticket_key)),
            Err(e) => Err(format!("Error adding comment to JIRA ticket: {}", e)),
        }
    }

    /// Get a Confluence page by ID
    pub async fn get_confluence_page(&self, page_id: String) -> Result<Value, String> {
        match self.client.get_confluence_page(&page_id).await {
            Ok(page) => Ok(page),
            Err(e) => Err(format!("Error getting Confluence page: {}", e)),
        }
    }

    /// Search for content in Confluence
    pub async fn search_confluence(
        &self,
        query: String,
        limit: Option<u32>,
    ) -> Result<Value, String> {
        match self.client.search_confluence(&query, limit).await {
            Ok(results) => Ok(results),
            Err(e) => Err(format!("Error searching Confluence: {}", e)),
        }
    }
}

impl ServerHandler for AtlassianMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(format!(
                "Atlassian MCP server for JIRA and Confluence integration. Connected to {}",
                self.config.atlassian.base_url
            )),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let _cli = Cli::parse();

    let config = Config::load()?;
    info!("Loaded configuration for {}", config.atlassian.base_url);

    let atlassian_server = AtlassianMcpServer::new(config.clone());

    eprintln!(
        "Atlassian MCP server running on stdio (connected to {})",
        config.atlassian.base_url
    );

    atlassian_server.serve(stdio()).await?;

    Ok(())
}