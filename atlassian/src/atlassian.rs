use anyhow::{Context, Result};
use base64::Engine;
use reqwest::{Client, RequestBuilder};
use serde_json::Value;
use std::fmt;
use tracing::{debug, error};

use crate::config::AtlassianConfig;

#[derive(Clone)]
pub struct AtlassianClient {
    client: Client,
    config: AtlassianConfig,
    auth_header: String,
}

impl AtlassianClient {
    pub fn new(config: AtlassianConfig) -> Self {
        let client = Client::new();
        let credentials = format!("{}:{}", config.email, config.token);
        let auth_header = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes())
        );

        Self {
            client,
            config,
            auth_header,
        }
    }

    fn request(&self, method: reqwest::Method, path: &str) -> RequestBuilder {
        let url = format!("{}{}", self.config.base_url, path);
        debug!("Making {} request to: {}", method, url);
        
        self.client
            .request(method, &url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
    }

    pub async fn get_jira_ticket(&self, ticket_key: &str) -> Result<Value> {
        let response = self
            .request(reqwest::Method::GET, &format!("/rest/api/3/issue/{}", ticket_key))
            .query(&[
                ("fields", "summary,description,status,created,updated,assignee,reporter,priority,issuetype")
            ])
            .send()
            .await
            .with_context(|| format!("Failed to get JIRA ticket {}", ticket_key))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("JIRA API error: {} - {}", status, text);
            anyhow::bail!("JIRA API error: {} - {}", status, text);
        }

        let ticket: Value = response
            .json()
            .await
            .with_context(|| "Failed to parse JIRA ticket response")?;

        Ok(ticket)
    }

    pub async fn search_jira_tickets(&self, jql: &str, max_results: Option<u32>) -> Result<Value> {
        let max_results = max_results.unwrap_or(10);
        
        let response = self
            .request(reqwest::Method::GET, "/rest/api/3/search")
            .query(&[
                ("jql", jql),
                ("maxResults", &max_results.to_string()),
                ("fields", "summary,status,created,updated"),
            ])
            .send()
            .await
            .with_context(|| "Failed to search JIRA tickets")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("JIRA API error: {} - {}", status, text);
            anyhow::bail!("JIRA API error: {} - {}", status, text);
        }

        let results: Value = response
            .json()
            .await
            .with_context(|| "Failed to parse JIRA search response")?;

        Ok(results)
    }

    pub async fn create_jira_ticket(
        &self,
        project_key: &str,
        summary: &str,
        description: &str,
        issue_type: Option<&str>,
    ) -> Result<Value> {
        let issue_type = issue_type.unwrap_or("Task");
        
        let payload = serde_json::json!({
            "fields": {
                "project": {
                    "key": project_key
                },
                "summary": summary,
                "description": {
                    "type": "doc",
                    "version": 1,
                    "content": [
                        {
                            "type": "paragraph",
                            "content": [
                                {
                                    "type": "text",
                                    "text": description
                                }
                            ]
                        }
                    ]
                },
                "issuetype": {
                    "name": issue_type
                }
            }
        });

        let response = self
            .request(reqwest::Method::POST, "/rest/api/3/issue")
            .json(&payload)
            .send()
            .await
            .with_context(|| "Failed to create JIRA ticket")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("JIRA API error: {} - {}", status, text);
            anyhow::bail!("JIRA API error: {} - {}", status, text);
        }

        let ticket: Value = response
            .json()
            .await
            .with_context(|| "Failed to parse JIRA create response")?;

        Ok(ticket)
    }

    pub async fn add_comment_to_jira_ticket(&self, ticket_key: &str, comment: &str) -> Result<Value> {
        let payload = serde_json::json!({
            "body": {
                "type": "doc",
                "version": 1,
                "content": [
                    {
                        "type": "paragraph",
                        "content": [
                            {
                                "type": "text",
                                "text": comment
                            }
                        ]
                    }
                ]
            }
        });

        let response = self
            .request(reqwest::Method::POST, &format!("/rest/api/3/issue/{}/comment", ticket_key))
            .json(&payload)
            .send()
            .await
            .with_context(|| format!("Failed to add comment to JIRA ticket {}", ticket_key))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("JIRA API error: {} - {}", status, text);
            anyhow::bail!("JIRA API error: {} - {}", status, text);
        }

        let comment_response: Value = response
            .json()
            .await
            .with_context(|| "Failed to parse JIRA comment response")?;

        Ok(comment_response)
    }

    pub async fn get_confluence_page(&self, page_id: &str) -> Result<Value> {
        let response = self
            .request(reqwest::Method::GET, &format!("/wiki/rest/api/content/{}", page_id))
            .query(&[("expand", "body.storage,version,space")])
            .send()
            .await
            .with_context(|| format!("Failed to get Confluence page {}", page_id))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Confluence API error: {} - {}", status, text);
            anyhow::bail!("Confluence API error: {} - {}", status, text);
        }

        let page: Value = response
            .json()
            .await
            .with_context(|| "Failed to parse Confluence page response")?;

        Ok(page)
    }

    pub async fn search_confluence(&self, query: &str, limit: Option<u32>) -> Result<Value> {
        let limit = limit.unwrap_or(10);
        let cql = format!("text ~ \"{}\"", query);

        let response = self
            .request(reqwest::Method::GET, "/wiki/rest/api/content/search")
            .query(&[
                ("cql", &cql),
                ("limit", &limit.to_string()),
                ("expand", &"space".to_string()),
            ])
            .send()
            .await
            .with_context(|| "Failed to search Confluence")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Confluence API error: {} - {}", status, text);
            anyhow::bail!("Confluence API error: {} - {}", status, text);
        }

        let results: Value = response
            .json()
            .await
            .with_context(|| "Failed to parse Confluence search response")?;

        Ok(results)
    }

    pub async fn get_confluence_spaces(&self) -> Result<Value> {
        let response = self
            .request(reqwest::Method::GET, "/wiki/rest/api/space")
            .query(&[("limit", "25")])
            .send()
            .await
            .with_context(|| "Failed to get Confluence spaces")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Confluence API error: {} - {}", status, text);
            anyhow::bail!("Confluence API error: {} - {}", status, text);
        }

        let spaces: Value = response
            .json()
            .await
            .with_context(|| "Failed to parse Confluence spaces response")?;

        Ok(spaces)
    }

    pub async fn get_recent_jira_tickets(&self) -> Result<Value> {
        self.search_jira_tickets("ORDER BY updated DESC", Some(10)).await
    }
}

impl fmt::Debug for AtlassianClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AtlassianClient")
            .field("base_url", &self.config.base_url)
            .field("email", &self.config.email)
            .finish()
    }
}