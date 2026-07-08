#!/usr/bin/env bash
# Single-task PoC: compare control vs Sensez with Deepseek v4 Flash.
#
# Usage:
#   bash evals/poc_deepseek_v4_run.sh
#   BENCHMARK=tinyforms_ab bash evals/poc_deepseek_v4_run.sh
#   TASK_ID=tinyforms-report-summary BENCHMARK=tinyforms_ab bash evals/poc_deepseek_v4_run.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SENSE_BIN="${SENSE_BIN:-$REPO_ROOT/target/debug/sensez}"
BENCHMARK="${BENCHMARK:-swe_polybench_ab}"
OC_HOME_DIR="${OC_HOME_DIR:-/tmp/opencode_poc_home}"
REPO_CACHE="${REPO_CACHE:-$HOME/.cache/swe_polybench_repos}"
MODEL="${MODEL:-opencode-go/deepseek-v4-flash}"

case "$BENCHMARK" in
  swe_polybench_ab|swe)
    TASKS_JSONL="${TASKS_JSONL:-$REPO_ROOT/evals/swe_polybench_ab/tasks.jsonl}"
    RESULTS_DIR="${RESULTS_DIR:-$REPO_ROOT/evals/swe_polybench_ab/results_deepseek_v4_poc}"
    WORKSPACES_DIR="${WORKSPACES_DIR:-/tmp/swe_polybench_poc_workspaces}"
    TASK_ID="${TASK_ID:-langchain-ai__langchain-19331}"
    RUN_TESTS="${RUN_TESTS:-0}"
    ;;
  tinyforms_ab|tinyforms)
    TASKS_JSONL="${TASKS_JSONL:-$REPO_ROOT/evals/tinyforms_ab/tasks.jsonl}"
    RESULTS_DIR="${RESULTS_DIR:-$REPO_ROOT/evals/tinyforms_ab/results_deepseek_v4_poc}"
    WORKSPACES_DIR="${WORKSPACES_DIR:-/tmp/tinyforms_poc_workspaces}"
    TASK_ID="${TASK_ID:-tinyforms-import-cleanup}"
    RUN_TESTS="${RUN_TESTS:-1}"
    ;;
  *)
    echo "Unknown BENCHMARK: $BENCHMARK" >&2
    exit 2
    ;;
esac

export SWE_POLYBENCH_REPO_CACHE="$REPO_CACHE"

echo "=== Seeding opencode config dirs ==="
python3 "$REPO_ROOT/evals/common/seed_opencode_home.py" \
  "$OC_HOME_DIR" \
  --sense-bin "$SENSE_BIN"

echo ""
echo "=== Preparing workspaces ==="
# Create a single-task JSONL
TEMP_TASKS=$(mktemp)
python3 -c "
import json
task_id = '$TASK_ID'
with open('$TASKS_JSONL') as f:
    for line in f:
        task = json.loads(line)
        if task_id == 'all' or task['id'] == task_id:
            print(json.dumps(task))
" > "$TEMP_TASKS"

if [ ! -s "$TEMP_TASKS" ]; then
  echo "No tasks matched TASK_ID=$TASK_ID in $TASKS_JSONL" >&2
  rm -f "$TEMP_TASKS"
  exit 2
fi

echo "Benchmark: $BENCHMARK"
echo "Tasks:"
python3 -c "
import json
with open('$TEMP_TASKS') as f:
    for line in f:
        task = json.loads(line)
        print(f'  {task[\"id\"]}: [{task[\"category\"]}] {task[\"summary\"][:100]}')
"
echo ""
echo "=== Pre-seeding repo cache ==="
mkdir -p "$REPO_CACHE"
python3 -c "
import json
with open('$TEMP_TASKS') as f:
    repos = sorted({json.loads(line)['repo'] for line in f})
for repo in repos:
    print(repo)
" | while read -r repo; do
  if [[ "$repo" == local/* ]]; then
    echo "  $repo (generated locally)"
    continue
  fi
  repo_name="${repo##*/}"
  if [ ! -d "$REPO_CACHE/$repo_name" ]; then
    echo "  Cloning $repo (bare)..."
    git clone --bare "https://github.com/$repo.git" "$REPO_CACHE/$repo_name" 2>&1 | tail -1
  else
    echo "  $repo_name (cached)"
  fi
done

TEST_ARGS=()
if [ "$RUN_TESTS" = "1" ]; then
  TEST_ARGS=(--test-command-template "{test_command}")
fi

echo ""
echo "=== Running A/B eval ==="
python3 "$REPO_ROOT/evals/common/run_ab.py" \
  --tasks "$TEMP_TASKS" \
  --workspace-template "$WORKSPACES_DIR/{task_id}-{variant}-{run}" \
  --oc-home-template "$OC_HOME_DIR/{variant}" \
  --allow-dirty-start \
  --prepare-command-template \
    "python3 $REPO_ROOT/evals/common/prepare_repo.py {repo} {base_commit} {workspace} {task_id}" \
  --agent-command-template \
    "opencode run -m $MODEL --format json --dangerously-skip-permissions --dir {workspace} -f {prompt_file}" \
  --stdin-message "Implement the benchmark task as described in the attached file. Make minimal, correct changes. Follow the instructions carefully." \
  --results-dir "$RESULTS_DIR" \
  --sense-bin "$SENSE_BIN" \
  --runs 1 \
  --agent-timeout 1200 \
  "${TEST_ARGS[@]}" \
  --parallel

rm -f "$TEMP_TASKS"

echo ""
echo "=== Results ==="
python3 "$REPO_ROOT/evals/common/summarize.py" "$RESULTS_DIR"

echo ""
echo "=== Individual metrics ==="
find "$RESULTS_DIR" -name metrics.json -exec sh -c '
  echo ""
  echo "--- $1 ---"
  python3 -c "
import json
m = json.load(open(\"$1\"))
for k in [\"task_id\", \"variant\", \"run\", \"agent_returncode\",
           \"agent_elapsed_seconds\", \"agent_timed_out\",
           \"sensez_diff\", \"sensez_delta_total\",
           \"quality_regression_score\", \"sensez_tool_calls\",
           \"input_tokens\", \"output_tokens\", \"reasoning_tokens\",
           \"diff_stats\"]:
    print(f\"  {k}: {m.get(k)}\")
"
' _ {} \;
