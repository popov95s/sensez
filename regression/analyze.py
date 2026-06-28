from __future__ import annotations

import argparse
import difflib
import filecmp
import shutil
from pathlib import Path


def compare_tree(results: Path, baselines: Path) -> tuple[str, ...]:
    failures: list[str] = []
    for result in sorted(results.rglob("*.json")):
        rel = result.relative_to(results)
        baseline = baselines / rel
        if not baseline.exists():
            failures.append(f"missing baseline: {rel}")
            continue
        if not filecmp.cmp(result, baseline, shallow=False):
            failures.append(f"changed baseline: {rel}\n{_diff(baseline, result)}")
    for baseline in sorted(baselines.rglob("*.json")):
        rel = baseline.relative_to(baselines)
        if not (results / rel).exists():
            failures.append(f"missing result for baseline: {rel}")
    return tuple(failures)


def accept_tree(results: Path, baselines: Path) -> None:
    baselines.mkdir(parents=True, exist_ok=True)
    for result in sorted(results.rglob("*.json")):
        rel = result.relative_to(results)
        dest = baselines / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(result, dest)


def _diff(expected: Path, actual: Path) -> str:
    before = expected.read_text().splitlines()
    after = actual.read_text().splitlines()
    lines = difflib.unified_diff(
        before,
        after,
        fromfile=str(expected),
        tofile=str(actual),
        lineterm="",
        n=3,
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=Path)
    parser.add_argument("baselines", type=Path)
    parser.add_argument("--accept", action="store_true")
    args = parser.parse_args()
    if args.accept:
        accept_tree(args.results, args.baselines)
        return 0
    failures = compare_tree(args.results, args.baselines)
    if failures:
        print("\n\n".join(failures))
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
