use crate::{
    config::{Config, Settings},
    github::Repo,
};

#[derive(Debug)]
pub struct Rule {
    current_text: String,
    desired_text: String,
    edit: Edit,
}

struct RuleSpec {
    enable_flag: &'static str,
    disable_flag: &'static str,
    enabled_text: &'static str,
    disabled_text: &'static str,
}

#[derive(Debug)]
pub enum Edit {
    Flag(String),
    Patch { field: String, value: String },
}

pub fn drift(repo: &Repo, config: &Config) -> Vec<Rule> {
    rules(repo, config)
        .into_iter()
        .filter(|rule| !rule.matches())
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
            enable_flag: "--enable-discussions",
            disable_flag: "--enable-discussions=false",
            enabled_text: "discussions enabled",
            disabled_text: "discussions disabled",
        },
    );
    push_text_rule(
        &mut rules,
        settings.squash_merge_message,
        &repo.squash_merge_commit_message,
        "squash merge message",
        "squash_merge_commit_message",
    );
    push_text_rule(
        &mut rules,
        settings.squash_merge_title,
        &repo.squash_merge_commit_title,
        "squash merge title",
        "squash_merge_commit_title",
    );
    rules
}

fn push_rule(rules: &mut Vec<Rule>, desired: Option<bool>, current: bool, spec: RuleSpec) {
    let Some(desired) = desired else {
        return;
    };
    rules.push(Rule {
        current_text: if current {
            spec.enabled_text.to_string()
        } else {
            spec.disabled_text.to_string()
        },
        desired_text: if desired {
            spec.enabled_text.to_string()
        } else {
            spec.disabled_text.to_string()
        },
        edit: Edit::Flag(if desired {
            spec.enable_flag.to_string()
        } else {
            spec.disable_flag.to_string()
        }),
    });
}

fn push_text_rule(
    rules: &mut Vec<Rule>,
    desired: Option<String>,
    current: &str,
    label: &'static str,
    field: &'static str,
) {
    let Some(desired) = desired else {
        return;
    };
    rules.push(Rule {
        current_text: format!("{label} {current}"),
        desired_text: format!("{label} {desired}"),
        edit: Edit::Patch {
            field: field.to_string(),
            value: desired,
        },
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
    pub fn matches(&self) -> bool {
        self.current_text == self.desired_text
    }

    pub fn edit(&self) -> &Edit {
        &self.edit
    }
}
