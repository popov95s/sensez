#!/usr/bin/env bash
#
# Clone benchmark targets at pinned commits into $BENCH_CACHE.
# Shallow clones — only the requested commit, no history.
#
# Usage: source targets.sh   (populates the TARGETS array)

set -euo pipefail

BENCH_CACHE="${BENCH_CACHE:-/tmp/sensez-benchmarks}"
mkdir -p "$BENCH_CACHE"

clone_at() {
  local name="$1" url="$2" ref="$3"
  local dest="$BENCH_CACHE/$name"

  if [ -f "$dest/.sensez-clone-done" ]; then
    local current
    current=$(git -C "$dest" rev-parse HEAD 2>/dev/null || echo "")
    if [ "$current" = "$ref" ]; then
      echo "  $name: already at $ref"
      return
    fi
    echo "  $name: ref changed, re-cloning"
  fi

  if [ -e "$dest" ]; then
    rm -rf "$dest"
  fi

  echo "  $name: cloning $url @ $ref ..."
  git init --quiet -b main "$dest"
  git -C "$dest" remote add origin "$url"
  git -C "$dest" fetch --depth 1 origin "$ref" --quiet
  git -C "$dest" checkout FETCH_HEAD --quiet
  touch "$dest/.sensez-clone-done"
  echo "  $name: done ($(git -C "$dest" rev-parse --short HEAD))"
}

clone_targets() {
  clone_at django \
    https://github.com/django/django.git \
    5.1.4

  clone_at zod \
    https://github.com/colinhacks/zod.git \
    v3.23.8
}

TARGETS=(
  "django"
  "zod"
)
