# Sensez for Visual Studio Code

Sensez reports structural duplication, dead-code candidates, import cycles,
boundary violations, and design smells directly in supported source files.

The extension starts `sensez server stdio`. `npm run package` stages the
matching release executable in `bundled/<platform>-<arch>/sensez`; build the
binary first with `cargo build --release --no-default-features --features
all-langs,lsp`. During development set `sensez.path` to a local build in a
trusted workspace.

See [RELEASING.md](RELEASING.md) for local VSIX packaging and the manual
Marketplace workflow.

Use **Sensez: Rescan Workspace** after changing configuration or when an
immediate full scan is desired. Diagnostics are refreshed after saves.
