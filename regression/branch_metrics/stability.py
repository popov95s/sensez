from pathlib import Path

from ..mcp_client import McpClient
from .assertions import assert_same_reported
from .reports import (
    brainz_report,
    optional_map,
    required_int,
    required_map,
    scan_full,
)
from .repository import git


def reported_stable_across_branch_switches(
    client: McpClient,
    repo: Path,
    target_name: str,
) -> object:
    before = brainz_report(client, repo)
    reported = required_map(
        before,
        ("all_time", "reported_by_detector"),
        target_name,
    )
    resolved = optional_map(
        before,
        ("all_time", "resolved_by_detector"),
    )
    reintroduced = optional_map(
        before,
        ("all_time", "reintroduced_by_detector"),
    )
    scans_before = required_int(
        before,
        ("all_time", "scans"),
        target_name,
    )

    for branch in ("sensez-regression-main", "sensez-regression-alt"):
        git(repo, "checkout", "-B", branch)
        scan_full(client, repo)
        current = brainz_report(client, repo)
        assert_same_reported(current, reported, target_name, branch)

    git(repo, "checkout", "sensez-regression-main")
    scan_full(client, repo)
    after = brainz_report(client, repo)
    assert_same_reported(
        after,
        reported,
        target_name,
        "sensez-regression-main",
    )
    scans_after = required_int(
        after,
        ("all_time", "scans"),
        target_name,
    )
    if scans_after != scans_before + 3:
        raise AssertionError(
            f"{target_name}: branch-switch regression did not run 3 scans "
            f"({scans_before} -> {scans_after})"
        )
    if optional_map(after, ("all_time", "resolved_by_detector")) != resolved:
        raise AssertionError(f"{target_name}: branch switch changed resolved totals")
    if optional_map(after, ("all_time", "reintroduced_by_detector")) != reintroduced:
        raise AssertionError(
            f"{target_name}: branch switch changed reintroduced totals"
        )
    return after
