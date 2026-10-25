# Configuration Reference

Sensez reads `sensez.toml` at the repository root. If that file is absent, it
falls back to a `[tool.sensez]` table in `pyproject.toml`; when both exist,
`sensez.toml` wins.

`sensez init` writes a starter file with every supported knob commented at its
default — that generated file is the complete surface. This page explains the
concepts behind it and the common tuning workflows. The
[sensez.toml Sensez uses on itself](https://github.com/popov95s/sensez/blob/main/sensez.toml)
is a heavily commented real-world example.

## Validation

Sensez checks the whole file before it scans. Every problem is printed as
`sensez.toml:LINE:COL: message`, with a "did you mean" hint where one fits.

- **Errors** make Sensez ignore the whole file and scan with the default
  configuration: TOML syntax errors, wrong value types, invalid values such as
  `cycles = "critical"`, and unknown keys inside `[smells]` (including
  `[smells.rules.*]` and the language tables) and `[action]`.
- **Warnings** ignore only the offending key and the scan continues: unknown
  keys in other tables (`[duplication]`, `[dead_code]`, `[cycles]`, …) and
  unknown top-level tables.

Neither changes the exit code: a `--fail-on` / `--fail-on-new` run is still
decided by its findings, and prints a `gate:` line on stderr noting that
configuration issues were reported. Check the printed problems (or
`meta.issues`, see below) in CI to catch a broken file.

MCP scan tools return the same problems as an error result. The end-of-turn
gate and the language server keep working on defaults instead.

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

Setting any knob, including `action`, enables the rule; use `enabled = false`
for explicit suppression. A rule's own `action` wins over the
`[action.smells]` entry for the same smell. A flat threshold under `[smells]` or
`[smells.<lang>]` (`max_function_lines`, `max_cyclomatic`, `max_nesting`, …)
also enables its rule, unless the same table sets the rule's toggle (for
example `long_function = false`) or lists it in `disabled`.

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
test_sources = ["**/*.e2e.ts"]               # extra test-runner globs (test roots)
test_support = ["**/testing/**"]             # extra test-helper globs
orphan_modules = true                        # report modules no root reaches
```

Dead code walks the import graph from two kinds of roots:

- **Production roots:** `entry_points` globs, manifest entries (`pyproject.toml`
  scripts, Cargo binaries and `lib.rs`, `package.json` `main` / `module` / `bin`
  / `exports`, where a `dist/`, `lib/`, `build/` or `out/` target also names its
  `src/` twin), and tool configs such as `*.config.ts`. An entry-point file is
  never reported, and its imports count as usage.
- **Test roots:** `test_sources` globs plus the profile's test files.

A symbol that only tests import is reported at Medium with `reason =
"test_only"`. Files matching `test_support` (JS/TS default: `**/e2e/**`,
`**/test-utils/**`, `**/fixtures/**`, `**/__fixtures__/**`) are test helpers:
their test importers count as users, so only truly unused helpers are reported.
A module that nothing imports and no root reaches is reported at Medium with
`reason = "unreachable_module"`; add its path to `entry_points` if a tool runs
it, or set `orphan_modules = false`.

Names a barrel re-exports (`export { x } from "./x"`) count as used in their
defining module only when someone imports them from the barrel (chains and
`export *` included). A re-export nobody imports is reported with kind
`reexport`.

Confidence is High unless evidence suggests hidden use, then Medium: a default
export, an importer that binds the module whole or not at all (`import()`,
`import mod`), a type-only namespace reference, an import from a tool config,
test-only use, or an unreachable module.

`entrypoints` and `entrypoint_names` accept a trailing `*` as a prefix pattern
(`entrypoint_names = ["on*"]`). `entrypoint_names` also applies to methods under
`unused_methods`. For JS/TS methods only, React lifecycle, custom-element and
test-reporter callback names are live by default, and methods of a class that
extends or implements anything outside the scan are not reported (they may be
framework hooks). In JS/TS, `entrypoints` matches the callee of an
exported initializer, so `entrypoints = ["register", "define*"]` keeps
`export const send = register({ ... })` alive.

If dead-code output feels incomplete, broader unused-symbol classes are
available as opt-ins (your generated config shows their defaults):

```toml
[dead_code]
unused_imports = true
unused_methods = true
```

Unused module-level variables and constants are reported by default; set
`unused_variables = false` to turn them off.

Prefer adding entry points over suppressing individual findings — one line of
config fixes a whole class of false positives.

## Duplication

```toml
[duplication]
threshold = 50   # minimum structural-token run to report a clone; raise to quieten, 0 disables
max_gap = 25     # stitch clones across small consistent edits; 0 disables
exclude = ["**/tests/**", "**/migrations/**"]   # clone-only excludes (files stay in the graph)
include_tests = false       # true also compares test files
include_generated = false   # true also compares generated files
```

Duplication and smells skip, on top of `exclude`:

- **Test files** matched by the language profile (JS/TS: `__tests__/`,
  `*.test.*`, `*.spec.*`, `tests/`, `test/`, plus `__mocks__/`,
  `__fixtures__/`, `e2e/`, `test-utils/` and `*.stories.*`) and the built-in
  `tests/`, `test/` and pytest file names. The Python profile also counts
  `examples/` and `docs_src/`, and the Rust profile `benches/`, so those are
  skipped too. Set `include_tests = true` under
  `[duplication]` or `[smells]` to analyse them. `weak_test_oracle` and
  `fixture_echo_assertion` always inspect test files.
- **Generated files** (JS/TS): paths under `_generated/` or `__generated__/`,
  `*.gen.*` and `*.generated.*`, or a first-1 KB header containing
  `@generated`, `do not edit`, `auto-generated`, `automatically generated` or
  `code generated by`. They stay in the graph for cycles and dead code, but
  never count as smell hotspots or importers for `god_module` and
  `shotgun_surgery_hazard`.
- **Migrations**: `migrations/`, plus `alembic/` and `versions/` for Python
  files only.

`sensez noze . --summary` lists the effective excludes and skip counts under
`scope`, and the human report prints one dimmed line under Duplication when
files were skipped.

Structural matching keeps member, property and type names, so a copy that
renames only locals is still a clone. Whole functions at least twice the
`threshold` are also compared after a consistent rename of every name and
literal, so a copy that renames its members is found too. `near_miss = true`
extends that rename pass to functions down to `threshold`.

Opt-in detectors for harder clones: `class_name_duplicates`,
`class_property_overlap_min`, and `[duplication.semantic]` (only in builds with
the `eyez` feature). If the semantic pass is enabled but its embedding model
cannot load (for example offline with no cached model), the scan prints an
`analyze` issue on stderr, and `--fail-on` / `--fail-on-new` exit `2` with
"incomplete scan" rather than passing. JSON output lists the issue under
`meta.issues` only when `SENSEZ_SCAN_DIAGNOSTICS` or `SENSEZ_TIMING` is set.

```toml
[duplication.semantic]
enabled = true
min_shape_score = 30       # token 4-gram overlap (0-100) separating edited copies from rewrites; above 100 is an error
comment_boost_score = 85   # doc similarity (50-100) that confirms an edited copy; below 50 is an error
comment_required = false   # true compares documented functions only
```

Semantic groups only consider functions of at least `threshold` tokens. Each
pair is scored four ways: structural overlap (4-gram Jaccard), and the cosine,
under the code embedding model, of the two functions' source, of their
signatures (name and parameter words, ignoring words that many functions in
the codebase share) and of their doc comments. A group carries one of two
verdicts:

- `semantic: edited copy (structure 0.48, code 0.91, names 0.88, docs n/a)`:
  overlap at least `min_shape_score`, confirmed by similar source and similar
  signatures, or by matching doc comments. Reported at the duplication action
  level.
- `semantic: same intent, structurally different code (structure 0.12, code 0.86, names 0.92, docs n/a)`:
  a rewrite of the same function, for example a loop and a `filter`/`map`
  chain. Reported as `info`.

Pairs are only compared when they share enough uncommon structure, signature
words or doc words; words and shapes used by most functions in the codebase
never pair two functions on their own.

When several detectors report the same code, the widest group is kept, its
line ranges grow to cover the others, and the folded groups are listed under
`absorbed` in JSON output. Each group's source is in `detector`. The `hint`
says the token count (a range when a folded group is wider), the number of
copies, what to do, and how many overlapping groups were folded in.

## Cycles

```toml
[cycles]
exclude = ["**/vendor/**"]   # import edges here never report a cycle
include_type_only = true     # also report cycles closed only by type imports
```

A cycle that only exists through type-level imports (TypeScript
`import type`, Python `if TYPE_CHECKING:`) is reported at `info` with a "type
coupling" hint. Set `include_type_only = false` to drop those; this is
recommended for TypeScript, where type imports vanish at build time.

Cycles among test files are reported. Only `migrations/` (and Python
`alembic/` and `versions/`) are excluded by default; add
`exclude = ["**/tests/**"]` to silence test fixtures.

Each cycle finding lists every module of the strongly connected group in
`modules`; `edges` is one representative closed loop (the shortest one through
the alphabetically first module), so following it returns to its start.

## Boundaries

Each rule forbids any module matching `from` from importing any module matching
`to`:

```toml
[[boundaries.forbidden]]
from = "app.domain"
to = "app.api"
```

Matching, per endpoint:

- A plain name is a module-name prefix — `app.domain` matches `app.domain`
  and `app.domain.*`; for JavaScript and TypeScript modules, which are named by
  path, `src/domain` matches `src/domain` and `src/domain/*`.
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
duplication = ["legacy-fork/"]             # an accepted copy location
```

`duplication` patterns work per location. A plain path substring drops the
matching occurrences from every clone group, and a group stays reported while
two or more other copies remain, so accepting a fork never hides duplication
elsewhere. A pattern containing `" + "` matches the group label
(`a.ts + b.ts + c.ts`) and hides that whole group.

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
