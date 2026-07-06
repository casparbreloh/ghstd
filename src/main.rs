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
        /// Apply to all non-archived repositories for the authenticated user.
        #[arg(long)]
        all: bool,
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
}

#[derive(Debug, Deserialize)]
struct ListedRepo {
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
    #[serde(rename = "isArchived")]
    is_archived: bool,
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
            let repos = resolve_repos(repo, all)?;
            status(repos)
        }
        Cmd::Apply { repo, all } => {
            let repos = resolve_repos(repo, all)?;
            apply(repos)
        }
        Cmd::Create {
            name,
            public,
            private: _,
        } => create(name, public),
    }
}

fn status(repos: Vec<String>) -> Result<()> {
    if repos.len() == 1 {
        let repo = get_repo(&repos[0])?;
        print_repo_status(&repo);
        return Ok(());
    }

    let mut drifted = 0;
    let width = repos.iter().map(String::len).max().unwrap_or(0);
    for repo_name in repos {
        let repo = get_repo(&repo_name)?;
        let drift = drift(&repo).len();
        if drift == 0 {
            println!("{repo_name:<width$}  ok");
        } else {
            drifted += 1;
            println!("{repo_name:<width$}  {drift} drift");
        }
    }
    println!();
    println!("{drifted} drift");
    Ok(())
}

fn apply(repos: Vec<String>) -> Result<()> {
    if repos.len() == 1 {
        apply_one(&repos[0], true)?;
        return Ok(());
    }

    let mut changed = 0;
    let width = repos.iter().map(String::len).max().unwrap_or(0);
    for repo in repos {
        let changes = apply_one(&repo, false)?;
        if changes == 0 {
            println!("{repo:<width$}  ok");
        } else {
            changed += 1;
            println!("{repo:<width$}  {changes} change");
        }
    }
    println!();
    println!("{changed} changed");
    Ok(())
}

fn apply_one(repo_name: &str, detailed: bool) -> Result<usize> {
    let repo = get_repo(repo_name)?;
    let changes = drift(&repo);
    if changes.is_empty() {
        if detailed {
            println!("{}", repo.full_name);
            println!("  ok");
        }
        return Ok(0);
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
        ])
        .output()
        .with_context(|| format!("failed to run gh repo edit for {}", repo.full_name))?;
    ensure_success(output, &format!("gh repo edit {}", repo.full_name))?;

    if detailed {
        println!("{}", repo.full_name);
        for rule in &changes {
            println!("  {}", changed_text(rule));
        }
    }

    Ok(changes.len())
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
    let changes = apply_one(&repo, false)?;
    if changes == 0 {
        println!("  ok");
    } else {
        println!("  applied {changes} change");
    }
    Ok(())
}

fn resolve_repos(repo: Option<String>, all: bool) -> Result<Vec<String>> {
    match (repo, all) {
        (Some(repo), false) => Ok(vec![repo]),
        (None, false) => Ok(vec![current_repo()?]),
        (None, true) => all_repos(),
        (Some(_), true) => bail!("pass either a repo or --all, not both"),
    }
}

fn all_repos() -> Result<Vec<String>> {
    let user = gh_json(&["api", "user", "--jq", ".login"])?;
    let repos: Vec<ListedRepo> = serde_json::from_str(&gh_json(&[
        "repo",
        "list",
        user.trim(),
        "--limit",
        "1000",
        "--json",
        "nameWithOwner,isArchived",
    ])?)?;
    Ok(repos
        .into_iter()
        .filter(|repo| !repo.is_archived)
        .map(|repo| repo.name_with_owner)
        .collect())
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
    ]
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
