# Sensez GitHub Action

Run Sensez in pull requests and mark structural duplication exactly where it
appears. By default the action emits non-blocking GitHub annotations. Set
`with-comments: true` to add inline pull-request review comments.

## Usage

```yaml
name: Sensez

on:
  pull_request:

permissions:
  contents: read
  pull-requests: write

jobs:
  sensez:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: popov95s/sensez@v1
        with:
          with-comments: false
```

## Blocking Mode

```yaml
- uses: popov95s/sensez@v1
  with:
    with-comments: true
    fail-on-new: must_fix
```

## Inputs

| Input | Default | Description |
|---|---:|---|
| `path` | `.` | Repository-relative path to scan. |
| `version` | `latest` | Sensez PyPI version to run. |
| `threshold` | | Optional duplication token threshold. |
| `with-comments` | `false` | Add inline PR review comments. |
| `fail-on-new` | | Fail when duplication meets this action level. |
| `level` | `warning` | Annotation level: `notice`, `warning`, or `error`. |

`pull-requests: write` is only required when `with-comments: true`. The action
uses `uv` to run both the Python wrapper and the published Sensez CLI.
