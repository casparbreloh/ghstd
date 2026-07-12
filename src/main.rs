mod config;
mod github;
mod standard;

use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, Subcommand};

use crate::{
    config::Config,
    github::Repo,
    standard::{Edit, Rule, drift, has_rules, rules},
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
                status_one(&repo.unwrap_or(github::current_repo()?), &config)
            }
        }
        Cmd::Apply { repo } => apply(&repo.unwrap_or(github::current_repo()?), &config),
        Cmd::Create { name, public, .. } => create(name, public, &config),
    }
}

fn status_one(repo_name: &str, config: &Config) -> Result<()> {
    let repo = github::get_repo(repo_name)?;
    print_repo_status(&repo, config);
    Ok(())
}

fn status_all(repos: Vec<Repo>, config: &Config) -> Result<()> {
    if repos.iter().all(|repo| !has_rules(repo, config)) {
        println!("no rules configured");
        return Ok(());
    }

    let mut drifted = 0;
    let width = repos
        .iter()
        .map(|repo| repo.full_name.len())
        .max()
        .unwrap_or(0);
    for repo in repos {
        let drift = drift(&repo, config).len();
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

fn apply(repo_name: &str, config: &Config) -> Result<()> {
    let repo = github::get_repo(repo_name)?;
    if !has_rules(&repo, config) {
        println!("{}", repo.full_name);
        println!("  no rules configured");
        return Ok(());
    }

    let (full_name, changes) = apply_standard(repo, config)?;
    print_changes(&full_name, &changes);
    Ok(())
}

fn create(name: String, public: bool, config: &Config) -> Result<()> {
    let repo = github::normalize_repo_name(&name)?;
    github::create_repo(&repo, public)?;

    println!("{repo}");
    println!("  created");
    let repo = github::get_repo(&repo)?;
    let (_, changes) = apply_standard(repo, config)?;
    if changes.is_empty() {
        println!("  ok");
    } else {
        for rule in &changes {
            println!("  {}", rule.changed_text());
        }
    }
    Ok(())
}

fn apply_standard(repo: Repo, config: &Config) -> Result<(String, Vec<Rule>)> {
    let changes = drift(&repo, config);
    if changes.is_empty() {
        return Ok((repo.full_name, changes));
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
    Ok((repo.full_name, changes))
}

fn print_repo_status(repo: &Repo, config: &Config) {
    println!("{}", repo.full_name);
    if !has_rules(repo, config) {
        println!("  no rules configured");
        return;
    }

    let changes = drift(repo, config);
    if changes.is_empty() {
        println!("  ok");
        return;
    }
    for rule in rules(repo, config) {
        let state = if rule.matches() { "ok" } else { "drift" };
        println!("  {state:<5} {}", rule.current_text());
    }
    println!();
    println!("{} drift", changes.len());
}

fn print_changes(full_name: &str, changes: &[Rule]) {
    println!("{full_name}");
    if changes.is_empty() {
        println!("  ok");
    } else {
        for rule in changes {
            println!("  {}", rule.changed_text());
        }
    }
}
