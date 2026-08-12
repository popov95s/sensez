"""Golden snapshots of unparsed CLI output from cold and cached scans."""

from pathlib import Path

from ..harness.artifacts import dump_normalized_text
from ..harness.commands import CommandEnvironment, CommandOutput, run_captured
from ..harness.models import RegressionRun


def dump_raw_cache_outputs(
    context: RegressionRun,
    repo: Path,
    cache: Path,
) -> None:
    cache.unlink(missing_ok=True)
    cache.with_name("parse-v2.bin").unlink(missing_ok=True)
    cold = _run(context, repo, "--cycles", "--all", "--json")
    assert cold.returncode == 0, "cold raw scan failed"
    assert cache.is_file(), "cold raw scan did not create a snapshot"

    warm = _run(context, repo, "--cycles", "--all", "--json")
    assert warm.returncode == 0, "warm raw scan failed"
    assert warm == cold, "cached scan changed raw process output"

    terminal = _run(context, repo, "--cycles", "--all")
    assert terminal.returncode == 0, "human-readable cached scan failed"

    gate = _run(
        context,
        repo,
        "--diff",
        "--cycles",
        "--all",
        "--json",
        "--fail-on-new",
        "warning",
    )
    assert gate.returncode == 1, "raw fail-on-new scan did not block"
    assert not any(
        output.stderr for output in (cold, warm, terminal, gate)
    ), "raw cache scans unexpectedly wrote to stderr"

    _write(context, repo, "cache.raw-cold.json", cold)
    _write(context, repo, "cache.raw-warm.json", warm)
    _write(context, repo, "cache.raw-terminal.txt", terminal)
    _write(context, repo, "cache.raw-fail-on-new.json", gate)


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
