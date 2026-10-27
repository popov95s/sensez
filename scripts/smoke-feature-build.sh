#!/usr/bin/env bash
#
# Build sensez with one feature set and scan a tiny project in every language.
# The npm and PyPI packages ship builds with only some languages compiled in;
# a scan must succeed, and must not touch a missing language profile.
#
# Usage: scripts/smoke-feature-build.sh lang-typescript,mcp,eyez

set -euo pipefail

features="${1:?usage: smoke-feature-build.sh <cargo features>}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
project="$(mktemp -d)"
trap 'rm -rf "$project"' EXIT

cargo build --release --no-default-features --features "$features" --manifest-path "$root/Cargo.toml"
binary="$root/target/release/sensez"

printf 'export const double = (x: number) => x * 2;\n' > "$project/a.ts"
printf 'export function triple(x) { return x * 3; }\n' > "$project/b.js"
printf 'def half(x):\n    return x / 2\n' > "$project/c.py"
printf 'pub fn negate(x: i32) -> i32 { -x }\n' > "$project/d.rs"

"$binary" noze "$project" --json > "$project/report.json"
python3 -c 'import json, sys; json.load(open(sys.argv[1]))' "$project/report.json"
"$binary" noze "$project" --fail-on must_fix
echo "ok: $features"
