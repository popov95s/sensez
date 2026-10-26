# CLI Reference

This page mirrors `src/cli/spec.rs`.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Clean: the scan ran and no gate tripped |
| `1` | Gate tripped: a finding met `--fail-on` / `--fail-on-new` |
| `2` | Error: bad arguments, no usable diff for `--diff` / `--diff-from` / `--fail-on-new`, or an incomplete scan under `--fail-on` / `--fail-on-new` (an analyzer such as `[duplication.semantic]` could not run) |

`--diff` and `--diff-from` never fall back to a full scan: outside a git work
tree, or with an unreadable diff file, they exit `2`. `--fail-on` without a
diff flag works outside git.

## `sensez --help`

```text
Sensez — the structural maintainability layer for your codebase.

Finds the cross-file problems linters and type-checkers can't see: duplication, dead code, import cycles, layering/boundary violations, and design smells. Opinionated guardrails that keep a codebase coherent and maintainable as it grows.

Run it alongside your linter and type-checker (e.g. Ruff/ty for Python, ESLint/tsc for JS/TS), not instead of them.

Usage: sensez [OPTIONS] [PATH] [COMMAND]

Commands:
  noze    Code smell and structure checks. Defaults to scanning the given path
  reflexez  Run only tests affected by current changes, using the existing test runner
  init    Set up a repository, or offer a global agent install outside Git
  mcp     MCP server commands
  brainz  Local metrics and value reports
  eyez    Docs/comment search commands
  help    Print this message or the help of the given subcommand(s)

Arguments:
  [PATH]
          Root directory of the target project when no subcommand is supplied

Options:
      --threshold <THRESHOLD>
          Minimum structural-token run length for the duplication detector (0 = off)

      --summary
          Emit aggregated per-rule JSON for configuration tuning

      --json
          Emit machine-readable JSON instead of a human-readable report

      --max <MAX>
          Cap each pillar to its top-N ranked findings; smells are capped per kind (0 = unlimited)

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

      --fail-on [<LEVEL>]
          Exit non-zero if any finding meets or exceeds the given action level (must_fix | warning | advisory | info). Defaults to `must_fix` when the flag is present without a value

      --fail-on-new [<LEVEL>]
          Exit non-zero if diff-scoped findings meet or exceed the given action level (must_fix | warning | advisory | info). Defaults to `must_fix` when the flag is present without a value

      --explain
          Append a plain-English legend defining each finding category shown

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## `sensez reflexez --help`

```text
Run only tests affected by current changes, using the existing test runner

Usage: sensez reflexez [OPTIONS] [PATH] [-- <RUNNER_ARGS>...]

Arguments:
  [PATH]            Repository root (default: current directory)
  [RUNNER_ARGS]...  Arguments passed unchanged to the selected test runner(s)

Options:
      --base <BASE>          Also include committed changes since the merge-base with this Git revision
      --staged               Consider staged changes only
      --changed-file <PATH>  Analyze this changed path instead of consulting Git. Repeatable
      --plan                 Print the test plan without executing it
      --json                 Emit the plan as machine-readable JSON; implies --plan
      --full                 Run each runner once without file arguments, so the runner discovers every test
      --strict-dynamic       Run the full suite on any opaque dynamic import or unreadable runner-config key
      --runner <RUNNER>      Force one runner instead of auto-detecting it [default: auto] [possible values: auto, pytest, vitest, jest]
  -h, --help                 Print help
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
          Minimum structural-token run length for the duplication detector (0 = off)

      --summary
          Emit aggregated per-rule JSON for configuration tuning

      --json
          Emit machine-readable JSON instead of a human-readable report

      --max <MAX>
          Cap each pillar to its top-N ranked findings; smells are capped per kind (0 = unlimited)

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

      --fail-on [<LEVEL>]
          Exit non-zero if any finding meets or exceeds the given action level (must_fix | warning | advisory | info). Defaults to `must_fix` when the flag is present without a value

      --fail-on-new [<LEVEL>]
          Exit non-zero if diff-scoped findings meet or exceed the given action level (must_fix | warning | advisory | info). Defaults to `must_fix` when the flag is present without a value

      --explain
          Append a plain-English legend defining each finding category shown

  -h, --help
          Print help

Examples:
  sensez noze . --diff --fail-on-new
  sensez noze . --diff --fail-on-new warning
  sensez noze . --diff --fail-on-new must_fix
```

### Output limits and gates

- By default each pillar shows its top 5 findings and smells show their top 3
  per kind. `--max N` sets that cap for every pillar and every smell kind, so
  `--smells --max 1` prints one finding per smell kind. `--all` prints every
  finding and cannot be combined with `--max`.
- `--fail-on` and `--fail-on-new` see the findings this run selected: pillar
  flags such as `--dead-code` and `--output-glob` filter the gate too, and
  low-confidence dead code never trips it. The display cap does not: a
  finding hidden by `--max` still fails the gate.
- Pillar flags leave the other pillars out of the report: the terminal omits
  their sections, and JSON lists them in `meta.filtered_pillars`, so a `0`
  total there means "not asked", not "clean".
- `--summary` counts the same selected findings: it honours `--diff`,
  `--diff-from`, `--fail-on-new`, the pillar flags and `--output-glob`, and
  its exit code follows `--fail-on` / `--fail-on-new` like a normal run.
- `--output-glob '!PATTERN'` excludes matching files (quote it for the shell).
  Includes and excludes combine: a finding is kept when its file matches an
  include (or no include is given) and no exclude. A clone group drops only its
  excluded occurrences and stays while two or more remain. `!` alone or `!!…`
  is rejected (exit `2`).
- `--fail-on-new` implies `--diff` and says so on stderr. In diff mode the
  terminal report starts with "diff mode: only findings touching changed
  lines".
- A level that is not `must_fix`, `warning`, `advisory` or `info` is rejected.
  `--fail-on` takes the next word as its level, so write `sensez noze . --fail-on`
  or `--fail-on=must_fix .`, not `--fail-on .`.

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
Set up a repository, or offer a global agent install outside Git

Usage: init [OPTIONS] [PATH]

Arguments:
  [PATH]
          Repository root (default: current directory)

Options:
      --agent <AGENT>
          Coding agent: claude-code | cursor | cline | codex | opencode | pi | other

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

Usage: sensez eyez search [OPTIONS] [PATH] <QUERY>

Arguments:
  <PATH> <QUERY>...
          Project root (default: current directory), then a natural-language intent query

Options:
      --top-k <TOP_K>
          Number of results to return (at least 1)
          
          [default: 10]

      --json
          Emit machine-readable JSON instead of a human-readable list

  -h, --help
          Print help
```

## `sensez eyez reindex --help`

```text
Refresh eyez caches for the repository

Usage: reindex [OPTIONS] [PATH]

Arguments:
  [PATH]
          Root directory of the target project (default: current directory)

Options:
      --force
          Delete existing eyez caches before rebuilding

      --semantic
          Also warm semantic-duplication bundles when configured

  -h, --help
          Print help
```

### Embedding model

`eyez search`, `eyez reindex` and `[duplication.semantic]` use the
`minishlab/potion-code-16M-v2` model at a pinned revision. It is looked up,
in order:

1. `SENSEZ_EYEZ_MODEL_DIR`: a directory holding `config.json`,
   `tokenizer.json` and `model.safetensors`. Nothing is downloaded.
2. The HuggingFace cache: `$HF_HOME/hub`, else `~/.cache/huggingface/hub`.
3. A download (about 35 MB) into that cache. Sensez prints one line to
   stderr before it starts; `HF_ENDPOINT` changes the host.

With `HF_HUB_OFFLINE=1` or `SENSEZ_OFFLINE=1`, step 3 is skipped and a missing
model is an error that names the cache it checked.
