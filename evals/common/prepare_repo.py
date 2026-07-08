#!/usr/bin/env python3
"""Clone a repository at a fixed commit into a clean workspace."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("repo")
    parser.add_argument("base_commit")
    parser.add_argument("workspace", type=Path)
    parser.add_argument("task_id", nargs="?")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.repo == "local/tinyforms":
        if not args.task_id:
            raise SystemExit("local/tinyforms preparation requires task_id")
        script = Path(__file__).resolve().parents[1] / "tinyforms_ab" / "prepare_repo.py"
        subprocess.run(["python3", str(script), args.task_id, str(args.workspace)], check=True)
        return

    if args.workspace.exists():
        shutil.rmtree(args.workspace)
    args.workspace.parent.mkdir(parents=True, exist_ok=True)
    repo_name = args.repo.rsplit("/", 1)[-1]
    cache_root = Path(os.environ.get("SWE_POLYBENCH_REPO_CACHE", "/private/tmp/swe_polybench_repos"))
    local_source = cache_root / repo_name
    if local_source.exists():
        subprocess.run(["git", "clone", str(local_source), str(args.workspace)], check=True)
    else:
        subprocess.run(["git", "clone", f"https://github.com/{args.repo}.git", str(args.workspace)], check=True)
    subprocess.run(["git", "-C", str(args.workspace), "checkout", args.base_commit], check=True)


if __name__ == "__main__":
    main()
