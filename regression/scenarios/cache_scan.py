"""Cache-scenario command execution and stable telemetry extraction."""

import json
import re
import time
from pathlib import Path
from typing import cast

from ..harness.commands import CommandEnvironment, run_captured, run_json
from ..harness.models import RegressionRun
from .cache_models import ScanReport


def wait_for_parse_cache(repo: Path, timeout: float = 30.0) -> Path:
    cache = repo / ".sensez/parse-v2.bin"
    deadline = time.monotonic() + timeout
    while not cache.is_file() and time.monotonic() < deadline:
        time.sleep(0.02)
    assert cache.is_file(), "incremental cache did not become ready"
    return cache


def scan(
    context: RegressionRun,
    repo: Path,
    threshold: int | None = None,
    diff: bool = False,
    env: CommandEnvironment | None = None,
) -> ScanReport:
    environment = (
        CommandEnvironment((("SENSEZ_ANALYSIS_CACHE", ""),))
        if env is None
        else env
    )
    options = ["--all"]
    if diff:
        options.append("--diff")
    if threshold is not None:
        options.extend(["--threshold", str(threshold)])
    return cast(
        ScanReport,
        run_json(
            [context.sensez, "noze", str(repo), *options, "--json"],
            repo,
            env=environment,
        ),
    )


def timed_scan(
    context: RegressionRun,
    repo: Path,
    threshold: int,
) -> tuple[ScanReport, dict[str, int]]:
    # Exercise the persisted parse cache directly.
    output = run_captured(
        [
            context.sensez,
            "noze",
            str(repo),
            "--all",
            "--threshold",
            str(threshold),
            "--json",
        ],
        repo,
        env=CommandEnvironment(
            (("SENSEZ_ANALYSIS_CACHE", ""), ("SENSEZ_TIMING", "1"))
        ),
    )
    assert output.returncode == 0, output.stderr
    match = re.search(
        r"parse-cache reused=(\d+)/(\d+) bytes=(\d+)/(\d+) "
        r"added=(\d+) modified=(\d+) "
        r"deleted=(\d+) unchanged=(\d+)",
        output.stderr,
    )
    assert match is not None, f"missing incremental cache telemetry: {output.stderr}"
    keys = (
        "reused",
        "total",
        "reused_bytes",
        "total_bytes",
        "added",
        "modified",
        "deleted",
        "unchanged",
    )
    return cast(ScanReport, json.loads(output.stdout)), dict(
        zip(keys, (int(value) for value in match.groups()), strict=True)
    )
