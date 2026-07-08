#!/usr/bin/env python3
"""Create a writable Codex home for nested eval agents."""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("target", type=Path)
    parser.add_argument("--source", type=Path, default=Path.home() / ".codex")
    parser.add_argument("--sense-bin", required=True)
    return parser.parse_args()


def copy_if_present(source: Path, target: Path) -> None:
    if source.exists():
        shutil.copy2(source, target)


def main() -> None:
    args = parse_args()
    args.target.mkdir(parents=True, exist_ok=True)

    auth = args.source / "auth.json"
    if not auth.exists():
        raise SystemExit(f"missing Codex auth file: {auth}")

    shutil.copy2(auth, args.target / "auth.json")
    copy_if_present(args.source / "installation_id", args.target / "installation_id")

    config = f"""[mcp_servers.sense]
command = "{args.sense_bin}"
args = ["mcp", "serve"]
"""
    (args.target / "config.toml").write_text(config)


if __name__ == "__main__":
    main()
