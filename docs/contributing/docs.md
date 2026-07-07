# Maintaining Docs

The docs are designed to auto-generate through the code.

## Source Of Truth

- `src/cli/spec.rs` for CLI commands and flags.
- `src/noze/glossary.rs` for `--explain` wording and docs-only metadata.
- `docs/examples/smells/` for editable Python and TypeScript examples.
- `sensez.toml` for the commented configuration example.
- `action/README.md` and `action.yml` for GitHub Action behavior.
- `docs/local-metrics.md` for the metrics model.

## Refresh Flow

Regenerate generated reference pages after changing glossary metadata:

```bash
just docs-generate
```

To preview the rendered site:

```bash
just docs-serve
```

## Versioned Publishing

The public site is versioned with `mike`.

- Pushes to `main`/`master` publish the moving `dev` docs version.
- Full releases publish the Cargo/PyPI release version, for example `0.2.0`.
- The `latest` alias is updated to point at the newest full release.

To build a local versioned copy without pushing:

```bash
just docs-version 0.2.0
just docs-version-serve
```

Then review the diff and keep the prose pages short. If a page starts to repeat
what the code already says, it should usually be rewritten to explain how to use
the feature instead of describing it again.

## Style Rules

- Prefer task-oriented pages over long conceptual essays.
- Put one idea on a page unless the topics are naturally inseparable.
- Keep code snippets runnable and concrete.
- Link to the source page for anything that is generated or mechanically
  derived.
