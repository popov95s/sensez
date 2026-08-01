# Getting Started

Sensez is meant to run inside an agent loop so it can catch smells before they
reach a pull request. The fastest path is to keep it close to the edit: use it inside your coding agent loop to provide it quick feedback on its work, as it's getting developed.

## 1. Install

=== "Python"

    ```bash
    # As a project dev dependency
    uv add --dev sensez

    # Or as a global CLI
    uv tool install sensez
    ```

=== "JS / TS"

    ```bash
    npm install --save-dev sensez
    ```

## 2. Initialize the repository

=== "Python"

    ```bash
    uv run sensez init
    ```

=== "JS / TS"

    ```bash
    npx sensez init
    ```

`sensez init` setup guides you through the install, after you select which coding agent to use it with. Then:

- writes a commented starter config — `sensez.toml`, or a `[tool.sensez]`
  section in `pyproject.toml` if you prefer;
- registers the Sensez MCP server with your agent, so the agent launches
  Sensez automatically at startup;
- installs the Sensez skill that teaches the agent when and how to scan;
- creates `.sensez/` for local metrics and caches, and adds it to `.gitignore`, if local metrics are enabled;
- optionally installs an experimental end-of-turn gate hook (Claude Code only).

What `init` writes for each supported agent:

| Agent | MCP config | Agent skill | Gate hook |
| --- | --- | --- | --- |
| Claude Code | `.mcp.json` | `.claude/skills/sensez` | Optional, in `.claude/settings.json` |
| Cursor | `.cursor/mcp.json` | — | — |
| Cline | `.cline/mcp.json` | `.cline/skills/sensez` | — |
| Codex | `.codex/config.toml` | `.codex/skills/sensez` | — |
| OpenCode | `opencode.jsonc` | `.opencode/skills/sensez` | — |
| Pi | `.pi/mcp.json` | `.pi/skills/sensez` | — |
| Other / none | — (init prints guidance instead) | — | — |

## 3. Restart your agent

Agents load MCP servers at startup. Restart the agent (or reload the window)
after `init`, or the sensez tools will not appear.

## 4. Tune it

Everything Sensez does is configured in the file `init` wrote. Continue with
the [Configuration Reference](reference/configuration.md) for action levels,
per-language overrides, boundary rules, and accepting findings, and the
[Finding Reference](reference/findings.md) for what each detector does.

## Next

- [CLI Reference](reference/cli.md) for every command and flag.
- [MCP and Agents](usage/mcp-and-agents.md) for the interactive workflow.
- [GitHub Action](usage/github-action.md) for pull-request feedback.
