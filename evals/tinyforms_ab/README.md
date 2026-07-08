# TinyForms Sensez A/B Eval

This eval measures whether Sensez improves code produced by coding agents on
small generated Python repository tasks.

The task manifest follows the same JSONL shape as `swe_polybench_ab/tasks.jsonl`.
The repository itself is generated locally by `prepare_repo.py`, so no GitHub
checkout or upstream benchmark harness is required.

## What It Compares

- `control`: the agent gets the normal task prompt.
- `sensez`: the same agent gets Sensez guidance and should have the Sensez MCP
  server configured by the calling environment.

The runner records the same artifacts as the SWE-PolyBench A/B suite: workspace
state, before/after Sensez scans, the final patch, agent output, optional tests,
and compact metrics.

## Task Source

Tasks are listed in `tasks.jsonl`. Each task is a focused one-function feature
with a unit test for correctness and a natural maintainability trap that Sensez
can score after the patch.

Recommended first run:

```bash
BENCHMARK=tinyforms_ab bash evals/poc_deepseek_v4_run.sh
```

To run a specific task:

```bash
BENCHMARK=tinyforms_ab TASK_ID=tinyforms-report-summary bash evals/poc_deepseek_v4_run.sh
```

The POC script uses the shared runner and shared prepare adapter:

```bash
python evals/common/prepare_repo.py local/tinyforms generated-fixture /tmp/workspace tinyforms-report-summary
```

## Correctness And Quality

Use two layers:

1. Local correctness from each task's `test_command`.
2. Maintainability scoring from Sensez diff findings and patch size.

Summarize results with:

```bash
python evals/common/summarize.py evals/tinyforms_ab/results
```
