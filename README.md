# ghstd

Small CLI for applying Caspar's standard GitHub repository settings.

```sh
ghstd status casparbreloh/bootstrap
ghstd status --all
ghstd apply casparbreloh/bootstrap
ghstd apply --all
ghstd create my-repo --private
```

The standard is intentionally small:

- delete branches on merge
- enable squash merge
- disable merge commits
- disable rebase merge
- enable auto-merge

`ghstd` shells out to the GitHub CLI, so authentication and permissions come
from `gh`.
