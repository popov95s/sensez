#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
uv run --no-project python -m regression.run "$@"
