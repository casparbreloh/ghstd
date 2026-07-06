# ghstd

Small CLI for applying Caspar's standard GitHub repository settings.

```sh
ghstd status casparbreloh/bootstrap
ghstd status --all
ghstd apply casparbreloh/bootstrap
ghstd create my-repo --private
```

`--all` scans non-archived repositories owned by the authenticated GitHub user.

The standard is intentionally small:

- delete branches on merge
- enable squash merge
- disable merge commits
- disable rebase merge
- enable auto-merge
- suggest updating pull request branches
- disable issues
- disable projects
- disable wiki
- disable discussions

`ghstd` shells out to the GitHub CLI, so authentication and permissions come
from `gh`.
