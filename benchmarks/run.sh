#!/usr/bin/env bash
#
# sensez performance benchmark — clones pinned repos, runs sensez N times
# per target, compares against stored baselines, and fails CI if average
# wall-clock time exceeds the baseline by more than THRESHOLD_PCT %.
#
# Baseline management:
#   Set SENSEZ_WRITE_BASELINE=1 to overwrite baselines.json with current
#   results (commit the updated file afterwards). Without this flag the
#   script compares against the committed baseline and exits non-zero when
#   any target regresses beyond the threshold.
#
# Usage:
#   ./benchmarks/run.sh
#   SENSEZ_WRITE_BASELINE=1 ./benchmarks/run.sh
#   SENSEZ_BIN=./custom-sensez RUNS=7 ./benchmarks/run.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

SENSEZ_BIN="${SENSEZ_BIN:-$REPO_ROOT/target/release/sensez}"
RUNS="${RUNS:-7}"
WARMUP="${WARMUP:-2}"
THRESHOLD_PCT="${THRESHOLD_PCT:-5}"
BASELINE_FILE="$SCRIPT_DIR/baselines.json"
BENCH_CACHE="${BENCH_CACHE:-/tmp/sensez-benchmarks}"

# -- helpers ------------------------------------------------------------------
red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
bold()  { printf '\033[1m%s\033[0m\n' "$*"; }

# -- clone targets ------------------------------------------------------------
echo "==> Cloning benchmark targets"
source "$SCRIPT_DIR/targets.sh"
clone_targets

# -- build sensez -------------------------------------------------------------
if [ ! -x "$SENSEZ_BIN" ]; then
  echo "==> Building sensez (release, all-langs)"
  cargo build --release --features all-langs --manifest-path "$REPO_ROOT/Cargo.toml"
fi
echo "==> sensez binary: $SENSEZ_BIN"

# -- time a single run --------------------------------------------------------
time_run() {
  local start end
  start=$(python3 -c 'import time; print(time.perf_counter())')
  "$SENSEZ_BIN" noze "$1" --json --all >/dev/null 2>&1
  end=$(python3 -c 'import time; print(time.perf_counter())')
  python3 -c "print(round($end - $start, 3))"
}

# -- benchmark one target -----------------------------------------------------
bench_target() {
  local name="$1" path="$2"

  for _ in $(seq 1 "$WARMUP"); do
    time_run "$path" >/dev/null
  done

  local times=() sum=0 t
  for _ in $(seq 1 "$RUNS"); do
    t=$(time_run "$path")
    times+=("$t")
    sum=$(python3 -c "print($sum + $t)")
  done

  local mean stddev commit
  mean=$(python3 -c "print(round($sum / $RUNS, 3))")
  stddev=$(python3 -c "
import statistics
print(round(statistics.stdev([$(IFS=,; echo "${times[*]}")]), 3))
" 2>/dev/null || echo "0.0")
  commit=$(git -C "$path" rev-parse --short HEAD 2>/dev/null || echo "unknown")

  python3 -c "
import json
print(json.dumps({
    'target': '$name',
    'mean_s': $mean,
    'stddev_s': $stddev,
    'runs': $RUNS,
    'commit': '$commit',
}))
"
}

# -- main ---------------------------------------------------------------------
echo ""
bold "==> Running benchmarks (${RUNS} runs, ${WARMUP} warmup, ${THRESHOLD_PCT}% threshold)"

RESULTS=()
for name in "${TARGETS[@]}"; do
  path="$BENCH_CACHE/$name"
  [ -d "$path" ] || { red "  $name: not found"; continue; }

  echo ""
  bold "--- $name ---"
  result=$(bench_target "$name" "$path")
  mean=$(echo "$result" | python3 -c 'import json,sys; print(json.load(sys.stdin)["mean_s"])')
  echo "  mean: ${mean}s"
  RESULTS+=("$result")
done

combined=$(python3 -c "
import json
combined = []
$(
  for r in "${RESULTS[@]}"; do
    echo "combined.append(json.loads('''$r'''))"
  done
)
print(json.dumps(combined))
")

echo ""
bold "==> Comparison"

if [ "${SENSEZ_WRITE_BASELINE:-0}" = "1" ]; then
  echo "$combined" > "$BASELINE_FILE"
  green "baseline written to $BASELINE_FILE — commit this file."
  exit 0
fi

if [ ! -f "$BASELINE_FILE" ]; then
  echo "$combined" > "$BASELINE_FILE"
  green "no baseline found — wrote initial baseline to $BASELINE_FILE"
  exit 0
fi

python3 - "$THRESHOLD_PCT" "$combined" "$(cat "$BASELINE_FILE")" <<'PY'
import json, sys

threshold_pct = float(sys.argv[1])
current = json.loads(sys.argv[2])
previous = json.loads(sys.argv[3])

prev_map = {r["target"]: r for r in previous}
failed = False

for cur in current:
    name = cur["target"]
    mean = cur["mean_s"]
    prev = prev_map.get(name)
    if prev is None:
        print(f"  {name}: {mean:.3f}s  (no baseline — skipped)")
        continue
    delta_pct = (mean - prev["mean_s"]) / prev["mean_s"] * 100
    marker = ""
    if delta_pct > threshold_pct:
        marker = "  \033[31mREGRESSION\033[0m"
        failed = True
    elif delta_pct > 0:
        marker = "  \033[33mwithin threshold\033[0m"
    else:
        marker = "  \033[32mok\033[0m"
    print(f"  {name}: {mean:.3f}s  (baseline {prev['mean_s']:.3f}s, {delta_pct:+.1f}%){marker}")

sys.exit(1 if failed else 0)
PY
