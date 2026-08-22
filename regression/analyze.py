from __future__ import annotations

import argparse
import difflib
import json
import shutil
from pathlib import Path


def compare_tree(
    results: Path, baselines: Path, require_complete: bool = True
) -> tuple[str, ...]:
    failures: list[str] = []
    for result in _artifacts(results):
        rel = result.relative_to(results)
        baseline = baselines / rel
        if not baseline.exists():
            failures.append(f"missing baseline: {rel}")
            continue
        if not same_artifact(result, baseline):
            failures.append(f"changed baseline: {rel}\n{_diff(baseline, result)}")
    if require_complete:
        for baseline in _artifacts(baselines):
            rel = baseline.relative_to(baselines)
            if not (results / rel).exists():
                failures.append(f"missing result for baseline: {rel}")
    return tuple(failures)


def same_artifact(left: Path, right: Path) -> bool:
    if left.name.startswith("cache.raw-"):
        return left.read_bytes() == right.read_bytes()
    return same_json(left, right)


def same_json(left: Path, right: Path) -> bool:
    """Compare JSON artifacts by value, ignoring whitespace and key ordering."""
    try:
        return json.loads(left.read_text()) == json.loads(right.read_text())
    except json.JSONDecodeError:
        return False


def accept_tree(results: Path, baselines: Path) -> None:
    baselines.mkdir(parents=True, exist_ok=True)
    for result in _artifacts(results):
        rel = result.relative_to(results)
        dest = baselines / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(result, dest)


def _diff(expected: Path, actual: Path) -> str:
    before = _artifact_lines(expected)
    after = _artifact_lines(actual)
    lines = difflib.unified_diff(
        before,
        after,
        fromfile=str(expected),
        tofile=str(actual),
        lineterm="",
        n=3,
    )
    return "\n".join(lines)


def _artifact_lines(path: Path) -> list[str]:
    if path.name.startswith("cache.raw-"):
        return path.read_text().splitlines()
    try:
        value = json.loads(path.read_text())
    except json.JSONDecodeError:
        return path.read_text().splitlines()
    return json.dumps(value, indent=2, sort_keys=True).splitlines()


def _artifacts(root: Path) -> list[Path]:
    return sorted(path for path in root.rglob("*") if path.is_file())


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
