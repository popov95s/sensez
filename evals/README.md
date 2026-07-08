# Sensez Evaluation Suites

Paired A/B evals measuring whether Sensez helps coding agents produce working code
with fewer maintainability regressions. All results below were generated with
**Deepseek v4 Flash** via the `opencode` CLI.

Shared runner code in `evals/common`. Prompts in `evals/prompts`.

## Active Suites

### `swe_polybench_ab` — SWE-PolyBench & SWE-bench (70 tasks)
70 Python tasks from SWE-PolyBench_500 and SWE-bench_Verified across django,
transformers, keras, langchain, yt-dlp, sympy, scikit-learn, flask, and more.
Feature, bug-fix, and refactoring tasks evaluated with a control prompt (SOLID/DRY
principles) vs a Sensez prompt that adds a mandatory `noze_sniff` feedback loop.

**Result**: Sensez eliminated 86% of newly-introduced quality issues, 90% of
clone tokens, and 100% of newly-introduced duplication across 8 improved tasks
out of 70. 67% less code written. 8.7% token overhead.

### `synthetic` — Targeted Pillar Tests (18 tasks)
Synthetic tasks in the Django codebase, each designed to trigger a specific
Sensez pillar: duplication, dead code, import cycles, and 8 unique design smells
(boolean blindness, loose typing, mutated parameter, tuple packing, implicit
schema, feature envy, literal membership, split variable, magic string default,
deep nesting).

**Result**: 5 wins across 18 paired tasks. 50 MCP tool calls. Clone tokens
eliminated (75→0). New quality issues reduced 34→28. 28% win rate — higher than
real-world benchmarks, proving synthetic tasks better exercise individual pillars.

### `tinyforms_ab` — Local Generated Tasks (3 tasks)
Three self-contained tasks in a locally-generated `tinyforms` Python package.
No GitHub cloning required. Each task targets a specific smell: split_variable,
mutated_parameter, and implicit_schema.

**Result**: `tinyforms-import-cleanup` showed a clear win — cognitive complexity
eliminated (20→0), smells halved (2→1), with 2 verified MCP calls.
Report-summary and request-cleanup were neutral (both agents wrote clean code).

## Measurement Stack

1. `qual_new` — weighted score of definitively new maintainability issues
2. `clone_tok` / `clone_new` — total and newly-introduced clone token counts
3. Severity metrics — cognitive complexity, clone class sizes, mutated param counts
4. Patch discipline — files touched, lines added/deleted
5. Agent behavior — MCP tool calls, token usage, wall time
6. Per-clone provenance — new / pre-existing / inherent classification

Generated result directories are git-ignored.
