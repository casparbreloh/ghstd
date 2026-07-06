use crate::{
    config::{Config, Settings},
    github::Repo,
};

#[derive(Debug)]
pub struct Rule {
    label: &'static str,
    current: bool,
    desired: bool,
    enable_flag: &'static str,
    disable_flag: &'static str,
    enabled_text: &'static str,
    disabled_text: &'static str,
}

pub fn drift(repo: &Repo, config: &Config) -> Vec<Rule> {
    rules(repo, config)
        .into_iter()
        .filter(|rule| rule.current != rule.desired)
        .collect()
}

pub fn rules(repo: &Repo, config: &Config) -> Vec<Rule> {
    let settings = settings_for(repo, config);
    let mut rules = Vec::new();
    push_rule(
        &mut rules,
        settings.delete_branch_on_merge,
        "delete branches on merge",
        repo.delete_branch_on_merge,
        "--delete-branch-on-merge",
        "--delete-branch-on-merge=false",
        "delete branches on merge",
        "keep branches after merge",
    );
    push_rule(
        &mut rules,
        settings.squash_merge,
        "squash merge",
        repo.allow_squash_merge,
        "--enable-squash-merge",
        "--enable-squash-merge=false",
        "squash merge enabled",
        "squash merge disabled",
    );
    push_rule(
        &mut rules,
        settings.merge_commit,
        "merge commits",
        repo.allow_merge_commit,
        "--enable-merge-commit",
        "--enable-merge-commit=false",
        "merge commits enabled",
        "merge commits disabled",
    );
    push_rule(
        &mut rules,
        settings.rebase_merge,
        "rebase merge",
        repo.allow_rebase_merge,
        "--enable-rebase-merge",
        "--enable-rebase-merge=false",
        "rebase merge enabled",
        "rebase merge disabled",
    );
    push_rule(
        &mut rules,
        settings.auto_merge,
        "auto-merge",
        repo.allow_auto_merge,
        "--enable-auto-merge",
        "--enable-auto-merge=false",
        "auto-merge enabled",
        "auto-merge disabled",
    );
    push_rule(
        &mut rules,
        settings.update_branch,
        "update branch suggestions",
        repo.allow_update_branch,
        "--allow-update-branch",
        "--allow-update-branch=false",
        "update branch suggestions enabled",
        "update branch suggestions disabled",
    );
    push_rule(
        &mut rules,
        settings.issues,
        "issues",
        repo.has_issues,
        "--enable-issues",
        "--enable-issues=false",
        "issues enabled",
        "issues disabled",
    );
    push_rule(
        &mut rules,
        settings.projects,
        "projects",
        repo.has_projects,
        "--enable-projects",
        "--enable-projects=false",
        "projects enabled",
        "projects disabled",
    );
    push_rule(
        &mut rules,
        settings.wiki,
        "wiki",
        repo.has_wiki,
        "--enable-wiki",
        "--enable-wiki=false",
        "wiki enabled",
        "wiki disabled",
    );
    push_rule(
        &mut rules,
        settings.discussions,
        "discussions",
        repo.has_discussions,
        "--enable-discussions",
        "--enable-discussions=false",
        "discussions enabled",
        "discussions disabled",
    );
    rules
}

fn push_rule(
    rules: &mut Vec<Rule>,
    desired: Option<bool>,
    label: &'static str,
    current: bool,
    enable_flag: &'static str,
    disable_flag: &'static str,
    enabled_text: &'static str,
    disabled_text: &'static str,
) {
    let Some(desired) = desired else {
        return;
    };
    rules.push(Rule {
        label,
        current,
        desired,
        enable_flag,
        disable_flag,
        enabled_text,
        disabled_text,
    });
}

fn settings_for(repo: &Repo, config: &Config) -> Settings {
    let mut settings = Settings::default();
    if let Some(base) = &config.standard {
        settings.merge(base);
    }
    if repo.private {
        if let Some(private) = &config.private {
            settings.merge(private);
        }
    } else if let Some(public) = &config.public {
        settings.merge(public);
    }
    settings
}

impl Rule {
    pub fn current_text(&self) -> &'static str {
        if self.current {
            self.enabled_text
        } else {
            self.disabled_text
        }
    }

    pub fn changed_text(&self) -> String {
        let state = if self.desired { "enabled" } else { "disabled" };
        format!("{} {state}", self.label)
    }

    pub fn flag(&self) -> String {
        if self.desired {
            self.enable_flag.to_string()
        } else {
            self.disable_flag.to_string()
        }
    }

    pub fn matches(&self) -> bool {
        self.current == self.desired
    }
}
