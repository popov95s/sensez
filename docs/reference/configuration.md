# Configuration Reference

Sensez reads configuration from `sensez.toml` at the repository root. If that
file is absent, it falls back to a `[tool.sensez]` table in `pyproject.toml`.
When both exist, `sensez.toml` wins.

The root `sensez.toml` in this repository is intentionally commented and serves
as the canonical example of the supported surface.

## Main Sections

- `[duplication]` controls clone thresholds and clone-only excludes.
- `[dead_code]` controls entrypoint heuristics, dynamic usage, and whether
  broader unused symbol classes are enabled.
- `[smells]` controls the smell families and per-language thresholds.
- `[boundaries]` defines forbidden import edges between layers.
- `[action]` configures how aggressively automated workflows react to findings.
- `[accept]` stores shared accepted findings.
- `[self_improvement]` controls local metrics and feedback recording.

## Practical Advice

- Keep the file small enough that a new contributor can scan it in one sitting.
- Prefer explicit thresholds and exclusions over broad global disablement.
- Treat the config as code: review changes to it the same way you would review a
  detector change.

## Related Pages

- [CLI Reference](cli.md)
- [Finding Reference](findings.md)
- [Local Metrics](../local-metrics.md)
