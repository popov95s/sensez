"""Benchmark Reflexez against full and native changed-test execution.

The output contains aggregate counts and timings only. Paths, test names, source,
package names, and repository identity are never serialized.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import time
from dataclasses import asdict, dataclass
from pathlib import Path

from .models import Comparison, Measurement, comparisons


@dataclass(frozen=True)
class ReflexezPlan:
    selected_files: tuple[str, ...]
    discovered_tests: int


@dataclass(frozen=True)
class VitestReport:
    selected_files: int
    executed_tests: int


def main() -> int:
    args = parse_args()
    root = args.repository.resolve()
    changed = changed_sources(root, args.diff)
    plan, selection_seconds = reflexez_plan(args.sensez.resolve(), root, changed)
    selected = list(plan.selected_files)
    measurements = [
        run_vitest(root, "reflexez", selected, selection_seconds),
        run_vitest(root, "native_related", changed, 0.0, related=True),
    ]
    if args.full:
        measurements.append(run_vitest(root, "full", [], 0.0))
    report = {
        "schema": 1,
        "scope": {
            "changed_source_files": len(changed),
            "discovered_test_files": plan.discovered_tests,
            "reflexez_planned_files": len(selected),
            "selection_seconds": round(selection_seconds, 3),
        },
        "measurements": [asdict(item) for item in measurements],
        "comparisons": {
            name: asdict(comparison)
            for name, comparison in comparisons(measurements).items()
        },
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    return int(any(item.exit_code != 0 for item in measurements))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("repository", type=Path)
    parser.add_argument("--sensez", type=Path, required=True)
    parser.add_argument("--diff", default="HEAD~1..HEAD")
    parser.add_argument("--full", action="store_true")
    return parser.parse_args()


def changed_sources(root: Path, diff: str) -> list[str]:
    output = subprocess.run(
        ["git", "diff", "--name-only", diff],
        cwd=root,
        text=True,
        capture_output=True,
        check=True,
    ).stdout
    extensions = {".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx"}
    return [
        name
        for name in output.splitlines()
        if Path(name).suffix.lower() in extensions and (root / name).is_file()
    ]


def reflexez_plan(
    sensez: Path, root: Path, changed: list[str]
) -> tuple[ReflexezPlan, float]:
    command = [str(sensez), "reflexez", str(root), "--plan", "--json"]
    for path in changed:
        command.extend(["--changed-file", path])
    started = time.perf_counter()
    process = subprocess.run(command, cwd=root, text=True, capture_output=True, check=True)
    elapsed = time.perf_counter() - started
    value = json.loads(process.stdout)
    if not isinstance(value, dict):
        raise ValueError("Reflexez plan must be a JSON object")
    selected = value.get("selected")
    if not isinstance(selected, list):
        raise ValueError("Reflexez plan must contain selected files")
    files = tuple(
        str(item["file"])
        for item in selected
        if isinstance(item, dict) and "file" in item
    )
    return ReflexezPlan(files, int(value["discovered_tests"])), elapsed


def run_vitest(
    root: Path,
    mode: str,
    paths: list[str],
    overhead: float,
    related: bool = False,
) -> Measurement:
    vitest = min(root.glob("**/node_modules/.bin/vitest"), key=lambda path: len(path.parts))
    command = [str(vitest), "related" if related else "run", *paths, "--reporter=json"]
    env = dict(os.environ)
    env["NO_COLOR"] = "1"
    started = time.perf_counter()
    process = subprocess.run(command, cwd=root, text=True, capture_output=True, env=env)
    elapsed = time.perf_counter() - started + overhead
    report = parse_vitest_report(process.stdout)
    return Measurement(
        mode=mode,
        selected_files=report.selected_files,
        executed_tests=report.executed_tests,
        wall_seconds=round(elapsed, 3),
        exit_code=process.returncode,
    )


def parse_vitest_report(output: str) -> VitestReport:
    start = output.find("{")
    value = json.loads(output[start:]) if start >= 0 else {}
    if not isinstance(value, dict):
        return VitestReport(0, 0)
    files = value.get("testResults", [])
    if not isinstance(files, list):
        return VitestReport(0, 0)
    assertion_counts = (
        len(item.get("assertionResults", []))
        for item in files
        if isinstance(item, dict)
    )
    return VitestReport(len(files), sum(assertion_counts))


if __name__ == "__main__":
    raise SystemExit(main())

