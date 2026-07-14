# Sensez

[![PyPI](https://img.shields.io/pypi/v/sensez?logo=pypi&logoColor=white)](https://pypi.org/project/sensez/) [![npm](https://img.shields.io/npm/v/sensez?logo=npm&logoColor=white)](https://www.npmjs.com/package/sensez) [![CI](https://github.com/popov95s/sensez/actions/workflows/ci.yml/badge.svg)](https://github.com/popov95s/sensez/actions/workflows/ci.yml) [![Website](https://img.shields.io/badge/website-sensez.dev-222222?logo=googlechrome&logoColor=white)](https://sensez.dev) [![Documentation](https://img.shields.io/badge/docs-latest-222222?logo=readthedocs&logoColor=white)](https://popov95s.github.io/sensez/latest/) [![License: MIT](https://img.shields.io/badge/license-MIT-2ea44f)](https://github.com/popov95s/sensez/blob/main/LICENSE.MD)

**Structural maintainability checks for coding agents and teams.**

Sensez is a suite of Rust CLIs and an MCP server that runs alongside your linter
and type-checker. It finds cross-file problems that those tools do not usually
own: duplication, dead code, import cycles, architecture-boundary violations,
and design smells.

It gives coding agents the `noze` to detect code smells, the `bonez` to respect
architectural boundaries, and the `spine` to do it fast. Python, JavaScript,
TypeScript/TSX, and Rust profiles are supported (Rust primarily for dogfooding).

**[Website](https://sensez.dev)** ·
**[Documentation](https://popov95s.github.io/sensez/latest/)** ·
**[MCP and agent guide](https://popov95s.github.io/sensez/latest/usage/mcp-and-agents/)** ·
**[Configuration reference](https://popov95s.github.io/sensez/latest/reference/configuration/)** ·

## Why Sensez?

Coding agents are excellent at producing code—and, occasionally, at producing
the same helper three times, gently ignoring architecture notes, and replacing
type-safe models with `dict[str, Any]`. The vibes are immaculate; the result
can still be a slopocalypse that takes hours to untangle.

Sensez closes that feedback loop while an edit is still fresh. It gives agents
short, structured feedback on the repository's shape, so problems can be fixed
before they become load-bearing.

The gap is especially visible in agent-driven work:

- **Context rots.** Architecture guidance fades across long, summarized turns.
- **CI is too late.** Small, non-blocking structural warnings are easy to defer.
- **Slow checks do not fit the turn.** If a check takes minutes, it cannot run
  in every feedback loop—and debt has time to accumulate.

```text
[Agent proposes turn finish] ──> [Sensez MCP sniff] ──> [Finds cycle / clone / smell]
                                      │
                                      └──> Immediate, actionable feedback
```

Sensez complements—not replaces—Ruff, ty, mypy, ESLint, TypeScript, `rustc`,
and Clippy. Use those tools for local correctness; use Sensez for the structural
relationships across the codebase.

## Quick start

### Python

```bash
# Add to a project; run it with `uv run sensez ...`
uv add --dev sensez
uv run sensez init

# Or install a global CLI
uv tool install sensez
sensez init

# Or run a one-off scan
uvx sensez noze .
```

### JavaScript and TypeScript

```bash
# Add as a development dependency
npm install --save-dev sensez

# Generate a starter config and scan
npx sensez init .
npx sensez noze .
```

`sensez .` and `sensez noze .` both run the default scan. The explicit form
`sensez noze sniff .` remains available for agent-oriented workflows.

## What it finds

| Area | Output key | What it catches |
| --- | --- | --- |
| Duplication | `duplication` | Structural clones, including local-rename copies. |
| Dead code | `dead_code` | Unreferenced symbols with confidence tiers. |
| Cycles | `cycles` | Import loops and load-order tangles. |
| Boundaries | `boundaries` | Imports that cross configured architecture rules. |
| Smells | `smells` | Design pressure inside functions, classes, modules, and the graph. |

Some examples of the included smells:

| Smell | Why Sensez flags it |
| --- | --- |
| `tuple_packing` | Positional tuples hide meaning; `tuple[int, str, int]` is not a data model. |
| `loose_typing` | `Any`, schema-erasing maps, and primitive containers erase caller contracts. |
| `boolean_blindness` | `do_thing(True, False)` makes argument meaning a guessing game. |
| `implicit_schema` | Repeated string-key access often means a real shape is hiding in a dict. |
| `mutated_parameter` | A function returns a parameter after chewing it up. |
| `feature_envy` | A method that mostly uses another object's data may belong elsewhere. |
| `message_chain` | Long `a.b.c.d` chains couple callers to deep object plumbing. |
| `god_module` | One module has become the place everything depends on. |
| `magic_string_default` | <code>&#124;&#124; ""</code> or <code>or ""</code> hides a string that should be required. |
| `split_variable` | Multiple reassignments of a variable in one scope add hidden state. |
| `nested_loop` | **Beta, opt-in:** nested iteration may have unintended complexity. |
| `n_plus_one_call` | **Beta, opt-in:** one-by-one external calls may need batching. |

The default report is intentionally fixable in one screen: each pillar shows its
top five offenders, each smell kind shows its total plus its top three examples,
and dead-code output includes high-confidence findings only. Use `--all` for
every finding, or `--max N` to set another cap.

```bash
# Focus a CI check on the pillars you care about
sensez noze . --duplicates
sensez noze . --duplicates --dead-code --json
```

## Performance

Sensez evaluates all structural pillars in one pass.

### Python: pylint benchmark

![Python benchmark: Sensez 0.27 s, Vulture 1.29 s, Repowise 17.26 s, Symilar 234.12 s](https://raw.githubusercontent.com/popov95s/sensez/main/docs/assets/benchmark-python.svg)

| Tool | Time | Scope |
| --- | ---: | --- |
| **sensez** | **0.27 s** | All structural pillars in one pass |
| vulture | 1.29 s | Python dead code |
| repowise | 17.26 s | Repository intelligence, including dead code |
| symilar | 234.12 s | Line-based duplication |

### JavaScript / TypeScript: zod benchmark

![JavaScript and TypeScript benchmark: Sensez 0.16 s, Fallow 0.48 s, Repowise 5.75 s](https://raw.githubusercontent.com/popov95s/sensez/main/docs/assets/benchmark-javascript.svg)

| Tool | Time | Scope |
| --- | ---: | --- |
| **sensez** | **0.16 s** | All structural pillars in one pass |
| fallow | 0.48 s | JS/TS structural dead-code and dependency findings |
| repowise | 5.75 s | Repository intelligence, including dead code |

## Agent impact

The same coding agent completed 70 real-world Python tasks from SWE-PolyBench
and SWE-bench Verified, plus 21 synthetic tasks designed to trigger specific
maintainability traps. Both variants received SOLID/DRY principles. The Sensez
variant additionally had to call `noze_sniff` in a feedback loop before it could
declare a task complete.

| | New quality issues | Clone tokens | New clones | Lines written | Tokens used |
| --- | ---: | ---: | ---: | ---: | ---: |
| Without Sensez | 14 | 1,251 | 129 | 2,080 | 1.23M |
| With Sensez | 2 | 126 | 0 | 682 | 1.34M |
| **Reduction** | **86%** | **90%** | **100%** | **67%** | **+8.7% overhead** |

Sensez agents produced structurally cleaner code with 67% fewer lines.
Benchmarks used SWE-PolyBench_500, SWE-bench Verified, and synthetic pillar
tests. See [the evaluation suite](https://github.com/popov95s/sensez/tree/main/evals)
for the methodology and per-benchmark results.

## MCP for agents

MCP is the recommended integration when Sensez should run repeatedly during a
coding session rather than as a one-off shell command. The `init` command would set this up automatically on agent start.

| Tool | Use |
| --- | --- |
| `noze_sniff` | Scan the repository for smells and structural issues. |
| `noze_gate` | Experimental end-of-turn diff gate; may be noisy for short or Q&A turns. |
| `noze_explain` | Explain a finding category. |
| `brainz_report` | Summarize local usage and resolution metrics. |
| `brainz_triage` | Record user-approved debt or false-positive verdicts. |
| `eyez_search_docs` | Disabled unless `eyez` is enabled; searches docstrings and comments. |

Sensez can also run standalone in GitHub Actions. See the
[GitHub Action guide](https://popov95s.github.io/sensez/latest/usage/github-action/).

## Configuration

Sensez reads `sensez.toml` from the project root, or `[tool.sensez]` from
`pyproject.toml` when `sensez.toml` is absent.

```bash
sensez init
```

The main configuration areas are:

- `[duplication]` for clone thresholds
- `[dead_code]` for dynamic entry points
- `[smells]` for smell toggles and thresholds
- `[[boundaries.forbidden]]` for architecture contracts
- `[action]` for how strongly agents and gates treat each pillar
- `[accept]` for shared accepted findings
- `[self_improvement]` for local metrics

```toml
[duplication]
threshold = 50

[dead_code]
entrypoint_names = ["register", "main", "setup"]

[smells.rules.long_function]
max_lines = 80
action = "warning"

# The beta performance heuristics are disabled by default.
[smells.rules.nested_loop]
enabled = true

[smells.rules.n_plus_one_call]
enabled = true
```

## Local-only metrics and privacy

`brainz` records scans, gate blocks, triage decisions, resolved findings,
regressions, detector precision, and usage reports locally.

```bash
sensez brainz report .
sensez brainz report . --json
```

Everything remains under `.sensez/local-metrics/`. Sensez sends no telemetry
and uploads no source code. Disable local metrics for a repository with:

```toml
[self_improvement]
enabled = false
```

## Project anatomy

- `spine`: file discovery, parsing, shared IR, and dependency graph.
- `profiles`: language adapters for Python, JS/TS, TSX, and Rust.
- `noze`: duplication, dead code, cycles, and design smells.
- `bonez`: architecture-boundary auditing; not yet enabled.
- `brainz`: local-only metrics and feedback memory.
- `eyez`: optional doc/comment search; not yet enabled.
- `mcp`: JSON-RPC/MCP surface for agent integration.
- `reporter`: terminal and JSON output.
- `setup`: starter configuration, MCP registration, and hook setup.

## Disclaimer

Please review
[DISCLAIMER.md](https://github.com/popov95s/sensez/blob/main/DISCLAIMER.md) for
the project disclaimer.
