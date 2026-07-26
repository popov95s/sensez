from __future__ import annotations

from ..harness.models import FixtureSession
from .assertions import assert_exact_transition_count
from .reports import brainz_report, optional_map, scan_full
from .repository import apply_branch_fixture, commit_all, git


def colleague_main_issue_is_not_reintroduced(
    case: FixtureSession,
) -> object:
    git(case.repo, "checkout", "-B", "main")
    scan_full(case.client, case.repo)

    git(case.repo, "checkout", "-B", "feature-fix")
    _introduce_and_fix(case)

    git(case.repo, "checkout", "main")
    apply_branch_fixture(case.repo, case.fixture, case.fixture["text"])
    commit_all(case.repo, "colleague introduces same issue on main")
    scan_full(case.client, case.repo)
    report = brainz_report(case.client, case.repo)
    assert_exact_transition_count(
        report,
        case.fixture["detector"],
        case.target_name,
        resolved=1,
        reintroduced=0,
    )
    return report


def return_to_fixed_branch_stays_resolved(
    case: FixtureSession,
) -> object:
    colleague_main_issue_is_not_reintroduced(case)
    git(case.repo, "checkout", "feature-fix")
    scan_full(case.client, case.repo)
    report = brainz_report(case.client, case.repo)
    assert_exact_transition_count(
        report,
        case.fixture["detector"],
        case.target_name,
        resolved=1,
        reintroduced=0,
    )
    reported = optional_map(
        report,
        ("all_time", "reported_by_detector"),
    )
    if reported.contains(case.fixture["detector"]):
        raise AssertionError(
            f"{case.target_name}: fixed feature branch still reports "
            f"{case.fixture['detector']}"
        )
    return report


def same_branch_revert_is_reintroduced(case: FixtureSession) -> object:
    git(case.repo, "checkout", "-B", "main")
    _introduce_and_fix(case)

    apply_branch_fixture(case.repo, case.fixture, case.fixture["text"])
    commit_all(case.repo, "revert fixture fix")
    scan_full(case.client, case.repo)
    report = brainz_report(case.client, case.repo)
    assert_exact_transition_count(
        report,
        case.fixture["detector"],
        case.target_name,
        resolved=1,
        reintroduced=1,
    )
    return report


def detached_scan_does_not_change_transitions(
    case: FixtureSession,
) -> object:
    git(case.repo, "checkout", "-B", "main")
    apply_branch_fixture(case.repo, case.fixture, case.fixture["text"])
    commit_all(case.repo, "introduce fixture issue")
    scan_full(case.client, case.repo)

    git(case.repo, "checkout", "--detach")
    apply_branch_fixture(case.repo, case.fixture, case.fixture["fix_text"])
    scan_full(case.client, case.repo)
    report = brainz_report(case.client, case.repo)
    assert_exact_transition_count(
        report,
        case.fixture["detector"],
        case.target_name,
        resolved=0,
        reintroduced=0,
    )
    return report


def _introduce_and_fix(case: FixtureSession) -> None:
    apply_branch_fixture(case.repo, case.fixture, case.fixture["text"])
    commit_all(case.repo, "introduce fixture issue")
    scan_full(case.client, case.repo)
    apply_branch_fixture(case.repo, case.fixture, case.fixture["fix_text"])
    commit_all(case.repo, "fix fixture issue")
    scan_full(case.client, case.repo)
    report = brainz_report(case.client, case.repo)
    assert_exact_transition_count(
        report,
        case.fixture["detector"],
        case.target_name,
        resolved=1,
        reintroduced=0,
    )
