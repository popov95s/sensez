from collections.abc import Callable

from ..branch_metrics.repository import branch_metric_repo
from ..branch_metrics.scenarios import (
    colleague_main_issue_is_not_reintroduced,
    detached_scan_does_not_change_transitions,
    return_to_fixed_branch_stays_resolved,
    same_branch_revert_is_reintroduced,
)
from ..harness.artifacts import dump_normalized
from ..harness.models import FixtureSession, RegressionRun
from ..harness.repositories import cleanup_repo
from ..mcp_client import McpClient


BranchScenario = Callable[[FixtureSession], object]


def run_branch_metric_scenarios(context: RegressionRun) -> None:
    scenarios: tuple[tuple[str, BranchScenario], ...] = (
        (
            "brainz.branch-colleague-main.json",
            colleague_main_issue_is_not_reintroduced,
        ),
        (
            "brainz.branch-return-feature-fixed.json",
            return_to_fixed_branch_stays_resolved,
        ),
        (
            "brainz.branch-same-branch-revert.json",
            same_branch_revert_is_reintroduced,
        ),
        (
            "brainz.branch-detached-scan.json",
            detached_scan_does_not_change_transitions,
        ),
    )
    for artifact, scenario in scenarios:
        _run_case(context, artifact, scenario)


def _run_case(
    context: RegressionRun,
    artifact: str,
    scenario: BranchScenario,
) -> None:
    repo = branch_metric_repo(context.target["name"], context.fixture)
    client = McpClient(context.sensez)
    try:
        client.request("initialize")
        report = scenario(
            FixtureSession(
                client,
                repo,
                context.fixture,
                context.target["name"],
            )
        )
        dump_normalized(
            context.out / artifact,
            report,
            repo,
            context.target,
        )
    finally:
        client.close()
        cleanup_repo(repo)
