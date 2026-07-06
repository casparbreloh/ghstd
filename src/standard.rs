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

struct RuleSpec {
    label: &'static str,
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
        repo.delete_branch_on_merge,
        RuleSpec {
            label: "delete branches on merge",
            enable_flag: "--delete-branch-on-merge",
            disable_flag: "--delete-branch-on-merge=false",
            enabled_text: "delete branches on merge",
            disabled_text: "keep branches after merge",
        },
    );
    push_rule(
        &mut rules,
        settings.squash_merge,
        repo.allow_squash_merge,
        RuleSpec {
            label: "squash merge",
            enable_flag: "--enable-squash-merge",
            disable_flag: "--enable-squash-merge=false",
            enabled_text: "squash merge enabled",
            disabled_text: "squash merge disabled",
        },
    );
    push_rule(
        &mut rules,
        settings.merge_commit,
        repo.allow_merge_commit,
        RuleSpec {
            label: "merge commits",
            enable_flag: "--enable-merge-commit",
            disable_flag: "--enable-merge-commit=false",
            enabled_text: "merge commits enabled",
            disabled_text: "merge commits disabled",
        },
    );
    push_rule(
        &mut rules,
        settings.rebase_merge,
        repo.allow_rebase_merge,
        RuleSpec {
            label: "rebase merge",
            enable_flag: "--enable-rebase-merge",
            disable_flag: "--enable-rebase-merge=false",
            enabled_text: "rebase merge enabled",
            disabled_text: "rebase merge disabled",
        },
    );
    push_rule(
        &mut rules,
        settings.auto_merge,
        repo.allow_auto_merge,
        RuleSpec {
            label: "auto-merge",
            enable_flag: "--enable-auto-merge",
            disable_flag: "--enable-auto-merge=false",
            enabled_text: "auto-merge enabled",
            disabled_text: "auto-merge disabled",
        },
    );
    push_rule(
        &mut rules,
        settings.update_branch,
        repo.allow_update_branch,
        RuleSpec {
            label: "update branch suggestions",
            enable_flag: "--allow-update-branch",
            disable_flag: "--allow-update-branch=false",
            enabled_text: "update branch suggestions enabled",
            disabled_text: "update branch suggestions disabled",
        },
    );
    push_rule(
        &mut rules,
        settings.issues,
        repo.has_issues,
        RuleSpec {
            label: "issues",
            enable_flag: "--enable-issues",
            disable_flag: "--enable-issues=false",
            enabled_text: "issues enabled",
            disabled_text: "issues disabled",
        },
    );
    push_rule(
        &mut rules,
        settings.projects,
        repo.has_projects,
        RuleSpec {
            label: "projects",
            enable_flag: "--enable-projects",
            disable_flag: "--enable-projects=false",
            enabled_text: "projects enabled",
            disabled_text: "projects disabled",
        },
    );
    push_rule(
        &mut rules,
        settings.wiki,
        repo.has_wiki,
        RuleSpec {
            label: "wiki",
            enable_flag: "--enable-wiki",
            disable_flag: "--enable-wiki=false",
            enabled_text: "wiki enabled",
            disabled_text: "wiki disabled",
        },
    );
    push_rule(
        &mut rules,
        settings.discussions,
        repo.has_discussions,
        RuleSpec {
            label: "discussions",
            enable_flag: "--enable-discussions",
            disable_flag: "--enable-discussions=false",
            enabled_text: "discussions enabled",
            disabled_text: "discussions disabled",
        },
    );
    rules
}

fn push_rule(rules: &mut Vec<Rule>, desired: Option<bool>, current: bool, spec: RuleSpec) {
    let Some(desired) = desired else {
        return;
    };
    rules.push(Rule {
        label: spec.label,
        current,
        desired,
        enable_flag: spec.enable_flag,
        disable_flag: spec.disable_flag,
        enabled_text: spec.enabled_text,
        disabled_text: spec.disabled_text,
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

pub fn has_rules(repo: &Repo, config: &Config) -> bool {
    settings_for(repo, config).has_rules()
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
