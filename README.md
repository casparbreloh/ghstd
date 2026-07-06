# ghstd

Small CLI for checking and applying configured GitHub repository settings.

```sh
ghstd status owner/repo
ghstd status --all
ghstd apply owner/repo
ghstd create my-repo --private
```

`ghstd` shells out to the GitHub CLI, so authentication and permissions come
from `gh`.

## Configuration

Configuration is read from:

```text
$XDG_CONFIG_HOME/ghstd/config.toml
```

or, when `XDG_CONFIG_HOME` is unset:

```text
~/.config/ghstd/config.toml
```

Example:

```toml
[standard]
auto_merge = true
delete_branch_on_merge = true
discussions = false
issues = false
merge_commit = false
projects = false
rebase_merge = false
squash_merge = true
squash_merge_message = "PR_BODY"
squash_merge_title = "PR_TITLE"
update_branch = true
wiki = false

[private]
discussions = true

[public]
issues = true
```

`[standard]` applies to every repository. `[private]` and `[public]` override
only the settings they specify. Omitted settings are ignored.

Supported settings:

- `auto_merge`
- `delete_branch_on_merge`
- `discussions`
- `issues`
- `merge_commit`
- `projects`
- `rebase_merge`
- `squash_merge`
- `squash_merge_message`
- `squash_merge_title`
- `update_branch`
- `wiki`

`status --all` scans non-archived repositories owned by the authenticated
GitHub user.
