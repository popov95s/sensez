# GitHub Action

The GitHub Action is the PR-facing wrapper for Sensez's feedback.
It is useful when you want structural review comments or annotations without
requiring every contributor to install Sensez locally.

The canonical action docs live in [`action/README.md`](../../action/README.md),
and the exact inputs are defined in [`action.yml`](../../action.yml).

## What It Does

- Scans the repository path you pass in `path`.
- Emits non-blocking annotations by default.
- Adds inline review comments when `with-comments: true`.
- Fails the job when `fail-on-new` is set and the findings meet that action
  level.
- Uses the `level` input to choose the annotation severity.

## Minimal Usage

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

## Notes

- `pull-requests: write` is only needed when `with-comments: true`.
- The action is currently duplication-focused, so it is best used as the PR
  wrapper around the `noze` workflow rather than as the full docs surface for
  every pillar.
