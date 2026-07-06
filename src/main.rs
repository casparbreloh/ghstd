use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use serde::Deserialize;

#[derive(Parser)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show whether repos match the standard
    Status {
        /// Repository as OWNER/REPO. Defaults to the current repository.
        repo: Option<String>,
        /// Check all non-archived repositories for the authenticated user.
        #[arg(long)]
        all: bool,
    },
    /// Apply the standard to repos
    Apply {
        /// Repository as OWNER/REPO. Defaults to the current repository.
        repo: Option<String>,
    },
    /// Create a repo and apply the standard
    Create {
        /// Repository name, or OWNER/REPO.
        name: String,
        /// Create a public repository.
        #[arg(long, conflicts_with = "private")]
        public: bool,
        /// Create a private repository. This is the default.
        #[arg(long)]
        private: bool,
    },
}

#[derive(Debug, Deserialize)]
struct Repo {
    full_name: String,
    delete_branch_on_merge: bool,
    allow_squash_merge: bool,
    allow_merge_commit: bool,
    allow_rebase_merge: bool,
    allow_auto_merge: bool,
    allow_update_branch: bool,
    has_issues: bool,
    has_projects: bool,
    has_wiki: bool,
    has_discussions: bool,
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

#[derive(Debug)]
struct Rule {
    label: &'static str,
    current: bool,
    desired: bool,
    enabled_text: &'static str,
    disabled_text: &'static str,
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Status { repo, all } => {
            if all {
                if repo.is_some() {
                    bail!("pass either a repo or --all, not both");
                }
                status_all(all_repos()?)
            } else {
                status_one(&repo.unwrap_or(current_repo()?))
            }
        }
        Cmd::Apply { repo } => apply(&repo.unwrap_or(current_repo()?)),
        Cmd::Create {
            name,
            public,
            private: _,
        } => create(name, public),
    }
}

fn status_one(repo_name: &str) -> Result<()> {
    let repo = get_repo(repo_name)?;
    print_repo_status(&repo);
    Ok(())
}

fn status_all(repos: Vec<Repo>) -> Result<()> {
    let mut drifted = 0;
    let width = repos
        .iter()
        .map(|repo| repo.full_name.len())
        .max()
        .unwrap_or(0);
    for repo in repos {
        let drift = drift(&repo).len();
        if drift == 0 {
            println!("{:<width$}  ok", repo.full_name);
        } else {
            drifted += 1;
            println!("{:<width$}  {drift} drift", repo.full_name);
        }
    }
    println!();
    println!("{drifted} drift");
    Ok(())
}

fn apply(repo_name: &str) -> Result<()> {
    let (full_name, changes) = apply_standard(repo_name)?;
    println!("{full_name}");
    if changes.is_empty() {
        println!("  ok");
    } else {
        for rule in &changes {
            println!("  {}", changed_text(rule));
        }
    }
    Ok(())
}

fn apply_standard(repo_name: &str) -> Result<(String, Vec<Rule>)> {
    let repo = get_repo(repo_name)?;
    let changes = drift(&repo);
    if changes.is_empty() {
        return Ok((repo.full_name, changes));
    }

    let output = Command::new("gh")
        .args([
            "repo",
            "edit",
            &repo.full_name,
            "--delete-branch-on-merge",
            "--enable-squash-merge",
            "--enable-merge-commit=false",
            "--enable-rebase-merge=false",
            "--enable-auto-merge",
            "--allow-update-branch",
            "--enable-issues=false",
            "--enable-projects=false",
            "--enable-wiki=false",
            "--enable-discussions=false",
        ])
        .output()
        .with_context(|| format!("failed to run gh repo edit for {}", repo.full_name))?;
    ensure_success(output, &format!("gh repo edit {}", repo.full_name))?;

    Ok((repo.full_name, changes))
}

fn create(name: String, public: bool) -> Result<()> {
    let repo = normalize_repo_name(&name)?;
    let visibility = if public { "--public" } else { "--private" };
    let output = Command::new("gh")
        .args(["repo", "create", &repo, visibility])
        .output()
        .with_context(|| format!("failed to run gh repo create {repo}"))?;
    ensure_success(output, &format!("gh repo create {repo}"))?;

    println!("{repo}");
    println!("  created");
    let (_, changes) = apply_standard(&repo)?;
    if changes.is_empty() {
        println!("  ok");
    } else {
        for rule in &changes {
            println!("  {}", changed_text(rule));
        }
    }
    Ok(())
}

fn all_repos() -> Result<Vec<Repo>> {
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
        let response: GraphQlResponse = serde_json::from_str(&gh_json(&[
            "api",
            "graphql",
            "-f",
            &format!("query={query}"),
        ])?)
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

fn current_repo() -> Result<String> {
    Ok(gh_json(&[
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

fn get_repo(repo: &str) -> Result<Repo> {
    serde_json::from_str(&gh_json(&["api", &format!("repos/{repo}")])?)
        .with_context(|| format!("failed to parse repo settings for {repo}"))
}

fn normalize_repo_name(name: &str) -> Result<String> {
    if name.contains('/') {
        return Ok(name.to_string());
    }
    let user = gh_json(&["api", "user", "--jq", ".login"])?;
    Ok(format!("{}/{}", user.trim(), name))
}

fn print_repo_status(repo: &Repo) {
    println!("{}", repo.full_name);
    let changes = drift(repo);
    if changes.is_empty() {
        println!("  ok");
        return;
    }
    for rule in rules(repo) {
        let state = if rule.current == rule.desired {
            "ok"
        } else {
            "drift"
        };
        println!("  {state:<5} {}", current_text(&rule));
    }
    println!();
    println!("{} drift", changes.len());
}

fn drift(repo: &Repo) -> Vec<Rule> {
    rules(repo)
        .into_iter()
        .filter(|rule| rule.current != rule.desired)
        .collect()
}

fn rules(repo: &Repo) -> Vec<Rule> {
    vec![
        Rule {
            label: "delete branches on merge",
            current: repo.delete_branch_on_merge,
            desired: true,
            enabled_text: "delete branches on merge",
            disabled_text: "keep branches after merge",
        },
        Rule {
            label: "squash merge",
            current: repo.allow_squash_merge,
            desired: true,
            enabled_text: "squash merge enabled",
            disabled_text: "squash merge disabled",
        },
        Rule {
            label: "merge commits",
            current: repo.allow_merge_commit,
            desired: false,
            enabled_text: "merge commits enabled",
            disabled_text: "merge commits disabled",
        },
        Rule {
            label: "rebase merge",
            current: repo.allow_rebase_merge,
            desired: false,
            enabled_text: "rebase merge enabled",
            disabled_text: "rebase merge disabled",
        },
        Rule {
            label: "auto-merge",
            current: repo.allow_auto_merge,
            desired: true,
            enabled_text: "auto-merge enabled",
            disabled_text: "auto-merge disabled",
        },
        Rule {
            label: "update branch suggestions",
            current: repo.allow_update_branch,
            desired: true,
            enabled_text: "update branch suggestions enabled",
            disabled_text: "update branch suggestions disabled",
        },
        Rule {
            label: "issues",
            current: repo.has_issues,
            desired: false,
            enabled_text: "issues enabled",
            disabled_text: "issues disabled",
        },
        Rule {
            label: "projects",
            current: repo.has_projects,
            desired: false,
            enabled_text: "projects enabled",
            disabled_text: "projects disabled",
        },
        Rule {
            label: "wiki",
            current: repo.has_wiki,
            desired: false,
            enabled_text: "wiki enabled",
            disabled_text: "wiki disabled",
        },
        Rule {
            label: "discussions",
            current: repo.has_discussions,
            desired: false,
            enabled_text: "discussions enabled",
            disabled_text: "discussions disabled",
        },
    ]
}

impl From<GraphQlRepo> for Repo {
    fn from(repo: GraphQlRepo) -> Self {
        Self {
            full_name: repo.name_with_owner,
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

fn current_text(rule: &Rule) -> &'static str {
    if rule.current {
        rule.enabled_text
    } else {
        rule.disabled_text
    }
}

fn changed_text(rule: &Rule) -> String {
    let state = if rule.desired { "enabled" } else { "disabled" };
    format!("{} {state}", rule.label)
}

fn gh_json(args: &[&str]) -> Result<String> {
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
