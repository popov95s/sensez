#!/usr/bin/env python3
"""Render a detailed benchmark report from a Sensez A/B results tree."""

from __future__ import annotations

import json
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


def load_runs(root: Path) -> list[dict[str, Any]]:
    runs = []
    for metrics_path in sorted(root.glob("*/*/run_*/metrics.json")):
        run_dir = metrics_path.parent
        runs.append(
            {
                "metrics": json.loads(metrics_path.read_text()),
                "agent": json.loads((run_dir / "agent.json").read_text()),
                "diff": json.loads((run_dir / "sensez_diff.json").read_text()),
            }
        )
    return runs


def usage(payload: dict[str, Any]) -> dict[str, int]:
    for line in reversed(payload.get("stdout", "").splitlines()):
        if '"usage":' not in line:
            continue
        try:
            return json.loads(line).get("usage", {})
        except Exception:
            return {}
    return {}


def summarize(runs: list[dict[str, Any]]) -> dict[str, Any]:
    by_variant = defaultdict(list)
    smell_kinds = defaultdict(Counter)
    for run in runs:
        variant = run["metrics"]["variant"]
        by_variant[variant].append(run)
        for smell in run["diff"].get("json", {}).get("smells", []):
            smell_kinds[variant][smell.get("kind", "unknown")] += 1
    return {"by_variant": by_variant, "smell_kinds": smell_kinds}


def pillar_totals(runs: list[dict[str, Any]], variant: str) -> dict[str, int]:
    totals = Counter()
    for run in runs:
        if run["metrics"]["variant"] != variant:
            continue
        for pillar, count in run["metrics"]["sensez_diff"].items():
            totals[pillar] += count
    return dict(totals)


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: report.py <results-dir>")
    root = Path(sys.argv[1])
    runs = load_runs(root)
    summary = summarize(runs)

    print("# Sensez A/B Report\n")
    print("## Totals by Variant\n")
    print("| variant | runs | input_tokens | output_tokens | reasoning_tokens | quality_score | diff_findings |")
    print("| --- | --- | --- | --- | --- | --- | --- |")
    for variant, variant_runs in sorted(summary["by_variant"].items()):
        inputs = outputs = reasoning = quality = diffs = 0
        for run in variant_runs:
            u = usage(run["agent"])
            inputs += u.get("input_tokens", 0)
            outputs += u.get("output_tokens", 0)
            reasoning += u.get("reasoning_output_tokens", 0)
            quality += run["metrics"].get("quality_regression_score", 0)
            diffs += run["metrics"]["sensez_diff"]["total"]
        print(f"| {variant} | {len(variant_runs)} | {inputs} | {outputs} | {reasoning} | {quality} | {diffs} |")

    print("\n## Diff Findings by Pillar\n")
    print("| variant | cycles | dead_code | boundaries | duplication | smells |")
    print("| --- | --- | --- | --- | --- | --- |")
    for variant in sorted(summary["by_variant"]):
        totals = pillar_totals(runs, variant)
        print(
            f"| {variant} | {totals.get('cycles', 0)} | {totals.get('dead_code', 0)} | "
            f"{totals.get('boundaries', 0)} | {totals.get('duplication', 0)} | {totals.get('smells', 0)} |"
        )

    print("\n## Top Smell Kinds\n")
    print("| variant | smell_kind | count |")
    print("| --- | --- | --- |")
    for variant, counter in sorted(summary["smell_kinds"].items()):
        for smell_kind, count in counter.most_common(10):
            print(f"| {variant} | {smell_kind} | {count} |")


if __name__ == "__main__":
    main()
