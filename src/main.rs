mod config;
mod github;
mod standard;

use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, Subcommand};

use crate::{
    config::Config,
    github::Repo,
    standard::{Edit, Rule, drift, has_rules},
};

#[derive(Parser)]
#[command(arg_required_else_help = true)]
struct Cli {
    /// Emit the CLI's usage specification.
    #[arg(long, hide = true)]
    usage_spec: bool,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show whether repos match the configured standard
    Status {
        /// Repository as OWNER/REPO. Defaults to the current repository.
        repo: Option<String>,
        /// Check all non-archived repositories for the authenticated user.
        #[arg(long)]
        all: bool,
    },
    /// Apply the configured standard to a repo
    Apply {
        /// Repository as OWNER/REPO. Defaults to the current repository.
        repo: Option<String>,
    },
    /// Create a repo and apply the configured standard
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

fn main() -> Result<()> {
    if std::env::args_os().len() == 2
        && std::env::args_os().nth(1).as_deref() == Some("--usage-spec".as_ref())
    {
        clap_usage::generate(&mut Cli::command(), "ghstd", &mut std::io::stdout());
        return Ok(());
    }

    let cli = Cli::parse();
    let config = config::load()?;
    match cli.cmd {
        Cmd::Status { repo, all } => {
            if all {
                if repo.is_some() {
                    bail!("pass either a repo or --all, not both");
                }
                status_all(github::all_repos()?, &config)
            } else {
                status_one(&repo_or_current(repo)?, &config)
            }
        }
        Cmd::Apply { repo } => apply(&repo_or_current(repo)?, &config),
        Cmd::Create { name, public, .. } => create(name, public, &config),
    }
}

fn repo_or_current(repo: Option<String>) -> Result<String> {
    repo.map_or_else(github::current_repo, Ok)
}

fn status_one(repo_name: &str, config: &Config) -> Result<()> {
    let repo = github::get_repo(repo_name)?;
    println!("{}", repo_status(&repo, config));
    Ok(())
}

fn status_all(repos: Vec<Repo>, config: &Config) -> Result<()> {
    let width = repos
        .iter()
        .map(|repo| repo.full_name.len())
        .max()
        .unwrap_or(0);
    for repo in repos {
        println!("{:<width$}  {}", repo.full_name, repo_status(&repo, config));
    }
    Ok(())
}

fn apply(repo_name: &str, config: &Config) -> Result<()> {
    let repo = github::get_repo(repo_name)?;
    if !has_rules(&repo, config) {
        println!("no rules");
        return Ok(());
    }

    let changes = apply_standard(repo, config)?;
    let result = if changes.is_empty() { "ok" } else { "applied" };
    println!("{result}");
    Ok(())
}

fn create(name: String, public: bool, config: &Config) -> Result<()> {
    let repo = github::normalize_repo_name(&name)?;
    github::create_repo(&repo, public)?;

    let repo = github::get_repo(&repo)?;
    apply_standard(repo, config)?;
    println!("created");
    Ok(())
}

fn apply_standard(repo: Repo, config: &Config) -> Result<Vec<Rule>> {
    let changes = drift(&repo, config);
    if changes.is_empty() {
        return Ok(changes);
    }

    let mut flags = Vec::new();
    let mut fields = Vec::new();
    for change in &changes {
        match change.edit() {
            Edit::Flag(flag) => flags.push(flag.clone()),
            Edit::Patch { field, value } => fields.push((field.clone(), value.clone())),
        }
    }
    if !flags.is_empty() {
        github::edit_repo(&repo.full_name, &flags)?;
    }
    if !fields.is_empty() {
        github::patch_repo(&repo.full_name, &fields)?;
    }
    Ok(changes)
}

fn repo_status(repo: &Repo, config: &Config) -> String {
    if !has_rules(repo, config) {
        return "no rules".to_string();
    }

    match drift(repo, config).len() {
        0 => "ok".to_string(),
        count => format!("{count} drift"),
    }
}
