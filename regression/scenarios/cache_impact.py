"""Repository-level cache regressions.

These edits are applied to real pinned Flask/Zod checkouts. The scenario keeps
the assertions intentionally small: the complete reports remain covered by the
normal full-scan baselines, while this file proves cache reuse does not hide
cross-file dependency effects.
"""

from pathlib import Path

from ..harness.artifacts import dump_normalized
from ..harness.commands import run_json
from ..harness.models import RegressionRun
from ..harness.repositories import cleanup_repo, scenario_repo


def run_cache_impact_scenario(context: RegressionRun) -> None:
    repo = scenario_repo(context.cache, context.target)
    files = _files(context.target["profile"])
    try:
        _enable_scenario_entrypoints(repo, context.target["profile"])
        _write_sources(repo, files)
        initial = _scan(context, repo, threshold=8)
        repeated = _scan(context, repo, threshold=8)
        assert initial == repeated, "repeated cached repository scan changed output"

        _write(repo / files["duplicate_right"], files["duplicate_body"])
        duplicate = _scan(context, repo, threshold=8)
        assert _has_cross_file_duplicate(duplicate, files), (
            "duplicate introduced in one file was not detected in the other"
        )

        _write(repo / files["duplicate_right"], files["duplicate_unique"])
        removed = _scan(context, repo, threshold=8)
        assert not _has_cross_file_duplicate(removed, files), (
            "removed duplicate remained in the cached report"
        )

        _write(repo / files["cycle_b"], files["cycle_b_changed_body"])
        cycle = _scan(context, repo)
        assert _has_cycle(cycle, files), "cross-file cycle introduced by edit was missed"

        result = {
            "initial": _snapshot(initial, files),
            "repeated": _snapshot(repeated, files),
            "duplicate_added": _snapshot(duplicate, files),
            "duplicate_removed": _snapshot(removed, files),
            "cycle_added": _snapshot(cycle, files),
        }
        dump_normalized(
            context.out / "cache.impact.json",
            result,
            repo,
            context.target,
        )
    finally:
        cleanup_repo(repo)


def _scan(context: RegressionRun, repo: Path, threshold: int | None = None) -> dict:
    options = ["--all"]
    if threshold is not None:
        options.extend(["--threshold", str(threshold)])
    return run_json(
        [context.sensez, "noze", str(repo), *options, "--json"],
        repo,
    )


def _write(path: Path, text: str) -> None:
    path.write_text(text)


def _enable_scenario_entrypoints(repo: Path, profile: str) -> None:
    suffix = "py" if profile == "py" else "ts"
    config = repo / "sensez.toml"
    text = config.read_text()
    marker = "entry_points = [\n"
    replacement = marker + (
        f'  "**/sensez_cache_cycle_a.{suffix}", "**/sensez-cache-cycle-a.{suffix}",\n'
        f'  "**/sensez_cache_consumer.{suffix}", "**/sensez-cache-consumer.{suffix}",\n'
    )
    config.write_text(text.replace(marker, replacement, 1))


def _write_sources(repo: Path, files: dict[str, str]) -> None:
    initial_sources = {
        "duplicate_left": files["duplicate_left_body"],
        "duplicate_right": files["duplicate_right_body"],
        "cycle_a": files["cycle_a_body"],
        "cycle_b": files["cycle_b_initial_body"],
        "provider": files["provider_body"],
        "consumer": files["consumer_body"],
    }
    for name, source in initial_sources.items():
        _write(repo / files[name], source)


def _has_cross_file_duplicate(report: dict, files: dict[str, str]) -> bool:
    expected = {
        Path(files["duplicate_left"]).name,
        Path(files["duplicate_right"]).name,
    }
    return any(
        expected.issubset({Path(item["file"]).name for item in clone["occurrences"]})
        for clone in report.get("duplication", [])
    )


def _snapshot(report: dict, files: dict[str, str]) -> dict:
    names = {Path(files[key]).name for key in ("duplicate_left", "duplicate_right", "cycle_a", "cycle_b")}
    duplication = [
        item
        for item in report.get("duplication", [])
        if any(Path(occurrence["file"]).name in names for occurrence in item.get("occurrences", []))
    ]
    cycle_names = {files["cycle_a_module"], files["cycle_b_module"]}
    cycles = [
        item
        for item in report.get("cycles", [])
        if any(
            any(module.endswith(name) for module in item.get("modules", []))
            for name in cycle_names
        )
    ]
    return {"duplication": duplication, "cycles": cycles}


def _has_cycle(report: dict, files: dict[str, str]) -> bool:
    expected = {files["cycle_a_module"], files["cycle_b_module"]}
    return any(
        all(any(module.endswith(name) for module in cycle.get("modules", [])) for name in expected)
        for cycle in report.get("cycles", [])
    )


def _files(profile: str) -> dict[str, str]:
    if profile == "py":
        ext = ".py"
        base = "src/flask/"
        duplicate_left = "sensez_cache_duplicate_left.py"
        duplicate_right = "sensez_cache_duplicate_right.py"
        cycle_a_name = "sensez_cache_cycle_a.py"
        cycle_b_name = "sensez_cache_cycle_b.py"
        cycle_import = "from flask.sensez_cache_cycle_a import cycle_a"
        consumer_live = "from flask.sensez_cache_provider import sensez_cache_live\nprint(sensez_cache_live())\n"
        consumer_other = "from flask.sensez_cache_provider import sensez_cache_other\nprint(sensez_cache_other())\n"
    else:
        ext = ".ts"
        base = "packages/zod/src/"
        duplicate_left = "sensez-cache-duplicate-left.ts"
        duplicate_right = "sensez-cache-duplicate-right.ts"
        cycle_a_name = "sensez-cache-cycle-a.ts"
        cycle_b_name = "sensez-cache-cycle-b.ts"
        cycle_import = 'import { cycleA } from "./sensez-cache-cycle-a";'
        consumer_live = 'import { sensezCacheLive } from "./sensez-cache-provider";\nconsole.log(sensezCacheLive());\n'
        consumer_other = 'import { sensezCacheOther } from "./sensez-cache-provider";\nconsole.log(sensezCacheOther());\n'

    if profile == "py":
        duplicate_body = (
            "def sensez_cache_duplicate_left(value):\n"
            + "".join(
                f"    value = cache_module.step_{index}(value)\n"
                for index in range(12)
            )
            + "    return value\n"
        )
        duplicate_unique = "def sensez_cache_duplicate_right(value):\n    return value + 1\n"
        cycle_a = "from flask.sensez_cache_cycle_b import cycle_value\n\ndef cycle_a():\n    return cycle_value()\n"
        cycle_b = "def cycle_value():\n    return 1\n"
        provider = "def sensez_cache_live():\n    return 1\n\ndef sensez_cache_other():\n    return 2\n"
    else:
        duplicate_body = (
            "export function sensezCacheDuplicateLeft(value: number): number {\n"
            + "".join(
                f"  value = cacheModule.step{index}(value);\n"
                for index in range(12)
            )
            + "  return value;\n}\n"
        )
        duplicate_unique = "export function sensezCacheDuplicateRight(value: number): number { return value + 1; }\n"
        cycle_a = 'import { cycleValue } from "./sensez-cache-cycle-b";\nexport function cycleA(): number { return cycleValue(); }\n'
        cycle_b = "export function cycleValue(): number { return 1; }\n"
        provider = "export function sensezCacheLive(): number { return 1; }\nexport function sensezCacheOther(): number { return 2; }\n"

    return {
        "duplicate_left": base + duplicate_left,
        "duplicate_right": base + duplicate_right,
        "duplicate_left_body": duplicate_body,
        "duplicate_right_body": duplicate_unique,
        "duplicate_body": duplicate_body,
        "duplicate_unique": duplicate_unique,
        "cycle_a": base + cycle_a_name,
        "cycle_b": base + cycle_b_name,
        "cycle_a_module": "flask.sensez_cache_cycle_a" if profile == "py" else "sensez-cache-cycle-a",
        "cycle_b_module": "flask.sensez_cache_cycle_b" if profile == "py" else "sensez-cache-cycle-b",
        "cycle_a_body": cycle_a,
        "cycle_b_initial_body": cycle_b,
        "cycle_b_changed_body": cycle_import
        + "\n"
        + (
            "def cycle_value():\n    return cycle_a()\n"
            if profile == "py"
            else "export function cycleValue(): number { return cycleA(); }\n"
        ),
        "provider": base + f"sensez-cache-provider{ext}",
        "provider_body": provider,
        "provider_symbol": "sensez_cache_live" if profile == "py" else "sensezCacheLive",
        "consumer": base + f"sensez-cache-consumer{ext}",
        "consumer_body": consumer_live,
        "consumer_other": consumer_other,
    }
