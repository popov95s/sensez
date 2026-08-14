"""Raw CLI snapshots for semantic cross-file cache regressions."""

from pathlib import Path

from ..harness.artifacts import dump_normalized_text
from ..harness.commands import CommandEnvironment, CommandOutput, run_captured
from ..harness.models import RegressionRun


def dump_raw_cache_output(
    context: RegressionRun,
    repo: Path,
    state: str,
) -> None:
    output = _run(context, repo, "--cycles", "--all", "--json")
    assert output.returncode == 0, f"raw {state} scan failed"
    assert not output.stderr, f"raw {state} scan wrote to stderr"
    _write(context, repo, f"cache.raw-{state}.json", output)


def _run(
    context: RegressionRun,
    repo: Path,
    *options: str,
) -> CommandOutput:
    return run_captured(
        [context.sensez, "noze", str(repo), *options],
        repo,
        env=CommandEnvironment((("SENSEZ_ANALYSIS_CACHE", ""),)),
    )


def _write(
    context: RegressionRun,
    repo: Path,
    name: str,
    output: CommandOutput,
) -> None:
    dump_normalized_text(context.out / name, output.stdout, repo, context.target)
