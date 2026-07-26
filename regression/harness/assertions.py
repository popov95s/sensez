from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from .json_values import JsonPath, json_path


@dataclass(frozen=True)
class FixtureIdentity:
    path: str
    symbol: str


def assert_brainz_totals_reported(report: object, target_name: str) -> None:
    reported = json_path(report, ("all_time", "reported_by_detector"))
    if not isinstance(reported, dict) or not reported:
        raise AssertionError(
            f"{target_name}: brainz report has empty reported_by_detector"
        )
    non_zero = [
        count for count in reported.values() if isinstance(count, int) and count > 0
    ]
    if not non_zero:
        raise AssertionError(
            f"{target_name}: no detector reported any findings: {reported}"
        )


def assert_gate_blocks(response: object, target_name: str) -> None:
    if json_path(response, ("decision",)) != "block":
        raise AssertionError(f"{target_name}: gate expected to block, got {response!r}")


def assert_gate_allows(response: object, reason: str) -> None:
    if response != {} and json_path(response, JsonPath(("decision",))) == "block":
        raise AssertionError(f"gate expected to allow ({reason}), got {response!r}")


def assert_gate_mentions_new_only(
    response: object,
    new_symbol: str,
    old_symbol: str,
    target_name: str,
) -> None:
    reason = json_path(response, ("reason",))
    if not isinstance(reason, str):
        raise AssertionError(f"{target_name}: missing gate reason in {response!r}")
    if "1 diff finding(s)" not in reason or new_symbol not in reason:
        raise AssertionError(f"{target_name}: expected one new gate finding: {reason}")
    if old_symbol in reason:
        raise AssertionError(
            f"{target_name}: unchanged finding was re-listed: {reason}"
        )


def assert_session_gate_isolated(
    response: object, owned: FixtureIdentity, other: FixtureIdentity
) -> None:
    reason = json_path(response, ("reason",))
    if json_path(response, ("decision",)) != "block" or not isinstance(reason, str):
        raise RuntimeError(f"expected scoped gate block, got {response!r}")
    owned_path = Path(owned.path).name
    other_path = Path(other.path).name
    if (
        owned.symbol not in reason
        or owned_path not in reason
        or other.symbol in reason
        or other_path in reason
    ):
        raise RuntimeError(f"shared-worktree gate leaked findings: {reason}")


def assert_finding_resolved(
    report: object, detector: str, target_name: str
) -> None:
    resolved = json_path(
        report, JsonPath(("all_time", "resolved_by_detector", detector))
    )
    count = resolved.get("count") if isinstance(resolved, dict) else None
    if not isinstance(count, int) or count < 1:
        raise AssertionError(
            f"{target_name}: expected {detector} to be resolved, got {resolved!r}"
        )


def assert_finding_reintroduced(
    report: object, detector: str, target_name: str
) -> None:
    reintroduced = json_path(
        report,
        JsonPath(("all_time", "reintroduced_by_detector", detector)),
    )
    count = reintroduced.get("count") if isinstance(reintroduced, dict) else None
    if not isinstance(count, int) or count < 1:
        raise AssertionError(
            f"{target_name}: expected {detector} to be reintroduced, "
            f"got {reintroduced!r}"
        )
