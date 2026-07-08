#!/usr/bin/env bash
# Run all eval suites: synthetic (Django) + tinyforms (local)
# Usage: bash evals/run_all_synthetic.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export SWE_POLYBENCH_REPO_CACHE="$HOME/.cache/swe_polybench_repos"
MODEL="${1:-opencode-go/deepseek-v4-flash}"
SENSE="$REPO_ROOT/target/debug/sensez"

echo "=== Synthetic Django tasks (15) ==="
python3 "$REPO_ROOT/evals/common/run_ab.py" \
  --tasks "$REPO_ROOT/evals/synthetic/tasks.jsonl" \
  --workspace-template "/tmp/swe_ws/{task_id}-{variant}-{run}" \
  --oc-home-template "/tmp/opencode_poc_home/{variant}" \
  --allow-dirty-start \
  --prepare-command-template \
    "python3 $REPO_ROOT/evals/common/prepare_repo.py {repo} {base_commit} {workspace}" \
  --agent-command-template \
    "opencode run -m $MODEL --format json --dangerously-skip-permissions --dir {workspace} -f {prompt_file}" \
  --stdin-message "Implement the benchmark task as described in the attached file. Make minimal, correct changes. Follow the instructions carefully." \
  --results-dir "$REPO_ROOT/evals/synthetic/results" \
  --sense-bin "$SENSE" \
  --runs 1 --agent-timeout 1200 --parallel

rm -rf /tmp/swe_ws

echo ""
echo "=== Synthetic smell-specific tasks (5) ==="
python3 "$REPO_ROOT/evals/common/run_ab.py" \
  --tasks "$REPO_ROOT/evals/synthetic/tasks_smells.jsonl" \
  --workspace-template "/tmp/swe_ws/{task_id}-{variant}-{run}" \
  --oc-home-template "/tmp/opencode_poc_home/{variant}" \
  --allow-dirty-start \
  --prepare-command-template \
    "python3 $REPO_ROOT/evals/common/prepare_repo.py {repo} {base_commit} {workspace}" \
  --agent-command-template \
    "opencode run -m $MODEL --format json --dangerously-skip-permissions --dir {workspace} -f {prompt_file}" \
  --stdin-message "Implement the benchmark task as described in the attached file. Make minimal, correct changes. Follow the instructions carefully." \
  --results-dir "$REPO_ROOT/evals/synthetic/results" \
  --sense-bin "$SENSE" \
  --runs 1 --agent-timeout 1200 --parallel

rm -rf /tmp/swe_ws

echo ""
echo "=== TinyForms tasks (3) ==="
# TinyForms uses local generation, not git clone
python3 "$REPO_ROOT/evals/common/run_ab.py" \
  --tasks "$REPO_ROOT/evals/tinyforms_ab/tasks.jsonl" \
  --workspace-template "/tmp/tinyforms/{task_id}-{variant}-{run}" \
  --oc-home-template "/tmp/opencode_poc_home/{variant}" \
  --allow-dirty-start \
  --prepare-command-template \
    "python3 $REPO_ROOT/evals/tinyforms_ab/prepare_repo.py {task_id} {workspace}" \
  --agent-command-template \
    "opencode run -m $MODEL --format json --dangerously-skip-permissions --dir {workspace} -f {prompt_file}" \
  --stdin-message "Implement the benchmark task as described in the attached file. Make minimal, correct changes. Follow the instructions carefully." \
  --results-dir "$REPO_ROOT/evals/tinyforms_ab/results" \
  --sense-bin "$SENSE" \
  --runs 1 --agent-timeout 1200 --parallel

rm -rf /tmp/tinyforms

echo ""
echo "=== Summary ==="
python3 "$REPO_ROOT/evals/common/summarize.py" "$REPO_ROOT/evals/synthetic/results" 2>&1 | head -20
echo ""
python3 "$REPO_ROOT/evals/common/summarize.py" "$REPO_ROOT/evals/tinyforms_ab/results" 2>&1 | head -20