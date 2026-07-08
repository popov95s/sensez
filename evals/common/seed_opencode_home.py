#!/usr/bin/env python3
"""Create opencode config directories for control vs Sensez eval variants.

Uses XDG_CONFIG_HOME to point opencode at a temp config directory so the
control and sensez variants can have different MCP server configurations
without interfering with each other or the user's real config.
"""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("target_parent", type=Path)
    parser.add_argument("--sense-bin", required=True)
    parser.add_argument(
        "--source-config",
        type=Path,
        default=Path.home() / ".config" / "opencode",
    )
    parser.add_argument(
        "--source-data",
        type=Path,
        default=Path.home() / ".local" / "share" / "opencode",
    )
    return parser.parse_args()


def copy_file_if_present(src: Path, dst: Path) -> None:
    if src.is_file():
        shutil.copy2(src, dst)


def write_control_config(config_dir: Path) -> None:
    config_dir.mkdir(parents=True, exist_ok=True)
    config = {
        "$schema": "https://opencode.ai/config.json",
    }
    (config_dir / "opencode.jsonc").write_text(json.dumps(config, indent=2) + "\n")


def write_sensez_config(config_dir: Path, sense_bin: str) -> None:
    config_dir.mkdir(parents=True, exist_ok=True)
    config = {
        "$schema": "https://opencode.ai/config.json",
        "mcp": {
            "sensez": {
                "type": "local",
                "command": [sense_bin, "mcp", "serve"],
                "enabled": True,
            }
        },
    }
    (config_dir / "opencode.jsonc").write_text(json.dumps(config, indent=2) + "\n")


def copy_auth(data_dir: Path, source_data: Path) -> None:
    data_dir.mkdir(parents=True, exist_ok=True)
    auth_src = source_data / "auth.json"
    if not auth_src.is_file():
        raise SystemExit(f"missing opencode auth file: {auth_src}")
    shutil.copy2(auth_src, data_dir / "auth.json")


def main() -> None:
    args = parse_args()

    control_config = args.target_parent / "control" / "config" / "opencode"
    control_data = args.target_parent / "control" / "data" / "opencode"
    sensez_config = args.target_parent / "sensez" / "config" / "opencode"
    sensez_data = args.target_parent / "sensez" / "data" / "opencode"

    write_control_config(control_config)
    write_sensez_config(sensez_config, args.sense_bin)

    copy_auth(control_data, args.source_data)
    copy_auth(sensez_data, args.source_data)

    print(f"Control config: {control_config}")
    print(f"Sensez config:  {sensez_config}")


if __name__ == "__main__":
    main()
