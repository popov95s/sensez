"""Repository-level cache regressions.

These edits are applied to real pinned Flask/Zod checkouts. The scenario keeps
the assertions intentionally small: the complete reports remain covered by the
normal full-scan baselines, while this file proves cache reuse does not hide
cross-file dependency effects.
"""

from pathlib import Path

from ..harness.artifacts import dump_normalized
from ..harness.commands import CommandEnvironment
from ..harness.models import RegressionRun
from ..harness.repositories import cleanup_repo, scenario_repo
from .cache_models import ScanReport, ScenarioFiles
from .cache_raw import dump_raw_cache_outputs
from .cache_scan import scan, timed_scan, wait_for_parse_cache


def run_cache_impact_scenario(context: RegressionRun) -> None:
    repo = scenario_repo(context.cache, context.target)
    files = _files(context.target["profile"])
    try:
        _enable_scenario_entrypoints(repo, context.target["profile"])
        _write_sources(repo, files)
        disabled = scan(context, repo, threshold=8)
        cache = repo / ".sensez/parse-v2.bin"
        assert not cache.exists(), "disabled cache created a parse cache"
        config_path = repo / "sensez.toml"
        original_config = config_path.read_text()
        cached_config = original_config + "\n[cache]\nenabled = true\n"
        config_path.write_text(cached_config)
        legacy = repo / ".sensez/parse-v1"
        legacy.mkdir(parents=True)
        (legacy / "obsolete.bin").write_bytes(b"obsolete")
        initial = scan(context, repo, threshold=8)
        incremental_cache = wait_for_parse_cache(repo)
        assert incremental_cache.exists(), "initial scan did not create incremental cache state"
        assert initial == disabled, "enabling cache changed cold scan output"
        if incremental_cache.is_file():
            assert incremental_cache.stat().st_size <= 1_000_000, "parse cache exceeded 1MB"
        assert not legacy.exists(), "legacy parse cache was not removed"
        initial_cache = cache.read_bytes() if cache.is_file() else None
        initial_mtime = cache.stat().st_mtime_ns if cache.is_file() else None
        repeated = scan(context, repo, threshold=8)
        assert initial == repeated, "repeated cached repository scan changed output"
        if initial_cache is not None:
            assert cache.read_bytes() == initial_cache, "unchanged scan rewrote parse cache"
            assert cache.stat().st_mtime_ns == initial_mtime, "unchanged scan rewrote parse cache"

        cache.write_bytes(b"corrupt parse cache")
        overridden = scan(
            context,
            repo,
            threshold=8,
            env=CommandEnvironment((("SENSEZ_ANALYSIS_CACHE", "0"),)),
        )
        assert overridden == initial, "disabled environment override changed output"
        assert cache.read_bytes() == b"corrupt parse cache", (
            "disabled environment override accessed the parse cache"
        )
        recovered = scan(context, repo, threshold=8)
        assert recovered == initial, "corrupt snapshot recovery changed output"
        assert cache.read_bytes().startswith(b"\x1f\x8b"), "corrupt parse cache was not replaced"

        _write(repo / files.duplicate_right, files.duplicate_body)
        duplicate, reuse = timed_scan(context, repo, threshold=8)
        assert reuse["modified"] == 1, f"expected one modified file, observed {reuse}"
        assert reuse["reused"] > 0, f"one-file edit reused no parsed files: {reuse}"
        cache.unlink()
        if incremental_cache.is_file():
            incremental_cache.unlink()
        duplicate_cold = scan(context, repo, threshold=8)
        assert duplicate == duplicate_cold, (
            "source edit produced different cached and cold reports"
        )
        assert _has_cross_file_duplicate(duplicate, files), (
            "duplicate introduced in one file was not detected in the other; "
            f"observed {_snapshot(duplicate, files)['duplication']}"
        )
        config_path.write_text(cached_config + "\n[duplication]\nthreshold = 500\n")
        config_changed = scan(context, repo)
        assert not _has_cross_file_duplicate(config_changed, files), (
            "config change reused an incompatible snapshot"
        )
        config_path.write_text(cached_config)
        config_restored = scan(context, repo)
        assert _has_cross_file_duplicate(config_restored, files), (
            "restored config did not recompute duplicate output"
        )

        (repo / files.duplicate_right).unlink()
        deleted = scan(context, repo, threshold=8)
        assert not _has_cross_file_duplicate(deleted, files), (
            "deleted source remained in the cached report"
        )
        _write(repo / files.duplicate_right, files.duplicate_unique)
        restored_source = scan(context, repo, threshold=8)

        _write(repo / files.cycle_b, files.cycle_b_changed_body)
        cycle = scan(context, repo)
        assert _has_cycle(cycle, files), "cross-file cycle introduced by edit was missed"
        cache.unlink()
        diff_cold = scan(context, repo, diff=True)
        diff_cached = scan(context, repo, diff=True)
        assert diff_cold == diff_cached, "cached diff report changed output"

        dump_raw_cache_outputs(context, repo, cache)

        result = {
            "initial": _snapshot(initial, files),
            "disabled": _snapshot(disabled, files),
            "environment_disabled": _snapshot(overridden, files),
            "repeated": _snapshot(repeated, files),
            "corrupt_recovery": _snapshot(recovered, files),
            "duplicate_added": _snapshot(duplicate, files),
            "incremental_parse": reuse,
            "config_changed": _snapshot(config_changed, files),
            "config_restored": _snapshot(config_restored, files),
            "source_deleted": _snapshot(deleted, files),
            "source_restored": _snapshot(restored_source, files),
            "cycle_added": _snapshot(cycle, files),
            "diff_cold": _snapshot(diff_cold, files),
            "diff_cached": _snapshot(diff_cached, files),
        }
        dump_normalized(
            context.out / "cache.impact.json",
            result,
            repo,
            context.target,
        )
    finally:
        cleanup_repo(repo)


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


def _write_sources(repo: Path, files: ScenarioFiles) -> None:
    initial_sources = (
        (files.duplicate_left, files.duplicate_left_body),
        (files.duplicate_right, files.duplicate_right_body),
        (files.cycle_a, files.cycle_a_body),
        (files.cycle_b, files.cycle_b_initial_body),
        (files.provider, files.provider_body),
        (files.consumer, files.consumer_body),
    )
    for path, source in initial_sources:
        _write(repo / path, source)


def _has_cross_file_duplicate(report: ScanReport, files: ScenarioFiles) -> bool:
    expected = {
        Path(files.duplicate_left).name,
        Path(files.duplicate_right).name,
    }
    return any(
        expected.issubset({Path(item["file"]).name for item in clone["occurrences"]})
        for clone in report.get("duplication", [])
    )


def _snapshot(report: ScanReport, files: ScenarioFiles) -> ScanReport:
    names = {
        Path(path).name
        for path in (files.duplicate_left, files.duplicate_right, files.cycle_a, files.cycle_b)
    }
    duplication = [
        item
        for item in report.get("duplication", [])
        if any(Path(occurrence["file"]).name in names for occurrence in item.get("occurrences", []))
    ]
    cycle_names = {files.cycle_a_module, files.cycle_b_module}
    cycles = [
        item
        for item in report.get("cycles", [])
        if any(
            any(module.endswith(name) for module in item.get("modules", []))
            for name in cycle_names
        )
    ]
    return {"duplication": duplication, "cycles": cycles}


def _has_cycle(report: ScanReport, files: ScenarioFiles) -> bool:
    expected = {files.cycle_a_module, files.cycle_b_module}
    return any(
        all(any(module.endswith(name) for module in cycle.get("modules", [])) for name in expected)
        for cycle in report.get("cycles", [])
    )


def _files(profile: str) -> ScenarioFiles:
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

    return ScenarioFiles(
        duplicate_left=base + duplicate_left,
        duplicate_right=base + duplicate_right,
        duplicate_left_body=duplicate_body,
        duplicate_right_body=duplicate_unique,
        duplicate_body=duplicate_body,
        duplicate_unique=duplicate_unique,
        cycle_a=base + cycle_a_name,
        cycle_b=base + cycle_b_name,
        cycle_a_module="flask.sensez_cache_cycle_a" if profile == "py" else "sensez-cache-cycle-a",
        cycle_b_module="flask.sensez_cache_cycle_b" if profile == "py" else "sensez-cache-cycle-b",
        cycle_a_body=cycle_a,
        cycle_b_initial_body=cycle_b,
        cycle_b_changed_body=cycle_import
        + "\n"
        + (
            "def cycle_value():\n    return cycle_a()\n"
            if profile == "py"
            else "export function cycleValue(): number { return cycleA(); }\n"
        ),
        provider=base + f"sensez-cache-provider{ext}",
        provider_body=provider,
        provider_symbol="sensez_cache_live" if profile == "py" else "sensezCacheLive",
        consumer=base + f"sensez-cache-consumer{ext}",
        consumer_body=consumer_live,
        consumer_other=consumer_other,
    )
