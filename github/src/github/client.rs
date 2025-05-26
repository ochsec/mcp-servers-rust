use anyhow::{anyhow, Result};
use octocrab::{Octocrab, OctocrabBuilder};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, AUTHORIZATION};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, error, info};
use url::Url;

use super::types::*;

#[derive(Debug, Clone)]
pub struct GitHubConfig {
    pub token: String,
    pub host: Option<String>,
    pub user_agent: String,
}

pub struct GitHubClient {
    rest_client: Arc<Octocrab>,
    graphql_client: Arc<reqwest::Client>,
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
        
        let octocrab = OctocrabBuilder::new()
            .personal_token(&config.token)
            .base_uri(&api_urls.rest_base)?
            .add_header(USER_AGENT, config.user_agent.clone())
            .build()?;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.token))?,
        );
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&config.user_agent)?,
        );

        let graphql_client = Arc::new(
            reqwest::Client::builder()
                .default_headers(headers)
                .build()?
        );

        info!("GitHub client initialized for host: {:?}", config.host);

        Ok(Self {
            rest_client: Arc::new(octocrab),
            graphql_client,
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

    pub fn rest(&self) -> &Octocrab {
        &self.rest_client
    }

    pub async fn graphql_query(&self, query: &str, variables: Option<Value>) -> Result<Value> {
        let request_body = serde_json::json!({
            "query": query,
            "variables": variables.unwrap_or(Value::Null)
        });

        debug!("Executing GraphQL query: {}", query);

        let response = self.graphql_client
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
        let repo_data = self.rest()
            .repos(owner, repo)
            .get()
            .await?;

        Ok(Repository::from_octocrab(repo_data))
    }

    pub async fn get_file_contents(&self, owner: &str, repo: &str, path: &str, reference: Option<&str>) -> Result<FileContent> {
        let mut request = self.rest()
            .repos(owner, repo)
            .get_content()
            .path(path);

        if let Some(ref_name) = reference {
            request = request.r#ref(ref_name);
        }

        let content = request.send().await?;
        Ok(FileContent::from_octocrab(content))
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
        let mut request = self.rest()
            .repos(owner, repo)
            .update_file(path, message, content);

        if let Some(sha) = sha {
            request = request.sha(sha);
        }

        if let Some(branch) = branch {
            request = request.branch(branch);
        }

        let commit = request.send().await?;
        Ok(FileCommit::from_octocrab(commit))
    }

    pub async fn search_repositories(&self, query: &str, sort: Option<&str>, order: Option<&str>, per_page: Option<u8>, page: Option<u32>) -> Result<SearchResults<Repository>> {
        let mut search = self.rest().search().repositories(query);

        if let Some(sort) = sort {
            search = search.sort(sort);
        }

        if let Some(order) = order {
            search = search.order(order);
        }

        if let Some(per_page) = per_page {
            search = search.per_page(per_page);
        }

        if let Some(page) = page {
            search = search.page(page);
        }

        let results = search.send().await?;
        Ok(SearchResults::from_octocrab_repos(results))
    }

    // Issue operations
    pub async fn get_issue(&self, owner: &str, repo: &str, number: u64) -> Result<Issue> {
        let issue = self.rest()
            .issues(owner, repo)
            .get(number)
            .await?;

        Ok(Issue::from_octocrab(issue))
    }

    pub async fn list_issues(&self, owner: &str, repo: &str, state: Option<&str>, labels: Option<Vec<String>>, assignee: Option<&str>, creator: Option<&str>, mentioned: Option<&str>, milestone: Option<&str>, sort: Option<&str>, direction: Option<&str>, since: Option<&str>, per_page: Option<u8>, page: Option<u32>) -> Result<Vec<Issue>> {
        let mut request = self.rest().issues(owner, repo).list();

        if let Some(state) = state {
            request = request.state(octocrab::params::State::from(state));
        }

        if let Some(labels) = labels {
            request = request.labels(&labels.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        }

        if let Some(assignee) = assignee {
            request = request.assignee(assignee);
        }

        if let Some(creator) = creator {
            request = request.creator(creator);
        }

        if let Some(mentioned) = mentioned {
            request = request.mentioned(mentioned);
        }

        if let Some(milestone) = milestone {
            if milestone == "none" {
                request = request.milestone("");
            } else if milestone != "*" {
                request = request.milestone(milestone);
            }
        }

        if let Some(sort) = sort {
            request = request.sort(octocrab::params::issues::Sort::from(sort));
        }

        if let Some(direction) = direction {
            request = request.direction(octocrab::params::Direction::from(direction));
        }

        if let Some(per_page) = per_page {
            request = request.per_page(per_page);
        }

        if let Some(page) = page {
            request = request.page(page);
        }

        let issues = request.send().await?;
        Ok(issues.items.into_iter().map(Issue::from_octocrab).collect())
    }

    pub async fn create_issue(&self, owner: &str, repo: &str, title: &str, body: Option<&str>, assignees: Option<Vec<String>>, milestone: Option<u64>, labels: Option<Vec<String>>) -> Result<Issue> {
        let mut request = self.rest()
            .issues(owner, repo)
            .create(title);

        if let Some(body) = body {
            request = request.body(body);
        }

        if let Some(assignees) = assignees {
            request = request.assignees(assignees);
        }

        if let Some(milestone) = milestone {
            request = request.milestone(milestone);
        }

        if let Some(labels) = labels {
            request = request.labels(labels);
        }

        let issue = request.send().await?;
        Ok(Issue::from_octocrab(issue))
    }

    // Pull request operations
    pub async fn get_pull_request(&self, owner: &str, repo: &str, number: u64) -> Result<PullRequest> {
        let pr = self.rest()
            .pulls(owner, repo)
            .get(number)
            .await?;

        Ok(PullRequest::from_octocrab(pr))
    }

    pub async fn list_pull_requests(&self, owner: &str, repo: &str, state: Option<&str>, head: Option<&str>, base: Option<&str>, sort: Option<&str>, direction: Option<&str>, per_page: Option<u8>, page: Option<u32>) -> Result<Vec<PullRequest>> {
        let mut request = self.rest().pulls(owner, repo).list();

        if let Some(state) = state {
            request = request.state(octocrab::params::State::from(state));
        }

        if let Some(head) = head {
            request = request.head(head);
        }

        if let Some(base) = base {
            request = request.base(base);
        }

        if let Some(sort) = sort {
            request = request.sort(octocrab::params::pulls::Sort::from(sort));
        }

        if let Some(direction) = direction {
            request = request.direction(octocrab::params::Direction::from(direction));
        }

        if let Some(per_page) = per_page {
            request = request.per_page(per_page);
        }

        if let Some(page) = page {
            request = request.page(page);
        }

        let prs = request.send().await?;
        Ok(prs.items.into_iter().map(PullRequest::from_octocrab).collect())
    }

    pub async fn create_pull_request(&self, owner: &str, repo: &str, title: &str, head: &str, base: &str, body: Option<&str>, draft: Option<bool>) -> Result<PullRequest> {
        let mut request = self.rest()
            .pulls(owner, repo)
            .create(title, head, base);

        if let Some(body) = body {
            request = request.body(body);
        }

        if let Some(draft) = draft {
            request = request.draft(draft);
        }

        let pr = request.send().await?;
        Ok(PullRequest::from_octocrab(pr))
    }

    // User operations
    pub async fn get_authenticated_user(&self) -> Result<User> {
        let user = self.rest().current().user().await?;
        Ok(User::from_octocrab(user))
    }

    pub async fn search_users(&self, query: &str, sort: Option<&str>, order: Option<&str>, per_page: Option<u8>, page: Option<u32>) -> Result<SearchResults<User>> {
        let mut search = self.rest().search().users(query);

        if let Some(sort) = sort {
            search = search.sort(sort);
        }

        if let Some(order) = order {
            search = search.order(order);
        }

        if let Some(per_page) = per_page {
            search = search.per_page(per_page);
        }

        if let Some(page) = page {
            search = search.page(page);
        }

        let results = search.send().await?;
        Ok(SearchResults::from_octocrab_users(results))
    }
}