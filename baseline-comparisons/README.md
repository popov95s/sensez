# baseline-comparisons

Reproducible speed + result-parity benchmarks for `sensez` against the
single-purpose **structural** tools it complements, across languages.

> Sensez is the structural maintainability layer — it is **not** a linter or
> type-checker. Keep using ruff + ty (Python) / eslint + tsc (TypeScript) /
> clippy (Rust) for per-file correctness. These benchmarks therefore compare
> sensez only to *other structural tools* (duplication / dead-code / cycle /
> smell detectors), never to linters or type-checkers.

## Tools by language and pillar

| Language | sensez pillar | solution | parity |
|---|---|---|---|
| Python | dead code | `vulture` | ✅ set overlap |
| Python | import cycles | `pycycle` | ✅ agree / disagree |
| Python | duplication | `symilar` (pylint) | ✅ count (different granularity) |
| Python | design smells | `smellcheck` | ⚖️ count, not 1:1 |
| Python + TS | dead code | `repowise` | ⚖️ count, not 1:1 |
| TypeScript | dead / structural | `fallow` | ⚖️ count, not 1:1 |

- ✅ — report.py knows the tool's output format and computes a real
  findings-overlap / agreement note against sensez.
- ⚖️ — a real finding count parsed from the tool's JSON, but same-pillar ≠ 1:1:
  these tools use different finding models (see each row's note in the report).

`import-linter` (the would-be boundaries solution) isn't benchmarked — it needs
per-project contracts, and it was dropped from the deps anyway (its `rich>=14`
pin conflicts with repowise's `rich<14`).

## Installing the toolchain

The Python solutions are pinned in `pyproject.toml` (a `uv` project) — one sync
installs all of them:

```bash
uv sync --project .    # vulture, pycycle, symilar (pylint), smellcheck, repowise
```

`fallow` (TypeScript) is a Node CLI pinned in this directory:

```bash
cd baseline-comparisons && npm install
```

For TypeScript targets, also install the *target's own* dependencies so `fallow`
can resolve imports — without them it warns and under-reports:

```bash
cd /path/to/ts-project && pnpm install   # or npm/yarn, whatever the project uses
```

bench.sh availability-guards every solution, so a partial toolchain still
produces a partial report (a missing tool is skipped with a note).

## Run a benchmark

```bash
# fetch the standard target repos into /tmp/bench-targets
./targets.sh
(cd /tmp/bench-targets/zod && corepack pnpm install --frozen-lockfile)

# one or more target codebases; language auto-detected (py vs ts) by file count
./bench.sh /path/to/django
./bench.sh django=/path/to/django web=/path/to/ts-app:ts   # name= label, :ts forces language

# tuning
THRESHOLD=40 RUNS=3 ./bench.sh <path>          # sensez dup threshold; sensez timing repeats
SKIP="symilar repowise" ./bench.sh <path>      # omit named solutions (e.g. the slow ones)
```

Sensez is built once with `--features all-langs` so it can scan Python,
TypeScript, and Rust targets. Each invocation appends timing rows to
`results/runs.jsonl` and saves raw tool output under `results/<name>/`.
**There is no timeout** — `symilar` is O(N²)-ish and can run for a very long
time on large codebases; that is intentional, so the comparison is honest.

## Visualize

```bash
uv run python report.py        # prints a terminal table + writes results/report.html
open results/report.html       # self-contained dashboard (no external deps)
```

`report.py` reads the latest run per (target, tool) from `results/runs.jsonl`,
recomputes findings/parity from the saved outputs, and renders a card per
target: Sensez' single all-pillar run vs. each solution's time, the slowdown
factor, and a parity note.

## Code layout

The harness is a small package, one responsibility per module (so sensez doesn't
flag its own benchmark code):

```
report.py            thin CLI: load → compare → render
bench/
  model.py           plain dataclasses (Run, SensezFindings, Verdict, Comp, Row)
  loading.py         read results/ artifacts (runs.jsonl, sensez.json)
  solutions.py       Solution registry + per-tool parse/compare strategies
  compare.py         runs → comparison rows (pure)
  render.py          rows → terminal text / HTML dashboard
```

## Adding a tool

1. **bench.sh** — add a `tool:pillar` entry to `solutions_for <lang>` and a
   branch in `run_solution` (availability-guarded, emitting JSON where it can).
2. **bench/solutions.py** — append one `Solution(name, label, judge)` to
   `SOLUTIONS`. Reuse a calibrated judge (`_judge_vulture`/`_judge_pycycle`/
   `_judge_symilar`) or `_cmp_judge(sensez_field, note)` for a JSON-emitting,
   same-pillar tool. No other module changes — that's the whole point.

## Expanded dead-code categories (opt-in)

By default sensez reports only unused top-level **functions/classes** (the
high-precision set). To approach vulture's coverage, drop an `sensez.toml` in the
scanned project:

```toml
[dead_code]
unused_imports = true      # imports whose bound name is never used in-file
unused_methods = true      # class methods never referenced in their module (Low conf.)
unused_variables = true    # unused module-level variables
```

With these on, sensez↔vulture agreement on pylint rises from ~9% to ~68%
overall (methods ~98%).

## Parity notes
- **Cycles**: agreement is exact in spirit — both report cycles, or both report
  none. sensez excludes inline (function-local) imports, since those are how
  Python breaks cycles.
- **Dead code**: compared against vulture's *unused functions/classes* (the
  shared category). sensez tiers findings High/Medium/Low; High is the
  high-precision actionable set. sensez does **not** track unused
  methods/locals/imports by default (that's ruff's `F401`/`F841`), so it is a
  complementary — not identical — signal.
- **Duplication**: Sensez is token-structural (rename-invariant), symilar is
  line-based; counts are the same order of magnitude, not the same unit.
- **Smells / multi-pillar tools** (`smellcheck`, `repowise`, `fallow`): timing
  is comparable and the finding counts are real (parsed from each tool's JSON),
  but same-pillar ≠ 1:1 — they use different finding models (smellcheck emits a
  finding per SC-pattern hit; repowise flags file/export-level dead code; fallow
  bundles unused + cycles + deps). Read `comp_n` as same-pillar *volume*, not an
  exact equivalent; each report row carries the specific caveat.
