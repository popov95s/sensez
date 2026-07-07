# CLI Reference

This page mirrors `src/cli/spec.rs`.

## `sensez --help`

```text
Sensez — the structural maintainability layer for your codebase.

Finds the cross-file problems linters and type-checkers can't see: duplication, dead code, import cycles, layering/boundary violations, and design smells. Opinionated guardrails that keep a codebase coherent and maintainable as it grows.

Run it alongside your linter and type-checker (e.g. Ruff/ty for Python, ESLint/tsc for JS/TS), not instead of them.

Usage: sensez [OPTIONS] [PATH] [COMMAND]

Commands:
  noze    Code smell and structure checks. Defaults to scanning the given path
  init    First-time setup for a repository
  mcp     MCP server commands
  brainz  Local metrics and value reports
  eyez    Docs/comment search commands
  help    Print this message or the help of the given subcommand(s)

Arguments:
  [PATH]
          Root directory of the target project when no subcommand is supplied

Options:
      --threshold <THRESHOLD>
          Minimum structural-token run length for the duplication detector

      --summary
          Emit aggregated per-rule JSON for configuration tuning

      --json
          Emit machine-readable JSON instead of a human-readable report

      --max <MAX>
          Cap each pillar to its top-N ranked findings (0 = unlimited)

      --all
          Print every finding instead of the default top offenders

      --duplicates
          Report only duplicate-code findings

      --dead-code
          Report only high-confidence dead-code findings by default

      --cycles
          Report only circular-import findings

      --boundaries
          Report only boundary violations

      --smells
          Report only design-smell findings

      --output-glob <GLOB>
          Keep only output findings whose source file matches GLOB. Repeatable

      --diff
          Keep only findings touching the working-tree diff vs HEAD (uses git)

      --diff-from <FILE>
          Keep only findings touching a unified diff read from FILE ("-" = stdin)

      --fail-on-new [<LEVEL>]
          Exit non-zero if diff-scoped findings meet or exceed the given action level. Defaults to `must_fix` when the flag is present without a value
          
          [possible values: must_fix, warning, advisory, info]

      --explain
          Append a plain-English legend defining each finding category shown

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## `sensez noze --help`

```text
Code smell and structure checks. Defaults to scanning the given path

Usage: noze [OPTIONS] [PATH]
       noze <COMMAND>

Commands:
  sniff    Explicit alias for `sensez noze [PATH]`
  explain  Explain a finding category in plain English
  help     Print this message or the help of the given subcommand(s)

Arguments:
  [PATH]
          Root directory of the target project (default: current directory)

Options:
      --threshold <THRESHOLD>
          Minimum structural-token run length for the duplication detector

      --summary
          Emit aggregated per-rule JSON for configuration tuning

      --json
          Emit machine-readable JSON instead of a human-readable report

      --max <MAX>
          Cap each pillar to its top-N ranked findings (0 = unlimited)

      --all
          Print every finding instead of the default top offenders

      --duplicates
          Report only duplicate-code findings

      --dead-code
          Report only high-confidence dead-code findings by default

      --cycles
          Report only circular-import findings

      --boundaries
          Report only boundary violations

      --smells
          Report only design-smell findings

      --output-glob <GLOB>
          Keep only output findings whose source file matches GLOB. Repeatable

      --diff
          Keep only findings touching the working-tree diff vs HEAD (uses git)

      --diff-from <FILE>
          Keep only findings touching a unified diff read from FILE ("-" = stdin)

      --fail-on-new [<LEVEL>]
          Exit non-zero if diff-scoped findings meet or exceed the given action level. Defaults to `must_fix` when the flag is present without a value
          
          [possible values: must_fix, warning, advisory, info]

      --explain
          Append a plain-English legend defining each finding category shown

  -h, --help
          Print help

Examples:
  sensez noze . --diff --fail-on-new
  sensez noze . --diff --fail-on-new warning
  sensez noze . --diff --fail-on-new must_fix
```

## `sensez noze explain --help`

```text
Explain a finding category in plain English

Usage: explain [TERM]

Arguments:
  [TERM]
          Pillar key or smell kind. Omit to list all

Options:
  -h, --help
          Print help
```

## `sensez init --help`

```text
First-time setup for a repository

Usage: init [OPTIONS] [PATH]

Arguments:
  [PATH]
          Repository root (default: current directory)

Options:
      --agent <AGENT>
          Coding agent to configure: claude-code | cursor | other

      --gate
          Install the Claude Code Stop-gate hook

      --no-metrics
          Disable local-only usage metrics

  -y, --yes
          Accept defaults for any question not answered by a flag

  -h, --help
          Print help
```

## `sensez mcp --help`

```text
MCP server commands

Usage: mcp <COMMAND>

Commands:
  serve  Run the MCP (Model Context Protocol) server over stdio
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

## `sensez mcp serve --help`

```text
Run the MCP (Model Context Protocol) server over stdio

Usage: serve

Options:
  -h, --help
          Print help
```

## `sensez brainz --help`

```text
Local metrics and value reports

Usage: brainz <COMMAND>

Commands:
  report  Display local-only metrics showing what Sensez helped fix
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

## `sensez brainz report --help`

```text
Display local-only metrics showing what Sensez helped fix

Usage: report [OPTIONS] [PATH]

Arguments:
  [PATH]
          Repository root (default: current directory)

Options:
      --json
          Emit the raw machine-readable metrics payload

  -h, --help
          Print help
```

## `sensez eyez --help`

```text
Docs/comment search commands

Usage: eyez <COMMAND>

Commands:
  search   Semantic search over docstrings + comments
  reindex  Refresh eyez caches for the repository
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

## `sensez eyez search --help`

```text
Semantic search over docstrings + comments

Usage: search [OPTIONS] <PATH> <QUERY>

Arguments:
  <PATH>
          Root directory of the target project

  <QUERY>
          Natural-language intent query

Options:
      --top-k <TOP_K>
          Number of results to return
          
          [default: 10]

      --json
          Emit machine-readable JSON instead of a human-readable list

  -h, --help
          Print help
```
