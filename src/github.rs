use std::process::Command;

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Repo {
    pub full_name: String,
    pub private: bool,
    pub delete_branch_on_merge: bool,
    pub allow_squash_merge: bool,
    pub allow_merge_commit: bool,
    pub allow_rebase_merge: bool,
    pub allow_auto_merge: bool,
    pub allow_update_branch: bool,
    pub has_issues: bool,
    pub has_projects: bool,
    pub has_wiki: bool,
    pub has_discussions: bool,
}

#[derive(Debug, Deserialize)]
struct GraphQlResponse {
    data: GraphQlData,
}

#[derive(Debug, Deserialize)]
struct GraphQlData {
    viewer: Viewer,
}

#[derive(Debug, Deserialize)]
struct Viewer {
    repositories: RepositoryConnection,
}

#[derive(Debug, Deserialize)]
struct RepositoryConnection {
    nodes: Vec<GraphQlRepo>,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphQlRepo {
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
    #[serde(rename = "isArchived")]
    is_archived: bool,
    #[serde(rename = "isPrivate")]
    is_private: bool,
    #[serde(rename = "deleteBranchOnMerge")]
    delete_branch_on_merge: bool,
    #[serde(rename = "squashMergeAllowed")]
    squash_merge_allowed: bool,
    #[serde(rename = "mergeCommitAllowed")]
    merge_commit_allowed: bool,
    #[serde(rename = "rebaseMergeAllowed")]
    rebase_merge_allowed: bool,
    #[serde(rename = "autoMergeAllowed")]
    auto_merge_allowed: bool,
    #[serde(rename = "allowUpdateBranch")]
    allow_update_branch: bool,
    #[serde(rename = "hasIssuesEnabled")]
    has_issues_enabled: bool,
    #[serde(rename = "hasProjectsEnabled")]
    has_projects_enabled: bool,
    #[serde(rename = "hasWikiEnabled")]
    has_wiki_enabled: bool,
    #[serde(rename = "hasDiscussionsEnabled")]
    has_discussions_enabled: bool,
}

pub fn current_repo() -> Result<String> {
    Ok(json(&[
        "repo",
        "view",
        "--json",
        "nameWithOwner",
        "--jq",
        ".nameWithOwner",
    ])?
    .trim()
    .to_string())
}

pub fn get_repo(repo: &str) -> Result<Repo> {
    serde_json::from_str(&json(&["api", &format!("repos/{repo}")])?)
        .with_context(|| format!("failed to parse repo settings for {repo}"))
}

pub fn all_repos() -> Result<Vec<Repo>> {
    let mut repos = Vec::new();
    let mut cursor = None;
    loop {
        let after = cursor
            .as_ref()
            .map(|cursor| format!(", after: \"{cursor}\""))
            .unwrap_or_default();
        let query = format!(
            r#"query {{
  viewer {{
    repositories(first: 100{after}, ownerAffiliations: OWNER, isFork: false) {{
      nodes {{
        nameWithOwner
        isArchived
        isPrivate
        deleteBranchOnMerge
        squashMergeAllowed
        mergeCommitAllowed
        rebaseMergeAllowed
        autoMergeAllowed
        allowUpdateBranch
        hasIssuesEnabled
        hasProjectsEnabled
        hasWikiEnabled
        hasDiscussionsEnabled
      }}
      pageInfo {{
        hasNextPage
        endCursor
      }}
    }}
  }}
}}"#
        );
        let response: GraphQlResponse =
            serde_json::from_str(&json(&["api", "graphql", "-f", &format!("query={query}")])?)
                .context("failed to parse repository GraphQL response")?;
        let connection = response.data.viewer.repositories;
        repos.extend(
            connection
                .nodes
                .into_iter()
                .filter(|repo| !repo.is_archived)
                .map(Repo::from),
        );
        if !connection.page_info.has_next_page {
            break;
        }
        cursor = connection.page_info.end_cursor;
    }
    Ok(repos)
}

pub fn normalize_repo_name(name: &str) -> Result<String> {
    if name.contains('/') {
        return Ok(name.to_string());
    }
    let user = json(&["api", "user", "--jq", ".login"])?;
    Ok(format!("{}/{}", user.trim(), name))
}

pub fn create_repo(repo: &str, public: bool) -> Result<()> {
    let visibility = if public { "--public" } else { "--private" };
    let output = Command::new("gh")
        .args(["repo", "create", repo, visibility])
        .output()
        .with_context(|| format!("failed to run gh repo create {repo}"))?;
    ensure_success(output, &format!("gh repo create {repo}"))?;
    Ok(())
}

pub fn edit_repo(repo: &str, flags: &[String]) -> Result<()> {
    let mut args = vec!["repo".to_string(), "edit".to_string(), repo.to_string()];
    args.extend(flags.iter().cloned());
    let output = Command::new("gh")
        .args(&args)
        .output()
        .with_context(|| format!("failed to run gh repo edit for {repo}"))?;
    ensure_success(output, &format!("gh repo edit {repo}"))?;
    Ok(())
}

fn json(args: &[&str]) -> Result<String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .with_context(|| format!("failed to run gh {}", args.join(" ")))?;
    ensure_success(output, &format!("gh {}", args.join(" ")))
}

fn ensure_success(output: std::process::Output, command: &str) -> Result<String> {
    if output.status.success() {
        return String::from_utf8(output.stdout).context("gh output was not utf-8");
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let message = if stderr.is_empty() { stdout } else { stderr };
    if message.is_empty() {
        Err(anyhow!("{command} failed with {}", output.status))
    } else {
        Err(anyhow!("{command} failed: {message}"))
    }
}

impl From<GraphQlRepo> for Repo {
    fn from(repo: GraphQlRepo) -> Self {
        Self {
            full_name: repo.name_with_owner,
            private: repo.is_private,
            delete_branch_on_merge: repo.delete_branch_on_merge,
            allow_squash_merge: repo.squash_merge_allowed,
            allow_merge_commit: repo.merge_commit_allowed,
            allow_rebase_merge: repo.rebase_merge_allowed,
            allow_auto_merge: repo.auto_merge_allowed,
            allow_update_branch: repo.allow_update_branch,
            has_issues: repo.has_issues_enabled,
            has_projects: repo.has_projects_enabled,
            has_wiki: repo.has_wiki_enabled,
            has_discussions: repo.has_discussions_enabled,
        }
    }
}
