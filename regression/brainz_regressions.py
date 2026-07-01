from __future__ import annotations

import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

from .mcp_client import McpClient, text_json


BRANCH_METRICS_CONFIG = """\
[dead_code]
unused_imports = true
unused_methods = true
unused_variables = true
"""


@dataclass(frozen=True)
class BranchCase:
    client: McpClient
    repo: Path
    fixture: object
    target_name: str


def branch_metric_repo(target_name: str, fixture) -> Path:
    tmp = Path(tempfile.mkdtemp(prefix=f"sensez-{target_name}-branch-metrics-"))
    repo = tmp / "repo"
    repo.mkdir()
    _git(repo, "init")
    (repo / "sensez.toml").write_text(BRANCH_METRICS_CONFIG)
    _write_base_source(repo, fixture)
    _commit_all(repo, "base")
    return repo


def assert_reported_stable_across_branch_switches(
    client: McpClient,
    repo: Path,
    target_name: str,
) -> object:
    before = _brainz_report(client, repo)
    reported = _required_map(
        before, ("all_time", "reported_by_detector"), target_name
    )
    resolved = _optional_map(before, ("all_time", "resolved_by_detector"))
    reintroduced = _optional_map(before, ("all_time", "reintroduced_by_detector"))
    scans_before = _required_int(before, ("all_time", "scans"), target_name)

    for branch in ("sensez-regression-main", "sensez-regression-alt"):
        _git(repo, "checkout", "-B", branch)
        _scan_full(client, repo)
        current = _brainz_report(client, repo)
        _assert_same_reported(current, reported, target_name, branch)

    _git(repo, "checkout", "sensez-regression-main")
    _scan_full(client, repo)
    after = _brainz_report(client, repo)
    _assert_same_reported(after, reported, target_name, "sensez-regression-main")
    scans_after = _required_int(after, ("all_time", "scans"), target_name)
    if scans_after != scans_before + 3:
        raise AssertionError(
            f"{target_name}: branch-switch regression did not run 3 scans "
            f"({scans_before} -> {scans_after})"
        )
    if _optional_map(after, ("all_time", "resolved_by_detector")) != resolved:
        raise AssertionError(f"{target_name}: branch switch changed resolved totals")
    if _optional_map(after, ("all_time", "reintroduced_by_detector")) != reintroduced:
        raise AssertionError(f"{target_name}: branch switch changed reintroduced totals")
    return after


def assert_exact_transition_count(
    report: object,
    detector: str,
    target_name: str,
    *,
    resolved: int,
    reintroduced: int,
) -> None:
    actual_resolved = _detector_count(report, "resolved_by_detector", detector)
    actual_reintroduced = _detector_count(report, "reintroduced_by_detector", detector)
    if actual_resolved != resolved or actual_reintroduced != reintroduced:
        raise AssertionError(
            f"{target_name}: expected {detector} transitions "
            f"resolved={resolved}, reintroduced={reintroduced}; got "
            f"resolved={actual_resolved}, reintroduced={actual_reintroduced}"
        )


def assert_colleague_main_issue_is_not_reintroduced(case: BranchCase) -> object:
    _git(case.repo, "checkout", "-B", "main")
    _scan_full(case.client, case.repo)

    _git(case.repo, "checkout", "-B", "feature-fix")
    _introduce_and_fix_on_current_branch(case)

    _git(case.repo, "checkout", "main")
    _apply_fixture(case.repo, case.fixture, case.fixture["text"])
    _commit_all(case.repo, "colleague introduces same issue on main")
    _scan_full(case.client, case.repo)
    main = _brainz_report(case.client, case.repo)
    assert_exact_transition_count(
        main,
        case.fixture["detector"],
        case.target_name,
        resolved=1,
        reintroduced=0,
    )
    return main


def assert_return_to_fixed_branch_stays_resolved(case: BranchCase) -> object:
    assert_colleague_main_issue_is_not_reintroduced(case)
    _git(case.repo, "checkout", "feature-fix")
    _scan_full(case.client, case.repo)
    feature = _brainz_report(case.client, case.repo)
    assert_exact_transition_count(
        feature,
        case.fixture["detector"],
        case.target_name,
        resolved=1,
        reintroduced=0,
    )
    reported = _optional_map(feature, ("all_time", "reported_by_detector"))
    if case.fixture["detector"] in reported:
        raise AssertionError(
            f"{case.target_name}: fixed feature branch still reports {case.fixture['detector']}"
        )
    return feature


def assert_same_branch_revert_is_reintroduced(case: BranchCase) -> object:
    _git(case.repo, "checkout", "-B", "main")
    _introduce_and_fix_on_current_branch(case)

    _apply_fixture(case.repo, case.fixture, case.fixture["text"])
    _commit_all(case.repo, "revert fixture fix")
    _scan_full(case.client, case.repo)
    reverted = _brainz_report(case.client, case.repo)
    assert_exact_transition_count(
        reverted,
        case.fixture["detector"],
        case.target_name,
        resolved=1,
        reintroduced=1,
    )
    return reverted


def _introduce_and_fix_on_current_branch(case: BranchCase) -> None:
    _apply_fixture(case.repo, case.fixture, case.fixture["text"])
    _commit_all(case.repo, "introduce fixture issue")
    _scan_full(case.client, case.repo)
    _apply_fixture(case.repo, case.fixture, case.fixture["fix_text"])
    _commit_all(case.repo, "fix fixture issue")
    _scan_full(case.client, case.repo)
    fixed = _brainz_report(case.client, case.repo)
    assert_exact_transition_count(
        fixed,
        case.fixture["detector"],
        case.target_name,
        resolved=1,
        reintroduced=0,
    )


def _scan_full(client: McpClient, repo: Path) -> None:
    client.call_tool("noze_sniff", {"path": str(repo), "diff": False})


def _brainz_report(client: McpClient, repo: Path) -> object:
    return text_json(client.call_tool("brainz_report", {"path": str(repo)}))


def _assert_same_reported(
    report: object,
    expected,
    target_name: str,
    step: str,
) -> None:
    actual = _required_map(report, ("all_time", "reported_by_detector"), target_name)
    if actual != expected:
        raise AssertionError(
            f"{target_name}: reported_by_detector changed on {step}: "
            f"expected {expected!r}, got {actual!r}"
        )


def _detector_count(report: object, category: str, detector: str) -> int:
    value = _path(report, ("all_time", category, detector, "count"))
    return value if isinstance(value, int) else 0


def _optional_map(value: object, path):
    found = _path(value, path)
    if found is None:
        return {}
    if isinstance(found, dict):
        return found
    raise AssertionError(f"expected object at {'.'.join(path)}, got {found!r}")


def _required_map(
    value: object,
    path,
    target_name: str,
):
    found = _optional_map(value, path)
    if not found:
        raise AssertionError(f"{target_name}: missing {'.'.join(path)}")
    return found


def _required_int(value: object, path, target_name: str) -> int:
    found = _path(value, path)
    if not isinstance(found, int):
        raise AssertionError(f"{target_name}: expected integer at {'.'.join(path)}")
    return found


def _path(value: object, keys) -> object:
    current = value
    for key in keys:
        if not isinstance(current, dict) or key not in current:
            return None
        current = current[key]
    return current


def _git(repo: Path, *args: str) -> None:
    proc = subprocess.run(
        ["git", *args],
        cwd=repo,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if proc.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed: {proc.stderr}")


def _commit_all(repo: Path, message: str) -> None:
    _git(repo, "add", ".")
    _git(
        repo,
        "-c",
        "user.email=sensez@example.test",
        "-c",
        "user.name=Sensez",
        "commit",
        "-m",
        message,
    )


def _apply_fixture(repo: Path, fixture, text: str) -> None:
    path = repo / fixture["path"]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def _write_base_source(repo: Path, fixture) -> None:
    path = repo / fixture["path"]
    if path.suffix == ".ts":
        (repo / "base.ts").write_text(
            "const sensezBranchBase = 1;\nconsole.log(sensezBranchBase);\n"
        )
    else:
        (repo / "base.py").write_text("print('base')\n")
