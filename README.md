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
delete_branch_on_merge = true
squash_merge = true
merge_commit = false
rebase_merge = false
auto_merge = true
update_branch = true

[private]
issues = false
projects = false
wiki = false
discussions = false

[public]
issues = true
projects = false
wiki = false
discussions = false
```

`[standard]` applies to every repository. `[private]` and `[public]` override
only the settings they specify. Omitted settings are ignored.

Supported settings:

- `delete_branch_on_merge`
- `squash_merge`
- `merge_commit`
- `rebase_merge`
- `auto_merge`
- `update_branch`
- `issues`
- `projects`
- `wiki`
- `discussions`

`status --all` scans non-archived repositories owned by the authenticated
GitHub user.
