"""Compare Reflexez with full pytest and warm pytest-testmon execution."""

from __future__ import annotations

import argparse
import json
import re
import statistics
import subprocess
import time
from dataclasses import asdict, dataclass
from pathlib import Path

from .models import Measurement, comparisons


@dataclass(frozen=True)
class ReflexezPlan:
    discovered_test_files: int
    selected_files: int


@dataclass(frozen=True)
class BenchmarkReport:
    schema: int
    target: dict[str, str]
    protocol: dict[str, object]
    scope: dict[str, int]
    testmon_baseline: dict[str, object]
    measurements: list[dict[str, object]]
    comparisons: dict[str, dict[str, float]]


def main() -> int:
    args = parse_args()
    root = args.repository.resolve()
    changed = root / args.changed_file
    original = changed.read_text()
    if args.before not in original:
        raise ValueError("--before text is absent or the fixture is already changed")
    pytest = root / args.pytest
    datafile = root / ".testmondata"
    if datafile.exists():
        raise ValueError("benchmark requires a checkout without .testmondata")
    try:
        warmup = execute(root, [str(pytest), "--testmon", "-q"])
        baseline = datafile.read_bytes()
        changed.write_text(original.replace(args.before, args.after, 1))
        plan = reflexez_plan(args.sensez.resolve(), root, args.changed_file)
        measurements = [
            median_run(
                root,
                "reflexez",
                reflexez_command(args.sensez.resolve(), root, args.changed_file),
                args.runs,
                plan.selected_files,
            ),
            median_run(root, "full", [str(pytest), "-q"], args.runs, None),
            median_testmon(root, pytest, datafile, baseline, args.runs),
        ]
        report = report_json(args, plan, warmup, measurements)
        print(json.dumps(asdict(report), indent=2, sort_keys=True))
        return int(any(item.exit_code != 0 for item in measurements))
    finally:
        changed.write_text(original)
        datafile.unlink(missing_ok=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("repository", type=Path)
    parser.add_argument("--sensez", type=Path, required=True)
    parser.add_argument("--changed-file", required=True)
    parser.add_argument("--before", required=True)
    parser.add_argument("--after", required=True)
    parser.add_argument("--pytest", type=Path, default=Path(".venv/bin/pytest"))
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--target", default="public-python-project")
    parser.add_argument("--commit", default="pinned")
    return parser.parse_args()


def reflexez_plan(sensez: Path, root: Path, changed: str) -> ReflexezPlan:
    command = [
        str(sensez), "reflexez", str(root), "--plan", "--json",
        "--changed-file", changed,
    ]
    value = json.loads(subprocess.run(
        command, cwd=root, text=True, capture_output=True, check=True
    ).stdout)
    if not isinstance(value, dict) or not isinstance(value.get("selected"), list):
        raise ValueError("invalid Reflexez plan")
    return ReflexezPlan(
        discovered_test_files=int(value["discovered_tests"]),
        selected_files=len(value["selected"]),
    )


def reflexez_command(sensez: Path, root: Path, changed: str) -> list[str]:
    return [
        str(sensez), "reflexez", str(root), "--changed-file", changed, "--", "-q"
    ]


def median_run(
    root: Path, mode: str, command: list[str], runs: int, files: int | None
) -> Measurement:
    samples = [execute(root, command) for _ in range(runs)]
    middle = sorted(samples, key=lambda item: item.wall_seconds)[len(samples) // 2]
    return Measurement(
        mode, files, middle.executed_tests, round(statistics.median(
            item.wall_seconds for item in samples
        ), 3), max(item.exit_code for item in samples)
    )


def median_testmon(
    root: Path, pytest: Path, datafile: Path, baseline: bytes, runs: int
) -> Measurement:
    samples = []
    for _ in range(runs):
        datafile.write_bytes(baseline)
        samples.append(execute(root, [str(pytest), "--testmon", "-q"]))
    middle = sorted(samples, key=lambda item: item.wall_seconds)[len(samples) // 2]
    return Measurement(
        "pytest_testmon", None, middle.executed_tests,
        round(statistics.median(item.wall_seconds for item in samples), 3),
        max(item.exit_code for item in samples),
    )


def execute(root: Path, command: list[str]) -> Measurement:
    started = time.perf_counter()
    process = subprocess.run(command, cwd=root, text=True, capture_output=True)
    elapsed = time.perf_counter() - started
    passed = re.search(r"(\d+) passed", process.stdout)
    executed = int(passed.group(1)) if passed else 0
    if process.returncode != 0:
        raise RuntimeError(process.stdout + process.stderr)
    return Measurement("sample", None, executed, elapsed, process.returncode)


def report_json(
    args: argparse.Namespace,
    plan: ReflexezPlan,
    warmup: Measurement,
    measurements: list[Measurement],
) -> BenchmarkReport:
    compared = comparisons(measurements)
    return BenchmarkReport(
        schema=1,
        target={"name": args.target, "commit": args.commit},
        protocol={
            "runs": args.runs,
            "statistic": "median",
            "python": "3.11",
            "pytest": pytest_version(args.repository.resolve() / args.pytest),
            "pytest_testmon": "2.2.0",
            "reflexez_selection_included": True,
        },
        scope={
            "changed_files": 1,
            "discovered_test_files": plan.discovered_test_files,
            "reflexez_planned_files": plan.selected_files,
        },
        testmon_baseline={
            **asdict(warmup),
            "mode": "pytest_testmon_baseline_build",
        },
        measurements=[asdict(item) for item in measurements],
        comparisons={name: asdict(value) for name, value in compared.items()},
    )


def pytest_version(executable: Path) -> str:
    process = subprocess.run(
        [str(executable), "--version"], text=True, capture_output=True, check=True
    )
    return process.stdout.strip().removeprefix("pytest ")


if __name__ == "__main__":
    raise SystemExit(main())

