use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub standard: Option<Settings>,
    pub public: Option<Settings>,
    pub private: Option<Settings>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub delete_branch_on_merge: Option<bool>,
    pub squash_merge: Option<bool>,
    pub merge_commit: Option<bool>,
    pub rebase_merge: Option<bool>,
    pub auto_merge: Option<bool>,
    pub update_branch: Option<bool>,
    pub issues: Option<bool>,
    pub projects: Option<bool>,
    pub wiki: Option<bool>,
    pub discussions: Option<bool>,
}

pub fn load() -> Result<Config> {
    let path = path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

fn path() -> PathBuf {
    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config_home).join("ghstd/config.toml");
    }
    let home = env::var_os("HOME").unwrap_or_else(|| ".".into());
    PathBuf::from(home).join(".config/ghstd/config.toml")
}

impl Settings {
    pub fn merge(&mut self, other: &Settings) {
        if other.delete_branch_on_merge.is_some() {
            self.delete_branch_on_merge = other.delete_branch_on_merge;
        }
        if other.squash_merge.is_some() {
            self.squash_merge = other.squash_merge;
        }
        if other.merge_commit.is_some() {
            self.merge_commit = other.merge_commit;
        }
        if other.rebase_merge.is_some() {
            self.rebase_merge = other.rebase_merge;
        }
        if other.auto_merge.is_some() {
            self.auto_merge = other.auto_merge;
        }
        if other.update_branch.is_some() {
            self.update_branch = other.update_branch;
        }
        if other.issues.is_some() {
            self.issues = other.issues;
        }
        if other.projects.is_some() {
            self.projects = other.projects;
        }
        if other.wiki.is_some() {
            self.wiki = other.wiki;
        }
        if other.discussions.is_some() {
            self.discussions = other.discussions;
        }
    }
}
