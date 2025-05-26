use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, AUTHORIZATION};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, error, info};
use url::Url;

use super::simple_types::*;

#[derive(Debug, Clone)]
pub struct GitHubConfig {
    pub token: String,
    pub host: Option<String>,
    pub user_agent: String,
}

pub struct GitHubClient {
    client: reqwest::Client,
    config: GitHubConfig,
    api_urls: ApiUrls,
}

#[derive(Debug, Clone)]
struct ApiUrls {
    rest_base: Url,
    graphql: Url,
    upload: Url,
}

impl GitHubClient {
    pub async fn new(config: GitHubConfig) -> Result<Self> {
        let api_urls = Self::parse_api_host(&config.host)?;
        
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.token))?,
        );
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&config.user_agent)?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        info!("GitHub client initialized for host: {:?}", config.host);

        Ok(Self {
            client,
            config,
            api_urls,
        })
    }

    fn parse_api_host(host: &Option<String>) -> Result<ApiUrls> {
        match host {
            None => Self::dotcom_urls(),
            Some(s) if s.is_empty() => Self::dotcom_urls(),
            Some(host) => {
                let url = Url::parse(host)
                    .map_err(|_| anyhow!("Invalid host URL: {}", host))?;
                
                let hostname = url.host_str()
                    .ok_or_else(|| anyhow!("Invalid hostname in URL: {}", host))?;

                if hostname.ends_with("github.com") {
                    Self::dotcom_urls()
                } else if hostname.ends_with("ghe.com") {
                    Self::ghec_urls(hostname)
                } else {
                    Self::ghes_urls(&url)
                }
            }
        }
    }

    fn dotcom_urls() -> Result<ApiUrls> {
        Ok(ApiUrls {
            rest_base: Url::parse("https://api.github.com/")?,
            graphql: Url::parse("https://api.github.com/graphql")?,
            upload: Url::parse("https://uploads.github.com/")?,
        })
    }

    fn ghec_urls(hostname: &str) -> Result<ApiUrls> {
        Ok(ApiUrls {
            rest_base: Url::parse(&format!("https://api.{}/", hostname))?,
            graphql: Url::parse(&format!("https://api.{}/graphql", hostname))?,
            upload: Url::parse(&format!("https://uploads.{}/", hostname))?,
        })
    }

    fn ghes_urls(url: &Url) -> Result<ApiUrls> {
        let scheme = url.scheme();
        let hostname = url.host_str()
            .ok_or_else(|| anyhow!("Invalid hostname in GHES URL"))?;

        Ok(ApiUrls {
            rest_base: Url::parse(&format!("{}://{}/api/v3/", scheme, hostname))?,
            graphql: Url::parse(&format!("{}://{}/api/graphql", scheme, hostname))?,
            upload: Url::parse(&format!("{}://{}/api/uploads/", scheme, hostname))?,
        })
    }

    pub async fn graphql_query(&self, query: &str, variables: Option<Value>) -> Result<Value> {
        let request_body = serde_json::json!({
            "query": query,
            "variables": variables.unwrap_or(Value::Null)
        });

        debug!("Executing GraphQL query: {}", query);

        let response = self.client
            .post(self.api_urls.graphql.clone())
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("GraphQL request failed: {}", error_text);
            return Err(anyhow!("GraphQL request failed: {}", error_text));
        }

        let response_data: Value = response.json().await?;
        
        if let Some(errors) = response_data.get("errors") {
            error!("GraphQL errors: {}", errors);
            return Err(anyhow!("GraphQL errors: {}", errors));
        }

        response_data.get("data")
            .cloned()
            .ok_or_else(|| anyhow!("No data in GraphQL response"))
    }

    // Repository operations
    pub async fn get_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
        let url = format!("{}repos/{}/{}", self.api_urls.rest_base, owner, repo);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to get repository: {}", response.status()));
        }
        
        let repo_data: Value = response.json().await?;
        self.parse_repository(repo_data)
    }

    pub async fn get_file_contents(&self, owner: &str, repo: &str, path: &str, reference: Option<&str>) -> Result<FileContent> {
        let mut url = format!("{}repos/{}/{}/contents/{}", self.api_urls.rest_base, owner, repo, path);
        
        if let Some(ref_name) = reference {
            url = format!("{}?ref={}", url, ref_name);
        }
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to get file contents: {}", response.status()));
        }
        
        let content_data: Value = response.json().await?;
        self.parse_file_content(content_data)
    }

    pub async fn create_or_update_file(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
        content: &str,
        message: &str,
        sha: Option<&str>,
        branch: Option<&str>,
    ) -> Result<FileCommit> {
        let url = format!("{}repos/{}/{}/contents/{}", self.api_urls.rest_base, owner, repo, path);
        
        let mut body = serde_json::json!({
            "message": message,
            "content": base64::encode(content.as_bytes())
        });
        
        if let Some(sha) = sha {
            body["sha"] = Value::String(sha.to_string());
        }
        
        if let Some(branch) = branch {
            body["branch"] = Value::String(branch.to_string());
        }
        
        let response = self.client.put(&url).json(&body).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to create/update file: {}", response.status()));
        }
        
        let commit_data: Value = response.json().await?;
        self.parse_file_commit(commit_data)
    }

    pub async fn search_repositories(&self, query: &str, sort: Option<&str>, order: Option<&str>, per_page: Option<u8>, page: Option<u32>) -> Result<SearchResults<Repository>> {
        let mut url = format!("{}search/repositories?q={}", self.api_urls.rest_base, urlencoding::encode(query));
        
        if let Some(sort) = sort {
            url = format!("{}&sort={}", url, sort);
        }
        
        if let Some(order) = order {
            url = format!("{}&order={}", url, order);
        }
        
        if let Some(per_page) = per_page {
            url = format!("{}&per_page={}", url, per_page);
        }
        
        if let Some(page) = page {
            url = format!("{}&page={}", url, page);
        }
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to search repositories: {}", response.status()));
        }
        
        let search_data: Value = response.json().await?;
        self.parse_repository_search_results(search_data)
    }

    // Issue operations
    pub async fn get_issue(&self, owner: &str, repo: &str, number: u64) -> Result<Issue> {
        let url = format!("{}repos/{}/{}/issues/{}", self.api_urls.rest_base, owner, repo, number);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to get issue: {}", response.status()));
        }
        
        let issue_data: Value = response.json().await?;
        self.parse_issue(issue_data)
    }

    pub async fn list_issues(&self, owner: &str, repo: &str, state: Option<&str>, labels: Option<Vec<String>>, assignee: Option<&str>, creator: Option<&str>, mentioned: Option<&str>, milestone: Option<&str>, sort: Option<&str>, direction: Option<&str>, since: Option<&str>, per_page: Option<u8>, page: Option<u32>) -> Result<Vec<Issue>> {
        let mut url = format!("{}repos/{}/{}/issues", self.api_urls.rest_base, owner, repo);
        let mut params = Vec::new();
        
        if let Some(state) = state {
            params.push(format!("state={}", state));
        }
        
        if let Some(labels) = labels {
            if !labels.is_empty() {
                params.push(format!("labels={}", labels.join(",")));
            }
        }
        
        if let Some(assignee) = assignee {
            params.push(format!("assignee={}", assignee));
        }
        
        if let Some(creator) = creator {
            params.push(format!("creator={}", creator));
        }
        
        if let Some(per_page) = per_page {
            params.push(format!("per_page={}", per_page));
        }
        
        if let Some(page) = page {
            params.push(format!("page={}", page));
        }
        
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to list issues: {}", response.status()));
        }
        
        let issues_data: Value = response.json().await?;
        self.parse_issues_list(issues_data)
    }

    pub async fn create_issue(&self, owner: &str, repo: &str, title: &str, body: Option<&str>, assignees: Option<Vec<String>>, milestone: Option<u64>, labels: Option<Vec<String>>) -> Result<Issue> {
        let url = format!("{}repos/{}/{}/issues", self.api_urls.rest_base, owner, repo);
        
        let mut body_json = serde_json::json!({
            "title": title
        });
        
        if let Some(body) = body {
            body_json["body"] = Value::String(body.to_string());
        }
        
        if let Some(assignees) = assignees {
            body_json["assignees"] = serde_json::to_value(assignees)?;
        }
        
        if let Some(milestone) = milestone {
            body_json["milestone"] = Value::Number(milestone.into());
        }
        
        if let Some(labels) = labels {
            body_json["labels"] = serde_json::to_value(labels)?;
        }
        
        let response = self.client.post(&url).json(&body_json).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to create issue: {}", response.status()));
        }
        
        let issue_data: Value = response.json().await?;
        self.parse_issue(issue_data)
    }

    // Pull request operations
    pub async fn get_pull_request(&self, owner: &str, repo: &str, number: u64) -> Result<PullRequest> {
        let url = format!("{}repos/{}/{}/pulls/{}", self.api_urls.rest_base, owner, repo, number);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to get pull request: {}", response.status()));
        }
        
        let pr_data: Value = response.json().await?;
        self.parse_pull_request(pr_data)
    }

    pub async fn list_pull_requests(&self, owner: &str, repo: &str, state: Option<&str>, head: Option<&str>, base: Option<&str>, sort: Option<&str>, direction: Option<&str>, per_page: Option<u8>, page: Option<u32>) -> Result<Vec<PullRequest>> {
        let mut url = format!("{}repos/{}/{}/pulls", self.api_urls.rest_base, owner, repo);
        let mut params = Vec::new();
        
        if let Some(state) = state {
            params.push(format!("state={}", state));
        }
        
        if let Some(head) = head {
            params.push(format!("head={}", head));
        }
        
        if let Some(base) = base {
            params.push(format!("base={}", base));
        }
        
        if let Some(per_page) = per_page {
            params.push(format!("per_page={}", per_page));
        }
        
        if let Some(page) = page {
            params.push(format!("page={}", page));
        }
        
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to list pull requests: {}", response.status()));
        }
        
        let prs_data: Value = response.json().await?;
        self.parse_pull_requests_list(prs_data)
    }

    pub async fn create_pull_request(&self, owner: &str, repo: &str, title: &str, head: &str, base: &str, body: Option<&str>, draft: Option<bool>) -> Result<PullRequest> {
        let url = format!("{}repos/{}/{}/pulls", self.api_urls.rest_base, owner, repo);
        
        let mut body_json = serde_json::json!({
            "title": title,
            "head": head,
            "base": base
        });
        
        if let Some(body) = body {
            body_json["body"] = Value::String(body.to_string());
        }
        
        if let Some(draft) = draft {
            body_json["draft"] = Value::Bool(draft);
        }
        
        let response = self.client.post(&url).json(&body_json).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to create pull request: {}", response.status()));
        }
        
        let pr_data: Value = response.json().await?;
        self.parse_pull_request(pr_data)
    }

    // User operations
    pub async fn get_authenticated_user(&self) -> Result<User> {
        let url = format!("{}user", self.api_urls.rest_base);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to get authenticated user: {}", response.status()));
        }
        
        let user_data: Value = response.json().await?;
        self.parse_user(user_data)
    }

    pub async fn search_users(&self, query: &str, sort: Option<&str>, order: Option<&str>, per_page: Option<u8>, page: Option<u32>) -> Result<SearchResults<User>> {
        let mut url = format!("{}search/users?q={}", self.api_urls.rest_base, urlencoding::encode(query));
        
        if let Some(sort) = sort {
            url = format!("{}&sort={}", url, sort);
        }
        
        if let Some(order) = order {
            url = format!("{}&order={}", url, order);
        }
        
        if let Some(per_page) = per_page {
            url = format!("{}&per_page={}", url, per_page);
        }
        
        if let Some(page) = page {
            url = format!("{}&page={}", url, page);
        }
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to search users: {}", response.status()));
        }
        
        let search_data: Value = response.json().await?;
        self.parse_user_search_results(search_data)
    }

    // Helper parsing methods - simplified implementations
    fn parse_user(&self, data: Value) -> Result<User> {
        serde_json::from_value(data).map_err(|e| anyhow!("Failed to parse user: {}", e))
    }

    fn parse_repository(&self, data: Value) -> Result<Repository> {
        serde_json::from_value(data).map_err(|e| anyhow!("Failed to parse repository: {}", e))
    }

    fn parse_issue(&self, data: Value) -> Result<Issue> {
        serde_json::from_value(data).map_err(|e| anyhow!("Failed to parse issue: {}", e))
    }

    fn parse_pull_request(&self, data: Value) -> Result<PullRequest> {
        serde_json::from_value(data).map_err(|e| anyhow!("Failed to parse pull request: {}", e))
    }

    fn parse_file_content(&self, data: Value) -> Result<FileContent> {
        serde_json::from_value(data).map_err(|e| anyhow!("Failed to parse file content: {}", e))
    }

    fn parse_file_commit(&self, data: Value) -> Result<FileCommit> {
        serde_json::from_value(data).map_err(|e| anyhow!("Failed to parse file commit: {}", e))
    }

    fn parse_repository_search_results(&self, data: Value) -> Result<SearchResults<Repository>> {
        serde_json::from_value(data).map_err(|e| anyhow!("Failed to parse repository search results: {}", e))
    }

    fn parse_user_search_results(&self, data: Value) -> Result<SearchResults<User>> {
        serde_json::from_value(data).map_err(|e| anyhow!("Failed to parse user search results: {}", e))
    }

    fn parse_issues_list(&self, data: Value) -> Result<Vec<Issue>> {
        serde_json::from_value(data).map_err(|e| anyhow!("Failed to parse issues list: {}", e))
    }

    fn parse_pull_requests_list(&self, data: Value) -> Result<Vec<PullRequest>> {
        serde_json::from_value(data).map_err(|e| anyhow!("Failed to parse pull requests list: {}", e))
    }
}