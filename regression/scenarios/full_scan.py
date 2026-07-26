from ..harness.artifacts import dump_normalized
from ..harness.commands import run_json
from ..harness.models import RegressionRun
from ..harness.paths import ROOT
from ..harness.repositories import cleanup_repo, scenario_repo


def run_full_scans(context: RegressionRun) -> None:
    repo = scenario_repo(context.cache, context.target)
    try:
        scans = (
            ("default.noze.json", []),
            ("default.noze.max5.json", ["--max", "5"]),
            ("full.noze.json", ["--all"]),
            (
                "full.noze.threshold40.json",
                ["--all", "--threshold", "40"],
            ),
            ("full.noze.max5.json", ["--all", "--max", "5"]),
        )
        for artifact, options in scans:
            report = run_json(
                [
                    context.sensez,
                    "noze",
                    str(repo),
                    *options,
                    "--json",
                ],
                ROOT,
            )
            dump_normalized(
                context.out / artifact,
                report,
                repo,
                context.target,
            )
    finally:
        cleanup_repo(repo)
