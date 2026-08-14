# Configuration Reference

Sensez reads `sensez.toml` at the repository root. If that file is absent, it
falls back to a `[tool.sensez]` table in `pyproject.toml`; when both exist,
`sensez.toml` wins.

`sensez init` writes a starter file with every supported knob commented at its
default — that generated file is the complete surface. This page explains the
concepts behind it and the common tuning workflows. The
[sensez.toml Sensez uses on itself](https://github.com/popov95s/sensez/blob/main/sensez.toml)
is a heavily commented real-world example.

## Action levels

Every finding carries an action level. From quietest to strictest:
`info` → `advisory` → `warning` → `must_fix`.

Action levels decide how automated workflows react:

- `sensez noze . --diff --fail-on-new [LEVEL]` exits non-zero when diff-scoped
  findings meet or exceed `LEVEL` (default `must_fix`) — this is the CI hook.
- Agents and the end-of-turn gate treat each pillar according to its level, so
  `must_fix` findings lead and `info` findings stay out of the way.

Pillar defaults: `boundaries` = `must_fix`, `cycles` = `warning`,
`duplication` = `advisory`, `dead_code` = `advisory`. Smells inherit the
severity of their detector. Override per pillar or per smell:

```toml
[action]
duplication = "warning"

[smells.rules.loose_typing]
action = "must_fix"
```

## Smell rules: base vs per-language

`[smells.rules.<kind>]` applies to every language.
`[smells.<lang>.rules.<kind>]` — where `<lang>` is `python`, `javascript`,
`typescript`, or `rust` — overrides the base for that language only. (The Rust
profile is dogfooding-only and not shipped in the published packages.)

Setting any knob implicitly enables the rule; use `enabled = false` for
explicit suppression.

```toml
# All languages: long functions warn past 80 lines.
[smells.rules.long_function]
max_lines = 80
action = "warning"

# ...except TypeScript, which allows 120.
[smells.typescript.rules.long_function]
max_lines = 120
```

Every rule's knobs and default enabled state are listed in the
[Finding Reference](findings.md).

## Dead code: teaching Sensez about your framework

Sensez ships no hardcoded framework names. Liveness is modeled from entry
points, and your lists are **merged with** the language-profile defaults — you
only add what is specific to your project:

```toml
[dead_code]
entrypoints = ["route", "fixture", "task"]   # decorator trailing names that register code
entrypoint_names = ["register", "main"]      # function/class names invoked dynamically
entrypoint_bases = ["AppConfig"]             # base classes loaded dynamically
entry_points = ["**/scripts/**"]             # file globs reached outside the import graph
```

If dead-code output feels incomplete, broader unused-symbol classes are
available as opt-ins (your generated config shows their defaults):

```toml
[dead_code]
unused_imports = true
unused_methods = true
unused_variables = true
```

Prefer adding entry points over suppressing individual findings — one line of
config fixes a whole class of false positives.

## Duplication

```toml
[duplication]
threshold = 50   # minimum structural-token run to report a clone; raise to quieten
max_gap = 10     # stitch clones across small consistent edits; 0 disables
exclude = ["**/tests/**", "**/migrations/**"]   # clone-only excludes (files stay in the graph)
```

Opt-in detectors for harder clones: `near_miss` (consistent renames),
`class_name_duplicates`, `class_property_overlap_min`, and
`[duplication.semantic]` (only in builds with the `eyez` feature).

## Boundaries

Each rule forbids any module matching `from` from importing any module matching
`to`:

```toml
[[boundaries.forbidden]]
from = "app.domain"
to = "app.api"
```

Matching, per endpoint:

- A plain name is a dotted prefix — `app.domain` matches `app.domain` and
  `app.domain.*`.
- A glob (`* ? [`) is matched against **both** the module name and the file
  path, so `from = "**/domain/**"` works even for namespace packages without
  `__init__.py`.
- `to` also matches the literal import target, so rules still fire when an
  import cannot be resolved to an in-repo module.

A rule whose `from` matches no module is reported under
`meta.unmatched_boundary_rules` in JSON output — a typo never looks like a
clean pass.

## Accepting findings: `[accept]`

The out-of-line alternative to `# noqa` — nothing is written into your source.
Keys are pillars or detector ids; values are substrings of the finding label as
shown in a scan. The file is committed, so accepted findings stop flagging for
every teammate and their agents:

```toml
[accept]
dead_code = ["legacy.compat::shim"]        # a specific accepted symbol
"smells/god_module" = ["app.registry"]     # a specific accepted detector hit
```

There are three ways to silence findings — pick by scope:

| Mechanism | Scope | Shared? | Use for |
| --- | --- | --- | --- |
| `exclude` globs | Whole files/dirs, per pillar or global | Yes (committed) | Generated code, fixtures, migrations |
| `[accept]` | Specific findings | Yes (committed) | Accepted debt the team agrees on |
| `brainz_triage` | Specific findings | No (local `.sensez/`) | Personal review verdicts |

## Global scope

```toml
roots = []                          # package roots; empty = auto-detect from project layout
exclude = ["**/generated/**"]       # excluded from every pillar
```

## Gate and local metrics

CLI scans are stateless and never persist parsed source facts. Long-lived MCP
and LSP processes reuse analysis state in memory for their own lifetime:

```toml
[cache]
# Enables process-local reuse in MCP/LSP only; CLI scans always remain stateless.
enabled = false
```

```toml
[gate]
repeat_limit = 2   # how often the gate re-reports the same finding before auto-deferring

[self_improvement]
enabled = false    # stops even the on-disk recording under .sensez/local-metrics/
```

## Recipes

### Adopting Sensez in an existing repository

1. Run `sensez noze . --summary` for aggregated per-rule counts to see what
   would be noisy.
2. Gate only on new work: run `sensez noze . --diff --fail-on-new` in CI (or
   use the [GitHub Action](../usage/github-action.md)), so the backlog does not
   block anyone.
3. Record the agreed backlog in `[accept]` as you review it.
4. Escalate action levels pillar by pillar as each one cleans up.

### Quieting a noisy detector

Prefer raising its threshold over disabling it, and scope the change to the
language that is noisy:

```toml
[smells.python.rules.god_module]
min_fan = 40
```

Use `enabled = false` only as a last resort — a disabled detector is invisible
in future scans.

### Fixing framework false positives

Add the framework's registration pattern to `[dead_code]` (above) instead of
suppressing findings one by one.

## Practical advice

- Keep the file small enough that a new contributor can scan it in one sitting.
- Prefer explicit thresholds and exclusions over broad global disablement.
- Treat the config as code: review changes to it the same way you would review
  a detector change.

## Related pages

- [CLI Reference](cli.md)
- [Finding Reference](findings.md)
- [Local Metrics](../local-metrics.md)
