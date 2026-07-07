# MCP and Agents

Sensez is designed to sit in the middle of an edit loop, not just at the end of
CI. The MCP server is the main integration point when you want the tool to run
repeatedly during a session.

## Start MCP

```bash
sensez mcp serve
```

The server speaks stdio MCP and exposes the themed tools used by the agent
workflow:

- `noze_sniff`
- `noze_gate`
- `noze_explain`
- `brainz_report`
- `brainz_triage`

## Use In A Session

`noze_sniff` is the "scan now" tool. `noze_gate` is the end-of-turn diff gate.
`brainz_report` summarizes what the tool has helped fix, and `brainz_triage`
records the user's debt or false-positive verdict.

The important habit is to keep feedback close to the edit:

1. Run a diff-scoped scan when the change is still fresh.
2. Surface only the findings that matter for the current turn.
3. Record debt or false positives when a finding is real but intentionally not
   being fixed now.

## Pair With CI

Use the MCP workflow for immediacy, and keep CI for enforcement. The agent
loop should be fast and conversational; CI should be the slower, stricter back
stop.
