#!/usr/bin/env python3
"""Export collected patches as SWE-PolyBench predictions JSONL."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("results_dir", type=Path)
    parser.add_argument("--variant", required=True, choices=["control", "sensez"])
    parser.add_argument("--run", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    rows = []
    pattern = f"*/{args.variant}/run_{args.run}/patch.diff"
    for patch_path in sorted(args.results_dir.glob(pattern)):
        task_id = patch_path.parents[2].name
        rows.append({"instance_id": task_id, "model_patch": patch_path.read_text()})
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w") as handle:
        for row in rows:
            handle.write(json.dumps(row) + "\n")


if __name__ == "__main__":
    main()
