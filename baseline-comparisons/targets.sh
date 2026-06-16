#!/usr/bin/env bash
#
# Clone the real-world codebases used by bench.sh into /tmp/bench-targets.
# Existing checkouts are left untouched so local experiments are not reset.

set -euo pipefail

ROOT="${BENCH_TARGETS:-/tmp/bench-targets}"
mkdir -p "$ROOT"

clone_once() {
  local name="$1" url="$2"
  local dest="$ROOT/$name"
  if [ -d "$dest/.git" ]; then
    echo "$name: already present at $dest"
    return
  fi
  git clone --depth 1 "$url" "$dest"
}

clone_once flask  https://github.com/pallets/flask.git
clone_once django https://github.com/django/django.git
clone_once pylint https://github.com/pylint-dev/pylint.git
clone_once zod    https://github.com/colinhacks/zod.git

cat <<EOF

targets ready under $ROOT

For TypeScript accuracy, install the target's own dependencies once:
  (cd "$ROOT/zod" && corepack pnpm install --frozen-lockfile)
EOF
