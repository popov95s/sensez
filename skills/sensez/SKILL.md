---
name: sensez
description: >-
  Use when checking a codebase for structural duplication, dead code candidates,
  circular imports, boundary violations, or design smells with the `sensez` CLI
  or MCP server. Use after edit turns to verify that newly written code is
  structurally correct. You can also run it on user triggers: "find duplicate code", "audit this
  project", "check dead code", "detect cycles", "enforce boundaries", "run
  Sensez", or "check this change". Not for per-file lint/type issues; use the
  language's linter and type-checker for those.
---

# Sensez

Sensez finds project-level maintainability problems that linters and
type-checkers usually cannot see across files. It currently has profiles for
Python, JavaScript, TypeScript/TSX, and Rust:

| Area | Output key | What to do with it |
|---|---|---|
| Duplication | `duplication` | Prefer reuse/extraction when the duplicated block is real code, not tests/fixtures. |
| Cycles | `cycles` | Treat as load-time architecture risk; break one import edge. |
| Boundaries | `boundaries` | Enforce the rule in `sensez.toml`; do not ignore unless the rule is wrong. |
| Dead code | `dead_code` | High confidence is actionable; Medium/Low need review. |
| Smells | `smells` | Advisory design pressure; lead with high-impact findings. |

## Build

From the repo root:

```bash
cargo build --release
cargo build --release --features mcp,eyez,all-langs
```

The binary is `./target/release/sensez`. For one-off local runs:

```bash
cargo run --release --bin sensez -- noze <path> --json
```

## CLI

Use JSON for anything an agent or script will parse:

```bash
sensez noze <path> --json
sensez noze <path> --max 20 --json
sensez noze <path> --threshold 60 --json
```

Diff-scoped mode is the right default inside an edit loop:

```bash
sensez noze . --diff --json
git diff HEAD | sensez noze . --diff-from - --json
sensez noze . --diff --fail-on-new
sensez noze . --diff --fail-on-new must_fix
```

`--max N` caps each pillar's returned findings while preserving true totals in
`meta.*_total`. Use it to keep agent payloads small.

`--fail-on-new LEVEL` exits non-zero when a diff-scoped finding meets or exceeds
the configured action level (`must_fix`, `warning`, `advisory`, or `info`). The
default when the flag is present without a value is `must_fix`.

## Reading Results

Important `meta` fields:

- `mode`: `full` or `diff`.
- `*_total`: true counts before `--max` truncation.
- `boundaries_configured`: `false` means boundaries were not checked.
- `external_edges` vs `internal_edges`: a very high external ratio may mean roots/imports were not resolved well.

Dead-code confidence:

- `High`: module is reachable and the symbol is not referenced. Usually safe to remove after a quick domain check.
- `Medium`: module is imported plainly; attribute access may hide usage.
- `Low`: module itself is not imported. Treat as possible script/framework entrypoint.

Do not present Medium/Low as guaranteed deletion work. Phrase them as review
candidates.

## Configuration

Sensez reads `sensez.toml`, or `[tool.sensez]` in `pyproject.toml` when a
project already centralizes tool config there.

Common knobs:

```toml
roots = []
exclude = []

[duplication]
threshold = 50

[dead_code]
entrypoints = ["route", "fixture", "task", "command", "app", "cli"]
entrypoint_names = ["register", "main", "setup"]
entry_points = []
unused_imports = false
unused_methods = false
unused_variables = false

[[boundaries.forbidden]]
from = "app.domain"
to = "app.web"
```

Prefer adding framework entrypoints or boundary rules to config over suppressing
individual findings in source.

## MCP

Build with MCP support, then run:

```bash
sensez mcp serve
```

The server speaks JSON-RPC over stdio. Use it when an agent will call Sensez
repeatedly; use `sensez noze --json` for one-shot checks.

Useful tools:

- `noze_sniff`: returns the same JSON report as the CLI.
- `eyez_search_docs`: semantic search over docstrings/comments when built with `eyez`.
- `brainz_report`: local-only session value metrics.
- `brainz_triage`: record user decisions for debt/false positives.
- `noze_gate`: end-of-turn diff gate for agent hooks.

## Agent Guidance

After each edit turn, run `sensez noze . --diff --json` to check that the code
just written is structurally sound. If acting as a gate, use
`sensez noze . --diff --fail-on-new must_fix` so only configured must-fix
findings block progress. Report only findings that touch the current change
unless the user asked for a full audit.

Lead with reliable signals: boundary violations, cycles, high-confidence dead
code, and large duplication classes. For smells, explain why the finding matters
in this codebase before recommending a refactor.

Do not use Sensez for unused local variables, per-file unused imports, formatting,
or type errors. Those belong to tools like ruff, ESLint, TypeScript, ty, mypy, or
the language compiler.
