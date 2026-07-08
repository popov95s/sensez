# SWE-PolyBench Sensez A/B Eval

This eval measures whether Sensez improves code produced by coding agents on
real Python repository tasks.

The first task set uses eight Python tasks from `AmazonScience/SWE-PolyBench_500`.
They are feature/refactor tasks chosen to be small enough for weaker models but
large enough to exercise API design, option plumbing, typing, duplication, and
cross-module changes.

## What It Compares

- `control`: the agent gets the normal task prompt.
- `sensez`: the same agent gets Sensez guidance and should have the Sensez MCP
  server configured by the calling environment.

The runner records:

- starting `git rev-parse HEAD` and `git status --short`
- before/after `sense noze --json` results
- final `git diff`
- agent stdout/stderr
- elapsed time
- optional benchmark/test command result
- compact metrics for smells, changed files, and changed lines

By default, a run fails if the workspace is dirty before the agent starts. Use
`--allow-dirty-start` only for debugging.

## Task Source

Tasks are listed in `tasks.jsonl`. The IDs are SWE-PolyBench instance IDs, so
they can be evaluated with the official SWE-PolyBench harness after collecting
patches.

Recommended first run:

```bash
python evals/common/seed_codex_home.py /private/tmp/codex-eval-home \
  --sense-bin /Users/spopov/Documents/istf/target/debug/sense

python evals/common/run_ab.py \
  --tasks evals/swe_polybench_ab/tasks.jsonl \
  --workspace-template "/path/to/workspaces/{task_id}-{variant}-{run}" \
  --prepare-command-template 'python3 /Users/spopov/Documents/istf/evals/common/prepare_repo.py {repo} {base_commit} {workspace}' \
  --control-agent-command-template 'env CODEX_HOME=/private/tmp/codex-eval-home codex --ask-for-approval never --sandbox workspace-write -m gpt-5.4-mini exec --json --ephemeral --ignore-user-config -' \
  --sensez-agent-command-template 'env CODEX_HOME=/private/tmp/codex-eval-home codex --dangerously-bypass-approvals-and-sandbox -m gpt-5.4-mini exec --json --ephemeral -' \
  --agent-prompt-stdin \
  --runs 3
```

If a workspace does not already exist, provide a preparation command:

```bash
python evals/common/seed_codex_home.py /private/tmp/codex-eval-home \
  --sense-bin /Users/spopov/Documents/istf/target/debug/sense

python evals/common/run_ab.py \
  --tasks evals/swe_polybench_ab/tasks.jsonl \
  --workspace-template "/path/to/workspaces/{task_id}-{variant}-{run}" \
  --prepare-command-template 'python3 /Users/spopov/Documents/istf/evals/common/prepare_repo.py {repo} {base_commit} {workspace}' \
  --control-agent-command-template 'env CODEX_HOME=/private/tmp/codex-eval-home codex --ask-for-approval never --sandbox workspace-write -m gpt-5.4-mini exec --json --ephemeral --ignore-user-config -' \
  --sensez-agent-command-template 'env CODEX_HOME=/private/tmp/codex-eval-home codex --dangerously-bypass-approvals-and-sandbox -m gpt-5.4-mini exec --json --ephemeral -' \
  --agent-prompt-stdin \
  --runs 3
```

The agent command template receives these placeholders:

- `{workspace}`
- `{prompt_file}`
- `{task_id}`
- `{variant}`
- `{run}`

Use `--agent-prompt-stdin` for agents like Codex that read prompts from stdin.
For agents that accept prompt file paths directly, omit that flag and use
`{prompt_file}` in the command template.

The runner expects a `sense` binary on `PATH`. Override it with `--sense-bin`.

## Correctness And Quality

Use three layers:

1. Independent tests collected by this harness via `--test-command-template`.
2. Official SWE-PolyBench scoring from exported patch predictions.
3. Maintainability scoring from Sensez diff findings and patch size.

For quick local pilots, pass the benchmark-relevant test command directly:

```bash
python evals/common/run_ab.py \
  --tasks /tmp/one_task.jsonl \
  --workspace-template "/path/to/workspaces/{task_id}-{variant}-{run}" \
  --agent-command-template 'codex exec -' \
  --agent-prompt-stdin \
  --test-command-template 'uv run --with pytest python -m pytest test/test_http.py -v --tb=short'
```

The summary reports agent success and test success separately. Treat test
success as the local correctness signal; use the official benchmark evaluator
for the publishable pass/fail score.

## Evaluate Correctness

For SWE-PolyBench, convert each `patch.diff` into the official predictions JSONL
format:

```bash
python evals/swe_polybench_ab/export_predictions.py \
  evals/swe_polybench_ab/results \
  --variant sensez \
  --run 1 \
  --output evals/swe_polybench_ab/predictions/sensez_run_1.jsonl
```

Then run the SWE-PolyBench evaluator against those predictions.

## Summarize

```bash
python evals/common/summarize.py evals/swe_polybench_ab/results
```

The summary is intentionally paired by task/run so it can answer: did Sensez
reduce new findings, patch size, time, and failed checks for the same task?
