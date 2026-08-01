# MCP and Agents

Sensez is designed to sit in the middle of an edit loop, not just at the end of
CI. The MCP server is the integration point that lets a coding agent scan
repeatedly during a session.

## Setup

There is nothing to run by hand. `sensez init` registers the server with your
coding agent — Claude Code, Cursor, Cline, Codex, OpenCode, and Pi are
supported — and the agent launches it automatically at startup. See
[Getting Started](../getting-started.md).

## Tools

- `noze_sniff` — scan the repository for duplication, dead code, cycles,
  boundary violations, and smells. Takes an absolute repository root as `path`.
  Results are diff-focused by default (findings touched by uncommitted
  changes, computed over the full graph); pass `diff=false` for a full audit
  and `limit` to cap findings per pillar.
- `noze_gate` — experimental end-of-turn diff gate, intended for hook
  integration rather than direct calls. May be noisy on short or Q&A turns.
- `noze_explain` — explain a finding category in plain English.
- `get_configuration_summary` — summarize the effective configuration and the
  noisiest rules, as a starting point for tuning.
- `brainz_report` — summarize local-only usage and resolution metrics.
- `brainz_triage` — record the user's debt or false-positive verdict on a
  finding.

## Use In A Session

The important habit is to keep feedback close to the edit:

1. Run a diff-scoped scan when the change is still fresh.
2. Surface only the findings that matter for the current turn.
3. Record debt or false positives when a finding is real but intentionally not
   being fixed now.

## Pair With CI

Use the MCP workflow for immediacy, and keep CI for enforcement. The agent
loop should be fast and conversational; CI should be the slower, stricter back
stop.
