#!/usr/bin/env bash
# 25-task A/B eval with Deepseek v4 Flash.
# Usage: bash evals/poc_multi_task.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SENSE_BIN="$REPO_ROOT/target/debug/sensez"
TASKS_JSONL="$REPO_ROOT/evals/swe_polybench_ab/tasks_expanded.jsonl"
RESULTS_DIR="$REPO_ROOT/evals/swe_polybench_ab/results_deepseek_v4_poc"
WORKSPACES_DIR="/tmp/swe_polybench_poc_workspaces"
OC_HOME_DIR="/tmp/opencode_poc_home"
REPO_CACHE="$HOME/.cache/swe_polybench_repos"
MODEL="opencode-go/deepseek-v4-flash"

export SWE_POLYBENCH_REPO_CACHE="$REPO_CACHE"

echo "=== Tasks ==="
python3 -c "
import json
with open('$TASKS_JSONL') as f:
    count = 0
    for line in f:
        t = json.loads(line)
        print(f'  {t[\"id\"]}: [{t[\"category\"]}] {t[\"summary\"][:80]}')
        count += 1
    print(f'\n  Total: {count} tasks')
"

echo ""
echo "=== Seeding opencode config dirs ==="
rm -rf "$OC_HOME_DIR"
python3 "$REPO_ROOT/evals/common/seed_opencode_home.py" \
  "$OC_HOME_DIR" \
  --sense-bin "$SENSE_BIN"

echo ""
echo "=== Pre-seeding repo cache ==="
mkdir -p "$REPO_CACHE"
for repo_name in keras langchain yt-dlp transformers; do
  if [ ! -d "$REPO_CACHE/$repo_name" ]; then
    echo "  Cloning $repo_name (bare)..."
    case "$repo_name" in
      keras)     URL="https://github.com/keras-team/keras.git" ;;
      langchain) URL="https://github.com/langchain-ai/langchain.git" ;;
      yt-dlp)    URL="https://github.com/yt-dlp/yt-dlp.git" ;;
      transformers) URL="https://github.com/huggingface/transformers.git" ;;
    esac
    git clone --bare "$URL" "$REPO_CACHE/$repo_name" 2>&1 | tail -1
  else
    echo "  $repo_name (cached)"
  fi
done

echo ""
echo "=== Running A/B eval (25 tasks) ==="
python3 "$REPO_ROOT/evals/common/run_ab.py" \
  --tasks "$TASKS_JSONL" \
  --workspace-template "$WORKSPACES_DIR/{task_id}-{variant}-{run}" \
  --oc-home-template "$OC_HOME_DIR/{variant}" \
  --allow-dirty-start \
  --prepare-command-template \
    "python3 $REPO_ROOT/evals/common/prepare_repo.py {repo} {base_commit} {workspace}" \
  --agent-command-template \
    "opencode run -m $MODEL --format json --dangerously-skip-permissions --dir {workspace} -f {prompt_file}" \
  --stdin-message "Implement the benchmark task as described in the attached file. Make minimal, correct changes. Follow the instructions carefully." \
  --results-dir "$RESULTS_DIR" \
  --sense-bin "$SENSE_BIN" \
  --runs 1 \
  --agent-timeout 1200 \
  --parallel

echo ""
echo "=== Results (all tasks) ==="
python3 "$REPO_ROOT/evals/common/summarize.py" "$RESULTS_DIR"
