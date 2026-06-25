---
name: sensez
description: >-
  Use when checking a codebase for structural duplication, dead code candidates,
  circular imports, boundary violations, or design smells through the Sensez MCP
  server. Use after edit turns to verify that newly written code is structurally
  correct. You can also run it on user triggers: "find duplicate code", "audit
  this project", "check dead code", "detect cycles", "enforce boundaries", "run
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

## MCP Server

Sensez is used by agents through its MCP server. Configure the MCP client to run:

```bash
sensez mcp serve
```

The server speaks JSON-RPC over stdio. Once connected, use the MCP tools for
Sensez work. Always pass an absolute repository root as `path` unless the user
explicitly asks for a partial scan.

## MCP Tools

- `noze_sniff`: analyze duplication, dead code, cycles, boundary violations, and
  smells. Results are diff-focused by default so they show findings touched by
  current uncommitted changes while still analyzing the full graph. Pass
  `diff=false` for an explicit full-repository audit.
- `get_configuration_summary`: use only when the user asks to tune or adjust
  Sensez thresholds/configuration. Summarize the noisiest rules and offer
  concrete config changes before editing config files.
- `noze_gate`: end-of-turn quality gate for agent hooks. It is intended for hook
  integration, not ordinary direct calls during conversation.
- `noze_explain`: define a pillar or smell kind in plain English. Use it when you
  need to explain a finding category without guessing.
- `brainz_report`: summarize local-only Sensez usage metrics. Use when the user
  asks how Sensez helped, or before a commit.
- `brainz_triage`: record the user's explicit verdict on a finding. Never call it
  unless the user classifies a finding as debt, false positive, or cleared.

## Running Scans

Call `noze_sniff` with `path` set to the absolute repository root. Omit `diff`
for the default diff-focused scan, or pass `diff=false` when the user asks for a
full audit. Use `limit` to cap each pillar's returned findings when context is
tight, and use `threshold` only when the user requests a different duplication
token threshold.

After each edit turn, call `noze_sniff` to check that the code just written is
structurally sound. Report only findings that touch the current change unless
the user asked for a full audit.

## Reading Results

Important `meta` fields:

- `mode`: `full` or `diff`.
- `*_total`: true counts before `limit` truncation.
- `boundaries_configured`: `false` means boundaries were not checked.
- `external_edges` vs `internal_edges`: a very high external ratio may mean roots/imports were not resolved well.

Dead-code confidence:

- `High`: module is reachable and the symbol is not referenced. Usually safe to remove after a quick domain check.
- `Medium`: module is imported plainly; attribute access may hide usage.
- `Low`: module itself is not imported. Treat as possible script/framework entrypoint.

Do not present Medium/Low as guaranteed deletion work. Phrase them as review
candidates.

## Agent Guidance

Lead with reliable signals: boundary violations, cycles, high-confidence dead
code, and large duplication classes. For smells, explain why the finding matters
in this codebase before recommending a refactor.

Prefer adding framework entrypoints or boundary rules to Sensez configuration
over suppressing individual findings in source. Use `get_configuration_summary`
when the user explicitly asks to tune noisy rules.

Do not use Sensez for unused local variables, per-file unused imports, formatting,
or type errors. Those belong to tools like ruff, ESLint, TypeScript, ty, mypy, or
the language compiler.
