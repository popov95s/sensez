#!/usr/bin/env bash
#
# setup-mcp.sh — build Sensez with semantic doc-search + MCP, register it as an
# MCP server, and pre-warm the embedding model.
#
# Usage:
#   scripts/setup-mcp.sh            # build, gitignore cache, write .mcp.json, warm model
#   scripts/setup-mcp.sh --serve    # the above, then run the server on stdio (smoke test)
#
# The MCP server speaks JSON-RPC over stdio and is normally launched BY the MCP
# client (Claude Code / Desktop), not run by hand. `--serve` is only for a quick
# manual check. The `search_docs` tool takes a `path` argument, so one registered
# server can search ANY repo — you don't need to re-register per project.

set -euo pipefail

# --- locate the repo root (this script lives in <root>/scripts) ----------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

FEATURES="${SENSEZ_FEATURES:-mcp,semantic,all-langs}"
BIN="$ROOT/target/release/sensez"

# --- 0. prerequisites ----------------------------------------------------------
if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust from https://rustup.rs and re-run." >&2
  exit 1
fi

# --- 1. build the release binary ----------------------------------------------
echo "==> Building Sensez CLI (sensez, release, features: $FEATURES)"
cargo build --release --features "$FEATURES"
echo "    binary: $BIN"

# --- 2. keep the derived cache out of git -------------------------------------
if [ -f .gitignore ] && ! grep -qxF '.sensez/' .gitignore; then
  printf '\n# sensez semantic doc-search cache (derived, model-specific)\n.sensez/\n' >> .gitignore
  echo "==> Added '.sensez/' to .gitignore"
fi

# --- 3. write a ready-to-use MCP server config --------------------------------
# Project-scoped .mcp.json is picked up by Claude Code when this repo is open.
# To use it everywhere, copy the "sensez" block into your user-scope MCP config.
MCP_JSON="$ROOT/.mcp.json"
echo "==> Writing $MCP_JSON"
cat > "$MCP_JSON" <<JSON
{
  "mcpServers": {
    "sensez": {
      "command": "$BIN",
      "args": ["serve"]
    }
  }
}
JSON

# If the Claude CLI is available, also register at user scope (works in any repo).
if command -v claude >/dev/null 2>&1; then
  echo "==> Registering 'sensez' with the Claude CLI (user scope)"
  claude mcp add sensez -s user -- "$BIN" serve 2>/dev/null \
    || echo "    (already registered or add failed — see .mcp.json above)"
else
  echo "    Claude CLI not on PATH — using project .mcp.json."
  echo "    For global use, add this to your user MCP config:"
  echo "      \"sensez\": { \"command\": \"$BIN\", \"args\": [\"serve\"] }"
fi

# --- 4. pre-warm the embedding model (first run downloads weights) ------------
# Triggers the one-time Model2Vec download so the first real search is instant.
echo "==> Pre-warming the embedding model (one-time download)"
TMPWARM="$(mktemp -d)"
trap 'rm -rf "$TMPWARM"' EXIT
printf 'def warm():\n    """warm up the embedding model."""\n    return 1\n' > "$TMPWARM/warm.py"
"$BIN" search "$TMPWARM" "warm up" --top-k 1 >/dev/null 2>&1 \
  && echo "    model ready." \
  || echo "    warmup search failed (model will download on first real use)."

echo
echo "Done. Restart Claude Code (or reload the window) to pick up the sensez MCP server."
echo "Then ask it to search docs, e.g.: search_docs path=$ROOT query=\"element comparer\""

# --- 5. optional: run the server directly (stdio smoke test) ------------------
if [ "${1:-}" = "--serve" ]; then
  echo
  echo "==> Starting sensez MCP server on stdio (Ctrl-C to stop)."
  echo "    It waits for JSON-RPC on stdin; this is just a liveness check."
  exec "$BIN" serve
fi
