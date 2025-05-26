use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: String,
    pub html_url: String,
    pub r#type: String,
    pub site_admin: bool,
    pub company: Option<String>,
    pub blog: Option<String>,
    pub location: Option<String>,
    pub bio: Option<String>,
    pub twitter_username: Option<String>,
    pub public_repos: Option<u32>,
    pub public_gists: Option<u32>,
    pub followers: Option<u32>,
    pub following: Option<u32>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl User {
    pub fn from_octocrab(user: octocrab::models::User) -> Self {
        Self {
            id: user.id.0,
            login: user.login,
            name: user.name,
            email: user.email,
            avatar_url: user.avatar_url,
            html_url: user.html_url,
            r#type: user.r#type,
            site_admin: user.site_admin,
            company: user.company,
            blog: user.blog,
            location: user.location,
            bio: user.bio,
            twitter_username: user.twitter_username,
            public_repos: user.public_repos,
            public_gists: user.public_gists,
            followers: user.followers,
            following: user.following,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: User,
    pub private: bool,
    pub html_url: String,
    pub description: Option<String>,
    pub fork: bool,
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pushed_at: Option<DateTime<Utc>>,
    pub clone_url: String,
    pub ssh_url: String,
    pub size: u32,
    pub stargazers_count: u32,
    pub watchers_count: u32,
    pub language: Option<String>,
    pub forks_count: u32,
    pub archived: bool,
    pub disabled: bool,
    pub open_issues_count: u32,
    pub license: Option<License>,
    pub allow_forking: Option<bool>,
    pub is_template: Option<bool>,
    pub topics: Vec<String>,
    pub visibility: String,
    pub default_branch: String,
}

impl Repository {
    pub fn from_octocrab(repo: octocrab::models::Repository) -> Self {
        Self {
            id: repo.id.0,
            name: repo.name,
            full_name: repo.full_name,
            owner: User::from_octocrab(repo.owner.unwrap()),
            private: repo.private.unwrap_or(false),
            html_url: repo.html_url,
            description: repo.description,
            fork: repo.fork.unwrap_or(false),
            url: repo.url,
            created_at: repo.created_at.unwrap(),
            updated_at: repo.updated_at.unwrap(),
            pushed_at: repo.pushed_at,
            clone_url: repo.clone_url.unwrap(),
            ssh_url: repo.ssh_url.unwrap(),
            size: repo.size.unwrap_or(0),
            stargazers_count: repo.stargazers_count.unwrap_or(0),
            watchers_count: repo.watchers_count.unwrap_or(0),
            language: repo.language,
            forks_count: repo.forks_count.unwrap_or(0),
            archived: repo.archived.unwrap_or(false),
            disabled: repo.disabled.unwrap_or(false),
            open_issues_count: repo.open_issues_count.unwrap_or(0),
            license: repo.license.map(License::from_octocrab),
            allow_forking: repo.allow_forking,
            is_template: repo.is_template,
            topics: repo.topics.unwrap_or_default(),
            visibility: repo.visibility.unwrap_or_else(|| "public".to_string()),
            default_branch: repo.default_branch.unwrap_or_else(|| "main".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub key: String,
    pub name: String,
    pub spdx_id: Option<String>,
    pub url: Option<String>,
}

impl License {
    pub fn from_octocrab(license: octocrab::models::License) -> Self {
        Self {
            key: license.key,
            name: license.name,
            spdx_id: license.spdx_id,
            url: license.url,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub user: User,
    pub labels: Vec<Label>,
    pub state: String,
    pub locked: bool,
    pub assignee: Option<User>,
    pub assignees: Vec<User>,
    pub milestone: Option<Milestone>,
    pub comments: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub html_url: String,
    pub pull_request: Option<PullRequestLink>,
}

impl Issue {
    pub fn from_octocrab(issue: octocrab::models::issues::Issue) -> Self {
        Self {
            id: issue.id.0,
            number: issue.number,
            title: issue.title,
            body: issue.body,
            user: User::from_octocrab(issue.user),
            labels: issue.labels.into_iter().map(Label::from_octocrab).collect(),
            state: issue.state.to_string(),
            locked: issue.locked,
            assignee: issue.assignee.map(User::from_octocrab),
            assignees: issue.assignees.into_iter().map(User::from_octocrab).collect(),
            milestone: issue.milestone.map(Milestone::from_octocrab),
            comments: issue.comments,
            created_at: issue.created_at,
            updated_at: issue.updated_at,
            closed_at: issue.closed_at,
            html_url: issue.html_url,
            pull_request: issue.pull_request.map(PullRequestLink::from_octocrab),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: u64,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub default: bool,
}

impl Label {
    pub fn from_octocrab(label: octocrab::models::Label) -> Self {
        Self {
            id: label.id.0,
            name: label.name,
            color: label.color,
            description: label.description,
            default: label.default,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub description: Option<String>,
    pub creator: User,
    pub open_issues: u32,
    pub closed_issues: u32,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub due_on: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
}

impl Milestone {
    pub fn from_octocrab(milestone: octocrab::models::issues::Milestone) -> Self {
        Self {
            id: milestone.id.0,
            number: milestone.number,
            title: milestone.title,
            description: milestone.description,
            creator: User::from_octocrab(milestone.creator),
            open_issues: milestone.open_issues,
            closed_issues: milestone.closed_issues,
            state: milestone.state.to_string(),
            created_at: milestone.created_at,
            updated_at: milestone.updated_at,
            due_on: milestone.due_on,
            closed_at: milestone.closed_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub user: User,
    pub state: String,
    pub locked: bool,
    pub assignee: Option<User>,
    pub assignees: Vec<User>,
    pub requested_reviewers: Vec<User>,
    pub milestone: Option<Milestone>,
    pub head: GitRef,
    pub base: GitRef,
    pub merged: bool,
    pub mergeable: Option<bool>,
    pub mergeable_state: Option<String>,
    pub merged_by: Option<User>,
    pub comments: u32,
    pub review_comments: u32,
    pub maintainer_can_modify: bool,
    pub commits: u32,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub merged_at: Option<DateTime<Utc>>,
    pub html_url: String,
    pub diff_url: String,
    pub patch_url: String,
    pub draft: bool,
}

impl PullRequest {
    pub fn from_octocrab(pr: octocrab::models::pulls::PullRequest) -> Self {
        Self {
            id: pr.id.0,
            number: pr.number,
            title: pr.title,
            body: pr.body,
            user: User::from_octocrab(pr.user),
            state: pr.state.to_string(),
            locked: pr.locked,
            assignee: pr.assignee.map(User::from_octocrab),
            assignees: pr.assignees.into_iter().map(User::from_octocrab).collect(),
            requested_reviewers: pr.requested_reviewers.into_iter().map(User::from_octocrab).collect(),
            milestone: pr.milestone.map(Milestone::from_octocrab),
            head: GitRef::from_octocrab(pr.head),
            base: GitRef::from_octocrab(pr.base),
            merged: pr.merged,
            mergeable: pr.mergeable,
            mergeable_state: pr.mergeable_state,
            merged_by: pr.merged_by.map(User::from_octocrab),
            comments: pr.comments,
            review_comments: pr.review_comments,
            maintainer_can_modify: pr.maintainer_can_modify,
            commits: pr.commits,
            additions: pr.additions,
            deletions: pr.deletions,
            changed_files: pr.changed_files,
            created_at: pr.created_at,
            updated_at: pr.updated_at,
            closed_at: pr.closed_at,
            merged_at: pr.merged_at,
            html_url: pr.html_url,
            diff_url: pr.diff_url,
            patch_url: pr.patch_url,
            draft: pr.draft,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestLink {
    pub url: String,
    pub html_url: String,
    pub diff_url: String,
    pub patch_url: String,
}

impl PullRequestLink {
    pub fn from_octocrab(link: octocrab::models::issues::PullRequestLink) -> Self {
        Self {
            url: link.url,
            html_url: link.html_url,
            diff_url: link.diff_url,
            patch_url: link.patch_url,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRef {
    pub label: String,
    pub r#ref: String,
    pub sha: String,
    pub user: User,
    pub repo: Repository,
}

impl GitRef {
    pub fn from_octocrab(git_ref: octocrab::models::pulls::Head) -> Self {
        Self {
            label: git_ref.label,
            r#ref: git_ref.r#ref,
            sha: git_ref.sha,
            user: User::from_octocrab(git_ref.user),
            repo: Repository::from_octocrab(git_ref.repo),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub name: String,
    pub path: String,
    pub sha: String,
    pub size: u64,
    pub url: String,
    pub html_url: String,
    pub git_url: String,
    pub download_url: Option<String>,
    pub r#type: String,
    pub content: Option<String>,
    pub encoding: Option<String>,
    pub target: Option<String>,
    pub submodule_git_url: Option<String>,
}

impl FileContent {
    pub fn from_octocrab(content: octocrab::models::repos::Content) -> Self {
        Self {
            name: content.name,
            path: content.path,
            sha: content.sha,
            size: content.size,
            url: content.url,
            html_url: content.html_url,
            git_url: content.git_url,
            download_url: content.download_url,
            r#type: content.content_type,
            content: content.content,
            encoding: content.encoding,
            target: content.target,
            submodule_git_url: content.submodule_git_url,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCommit {
    pub content: FileContent,
    pub commit: CommitInfo,
}

impl FileCommit {
    pub fn from_octocrab(commit: octocrab::models::repos::ContentItems) -> Self {
        Self {
            content: FileContent::from_octocrab(commit.content),
            commit: CommitInfo::from_octocrab(commit.commit),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub url: String,
    pub html_url: String,
    pub author: GitActor,
    pub committer: GitActor,
    pub message: String,
    pub tree: GitTree,
    pub parents: Vec<GitParent>,
}

impl CommitInfo {
    pub fn from_octocrab(commit: octocrab::models::repos::CommitItem) -> Self {
        Self {
            sha: commit.sha,
            url: commit.url,
            html_url: commit.html_url,
            author: GitActor::from_octocrab(commit.author),
            committer: GitActor::from_octocrab(commit.committer),
            message: commit.message,
            tree: GitTree::from_octocrab(commit.tree),
            parents: commit.parents.into_iter().map(GitParent::from_octocrab).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitActor {
    pub name: String,
    pub email: String,
    pub date: DateTime<Utc>,
}

impl GitActor {
    pub fn from_octocrab(actor: octocrab::models::repos::GitActor) -> Self {
        Self {
            name: actor.name,
            email: actor.email,
            date: actor.date,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitTree {
    pub sha: String,
    pub url: String,
}

impl GitTree {
    pub fn from_octocrab(tree: octocrab::models::repos::GitTree) -> Self {
        Self {
            sha: tree.sha,
            url: tree.url,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitParent {
    pub sha: String,
    pub url: String,
    pub html_url: String,
}

impl GitParent {
    pub fn from_octocrab(parent: octocrab::models::repos::GitParent) -> Self {
        Self {
            sha: parent.sha,
            url: parent.url,
            html_url: parent.html_url,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults<T> {
    pub total_count: u32,
    pub incomplete_results: bool,
    pub items: Vec<T>,
}

impl SearchResults<Repository> {
    pub fn from_octocrab_repos(results: octocrab::models::search::SearchRepos) -> Self {
        Self {
            total_count: results.total_count,
            incomplete_results: results.incomplete_results,
            items: results.items.into_iter().map(Repository::from_octocrab).collect(),
        }
    }
}

impl SearchResults<User> {
    pub fn from_octocrab_users(results: octocrab::models::search::SearchUsers) -> Self {
        Self {
            total_count: results.total_count,
            incomplete_results: results.incomplete_results,
            items: results.items.into_iter().map(User::from_octocrab).collect(),
        }
    }
}