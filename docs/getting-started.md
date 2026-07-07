# Getting Started

Sensez is meant to run inside an agent loop so it can catch smells before they
reach a pull request. The fastest path is to keep it close to the edit: use it inside your coding agent loop to provide it quick feedback on its work, as it's getting developed.

## Initialization

=== "Python"

    If your repo is Python-first, install Sensez with `uv` and initialize the
    repo config.

    ```bash
    uv add --dev sensez
    uv run sensez init
    ```
    or if you prefer to install it globally as a tool:
    
    ```bash
    uv tool install sensez
    sensez init
    ```

    The setup will guide you through the basics of using the tool. After setup, **you will need to restart your coding agent** for the MCP to be picked up.
    
    For one off checks using the CLI, you can use:
    ```bash
    uvx sensez noze .
    ```

=== "JS / TS"

    If your repo is JavaScript or TypeScript-first, install Sensez with npm and
    initialize the repo config.

    ```bash
    npm install --save-dev sensez
    npx sensez init .
    ```
    
    The setup will guide you through the basics of using the tool. After setup, **you will need to restart your coding agent** for the MCP to be picked up.
    
    ```bash
    npx sensez noze .
    ```


## Next

- [CLI Reference](reference/cli.md) for every command and flag.
- [Finding Reference](reference/findings.md) for the plain-English meaning of
  each pillar and smell kind.
- [Configuration Reference](reference/configuration.md) for `sensez.toml` and
  `[tool.sensez]`.
- [MCP and Agents](usage/mcp-and-agents.md) for interactive workflows.
