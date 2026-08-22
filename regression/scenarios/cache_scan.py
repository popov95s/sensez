"""Cache-scenario command execution."""

from pathlib import Path
from typing import cast

from ..harness.commands import CommandEnvironment, run_json
from ..harness.models import RegressionRun
from .cache_models import ScanReport

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
