#!/usr/bin/env bash
#
# sensez benchmark harness — compares sensez (all pillars, ONE pass) against the
# single-purpose structural tools it complements, per language, on real
# codebases.
#
# Sensez is the structural maintainability layer; it is NOT a linter or
# type-checker. These comparisons measure it against other *structural* tools
# (duplication / dead-code / cycle / smell detectors) — never against
# ruff/eslint/ty/tsc, which solve a different (per-file) problem.
#
# Other solutions by language / pillar:
#   Python      vulture (dead) · pycycle (cycles) · symilar (dup) · smellcheck (smells)
#   TypeScript  fallow-rs (dead/structural)
#   Python+TS   repowise (multi-pillar repo analysis)
#
# Each solution is AVAILABILITY-GUARDED: a tool that isn't installed is
# skipped with a note, so a partial toolchain still yields a partial report.
#
# Usage:
#   ./bench.sh <path|name=path> ...
#   ./bench.sh name=path:py  name=path:ts ...   # force language; else auto-detected
#   THRESHOLD=40 RUNS=3 ./bench.sh ...          # sensez dup threshold; sensez timing repeats
#   SKIP="symilar repowise" ./bench.sh ...      # skip named solutions
#
# Each run appends rows to results/runs.jsonl and saves raw tool output under
# results/<name>/. Build the dashboard afterwards:  uv run python report.py
#
# NOTE: there is no timeout on any solution. Some (symilar) are O(N^2)-ish and
# may run a very long time on large codebases — intentional, so timing is honest.

set -uo pipefail
BC="$(cd "$(dirname "$0")" && pwd)"
SENSEZ="$(cd "$BC/.." && pwd)/target/release/sense"
THRESHOLD="${THRESHOLD:-40}"
RUNS="${RUNS:-3}"
SKIP="${SKIP:-}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$BC/results"

# sensez must be built with every language grammar so it can scan py/ts/rust
# targets. (mcp/semantic are irrelevant to benchmarking and left off.)
if [ ! -x "$SENSEZ" ]; then
  echo "building sensez release binary (all-langs)..."
  ( cd "$BC/.." && cargo build --release --features all-langs ) || { echo "cargo build failed"; exit 1; }
fi

uvrun() { uv run --project "$BC" --quiet "$@"; }
have()  { command -v "$1" >/dev/null 2>&1; }
skipped() { case " $SKIP " in *" $1 "*) return 0 ;; *) return 1 ;; esac; }
FALLOW="$BC/node_modules/.bin/fallow"

# A Python entry-point is importable iff its module spec resolves in the bench venv.
py_tool_available() { uvrun python -c "import importlib.util as u,sys; sys.exit(0 if u.find_spec('$1') else 1)" 2>/dev/null; }

# real wall-clock seconds of a command (stdout → $1, stderr discarded).
measure() { local out="$1"; shift; /usr/bin/time -p "$@" >"$out" 2>"$TMP/t"; grep '^real' "$TMP/t" | awk '{print $2}'; }

record() { # target path files lines tool pillar seconds out lang
  python3 - "$@" >>"$BC/results/runs.jsonl" <<'PY'
import json, sys, time
k = ["target","path","files","lines","tool","pillar","seconds","out","lang"]
rec = dict(zip(k, sys.argv[1:10]))
rec["files"] = int(rec["files"]); rec["lines"] = int(rec["lines"]); rec["seconds"] = float(rec["seconds"])
rec["ts"] = int(time.time())
print(json.dumps(rec))
PY
}

# Extensions + solution roster per language. To add a tool: give it a branch
# in run_solution and list it here.
exts_for()        { case "$1" in py) echo "py";; ts) echo "ts tsx";; rust) echo "rs";; esac; }
solutions_for() { # echo "tool:pillar" per language (order = report order)
  case "$1" in
    py) echo "vulture:dead pycycle:cycles symilar:dup smellcheck:smells repowise:dead" ;;
    ts) echo "fallow:dead repowise:dead" ;;
    *)  echo "" ;;
  esac
}

# Count files/lines for a language under $path.
# Vendor/build dirs excluded from the size denominator — they're gitignored, so
# sensez (and fallow) don't analyze them; counting them would inflate lines/file.
PRUNE=( -path '*/node_modules/*' -o -path '*/.venv/*' -o -path '*/.git/*' -o -path '*/target/*' -o -path '*/dist/*' -o -path '*/build/*' )
corpus_size() { # path lang -> "files lines"
  local path="$1" lang="$2" files=0 lines=0 e
  for e in $(exts_for "$lang"); do
    local n l
    n=$(find "$path" \( "${PRUNE[@]}" \) -prune -o -name "*.$e" -print 2>/dev/null | wc -l | tr -d ' ')
    l=$(find "$path" \( "${PRUNE[@]}" \) -prune -o -name "*.$e" -print 2>/dev/null -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')
    files=$((files + n)); lines=$((lines + l))
  done
  echo "$files $lines"
}

# Auto-detect a target's primary language by file count (py vs ts).
detect_lang() {
  local path="$1" py ts
  py=$(find "$path" -name '*.py' 2>/dev/null | wc -l)
  ts=$(find "$path" \( -name '*.ts' -o -name '*.tsx' \) 2>/dev/null | wc -l)
  [ "$ts" -gt "$py" ] && echo ts || echo py
}

# Run one solution; writes raw (JSON where supported) output to $out. Returns
# nonzero (caller skips recording) when the tool isn't installed. Every tool is
# pinned to a single pillar so report.py can compare it to the matching sensez
# count; sensez still does ALL pillars in the one `sense scan` run above.
run_solution() { # tool path out lang  -> measured seconds on stdout, or "" if unavailable
  local tool="$1" path="$2" out="$3" lang="$4"
  case "$tool" in
    vulture)    py_tool_available vulture || return 1
                measure "$out" sh -c "uv run --project '$BC' --quiet vulture '$path' --min-confidence 60 2>/dev/null; true" ;;
    pycycle)    py_tool_available pycycle || return 1
                measure "$out" sh -c "cd '$path' && uv run --project '$BC' --quiet pycycle --here 2>&1; true" ;;
    symilar)    py_tool_available pylint || return 1
                measure "$out" sh -c "find '$path' -name '*.py' | xargs uv run --project '$BC' --quiet symilar 2>/dev/null; true" ;;
    # Python code-smell detector (PyPI: cheickmec/smellcheck) — sensez smells pillar.
    smellcheck) py_tool_available smellcheck || return 1
                measure "$out" sh -c "uv run --project '$BC' --quiet smellcheck '$path' --format json 2>/dev/null; true" ;;
    # Codebase-intelligence layer (PyPI: repowise-dev) — sensez dead-code pillar.
    repowise)   py_tool_available repowise || return 1
                measure "$out" sh -c "uv run --project '$BC' --quiet repowise dead-code '$path' --format json 2>/dev/null; true" ;;
    # TS/JS codebase analyzer (npm: fallow) — runs on cwd; sensez dead-code pillar.
    fallow)     [ -x "$FALLOW" ] || return 1
                measure "$out" sh -c "cd '$path' && '$FALLOW' dead-code --format json 2>/dev/null; true" ;;
    *)          return 1 ;;
  esac
}

TARGETS=()
for arg in "$@"; do TARGETS+=("$arg"); done
[ "${#TARGETS[@]}" -eq 0 ] && { echo "usage: ./bench.sh [name=]path[:py|:ts] ...   (SKIP=\"tool ...\" to omit)"; exit 1; }

for spec in "${TARGETS[@]}"; do
  # name=path:lang — name and :lang are optional.
  lang=""; case "$spec" in *:py) lang=py; spec="${spec%:py}";; *:ts) lang=ts; spec="${spec%:ts}";; esac
  if [[ "$spec" == *=* ]]; then name="${spec%%=*}"; path="${spec#*=}"; else path="$spec"; name="$(basename "$spec")"; fi
  path="$(cd "$path" 2>/dev/null && pwd)" || { echo "skip (not a dir): $spec"; continue; }
  [ -z "$lang" ] && lang="$(detect_lang "$path")"

  read -r files lines <<<"$(corpus_size "$path" "$lang")"
  od="$BC/results/$name"; mkdir -p "$od"
  echo "=== $name  [$lang]  ($files files, $lines lines)  path=$path ==="

  # sensez — all pillars in one pass, best of $RUNS.
  best=""
  for _ in $(seq 1 "$RUNS"); do
    t=$(measure "$od/sensez.json" "$SENSEZ" scan "$path" --threshold "$THRESHOLD" --json)
    [ -z "$best" ] && best="$t"
    awk "BEGIN{exit !($t<$best)}" && best="$t"
  done
  echo "  sensez (all pillars): ${best}s"
  record "$name" "$path" "$files" "$lines" sensez all "$best" "$od/sensez.json" "$lang"

  # Other solutions applicable to this language.
  for entry in $(solutions_for "$lang"); do
    tool="${entry%%:*}"; pillar="${entry##*:}"
    if skipped "$tool"; then echo "  $tool: skipped (SKIP)"; continue; fi
    out="$od/$tool.txt"
    if t=$(run_solution "$tool" "$path" "$out" "$lang") && [ -n "$t" ]; then
      echo "  $tool ($pillar): ${t}s"
      record "$name" "$path" "$files" "$lines" "$tool" "$pillar" "$t" "$out" "$lang"
    else
      echo "  $tool: not installed — skipped (see README for install)"
    fi
  done
done

echo
echo "done. build the dashboard with:  (cd $BC && uv run python report.py)"
