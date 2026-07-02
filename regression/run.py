from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import NotRequired, Sequence, TypeAlias, TypedDict, cast

from .analyze import accept_tree, compare_tree
from .brainz_regressions import (
    BranchCase,
    assert_colleague_main_issue_is_not_reintroduced,
    assert_detached_scan_does_not_change_transitions,
    assert_exact_transition_count,
    assert_reported_stable_across_branch_switches,
    assert_return_to_fixed_branch_stays_resolved,
    assert_same_branch_revert_is_reintroduced,
    branch_metric_repo,
)
from .mcp_client import McpClient, text_json
from .normalize import dump_json, normalize_artifact


ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "regression" / "targets.toml"
RESULTS = ROOT / "regression" / "results"
BASELINES = ROOT / "regression" / "baselines"
CommandArg = str | Path | int
JsonPathLike: TypeAlias = "JsonPath | tuple[str, ...]"
REGRESSION_SENSEZ_TOML = """\
[dead_code]
entry_points = [
  "**/packages/zod/**",
  "**/src/flask/app.py",
]
unused_imports = true
unused_methods = true
unused_variables = true
"""


class Target(TypedDict):
    name: str
    profile: str
    url: str
    commit: str
    scenarios: list[str]
    setup: NotRequired[list[str]]


class DeadCodeFixture(TypedDict):
    path: str
    symbol: str
    detector: str
    text: str
    fix_text: str


class ProfileConfig(TypedDict):
    dead_code_fixture: DeadCodeFixture


class RegressionConfig(TypedDict):
    cache_root: str
    targets: list[Target]
    profiles: dict[str, ProfileConfig]


@dataclass(frozen=True)
class JsonPath:
    segments: tuple[str, ...]


def main() -> int:
    args = parse_args()
    config = cast(RegressionConfig, tomllib.loads(CONFIG.read_text()))
    targets = select_targets(config["targets"], args)
    sensez = args.sensez.resolve()
    if not sensez.exists():
        print(f"missing release binary: {sensez}", file=sys.stderr)
        print("build it with: cargo build --release --all-features")
        return 2
    failures: list[str] = []
    for target in targets:
        try:
            run_target(config, target, sensez, args.accept)
        except Exception as exc:
            failures.append(f"{target['name']}: {exc}")
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", action="append", default=[])
    parser.add_argument("--profile", action="append", default=[])
    parser.add_argument("--all", action="store_true")
    parser.add_argument("--ci", action="store_true")
    parser.add_argument("--accept", action="store_true")
    parser.add_argument("--sensez", type=Path, default=ROOT / "target/release/sensez")
    args = parser.parse_args()
    if args.accept and args.ci and not os.getenv("SENSEZ_ACCEPT_BASELINE"):
        parser.error("--accept in CI requires SENSEZ_ACCEPT_BASELINE=1")
    return args


def select_targets(targets: list[Target], args: argparse.Namespace) -> list[Target]:
    if args.all or (not args.target and not args.profile):
        return targets
    names = set(args.target)
    profiles = set(args.profile)
    selected = [
        t for t in targets if t["name"] in names or t["profile"] in profiles
    ]
    missing = names - {t["name"] for t in selected}
    if missing:
        raise SystemExit(f"unknown target(s): {', '.join(sorted(missing))}")
    return selected


def run_target(config: RegressionConfig, target: Target, sensez: Path, accept: bool) -> None:
    name = target["name"]
    print(f"== {name} ==")
    cache = ensure_cache(config["cache_root"], target)
    out = RESULTS / name
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    run_full_scans(sensez, cache, target, out)
    run_mcp_scenarios(sensez, config, target, cache, out)
    run_gate_reblock_scenario(sensez, config, target, cache, out)
    run_branch_metric_scenarios(sensez, config, target, out)
    baseline = BASELINES / name
    if accept:
        accept_tree(out, baseline)
        print(f"accepted baselines for {name}")
        return
    failures = compare_tree(out, baseline)
    if failures:
        raise RuntimeError("\n\n".join(failures))


def ensure_cache(root_text: str, target: Target) -> Path:
    root = Path(root_text)
    root.mkdir(parents=True, exist_ok=True)
    dest = root / target["name"]
    if not (dest / ".git").exists():
        seed = Path("/tmp/bench-targets") / target["name"]
        if (seed / ".git").exists():
            run(["git", "clone", "--local", str(seed), str(dest)], ROOT)
        else:
            run(["git", "clone", target["url"], str(dest)], ROOT)
    run(["git", "fetch", "--depth", "1", "origin", target["commit"]], dest, check=False)
    run(["git", "checkout", "--force", target["commit"]], dest)
    run(["git", "clean", "-ffd"], dest)
    if target.get("setup") and not (dest / "node_modules").exists():
        setup = target["setup"]
        run(setup, dest)
    return dest


def scenario_repo(cache: Path, target: Target) -> Path:
    name = target["name"]
    tmp = Path(tempfile.mkdtemp(prefix=f"sensez-{name}-"))
    dest = tmp / name
    run(["git", "clone", "--local", str(cache), str(dest)], ROOT)
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=cache, text=True).strip()
    run(["git", "checkout", "--force", head], dest)
    run(["git", "checkout", "-B", "sensez-regression-worktree"], dest)
    if target.get("setup"):
        cached_modules = cache / "node_modules"
        if cached_modules.exists():
            shutil.copytree(cached_modules, dest / "node_modules", symlinks=True)
        else:
            run(target["setup"], dest)
    write_regression_config(dest)
    return dest


def write_regression_config(repo: Path) -> None:
    (repo / "sensez.toml").write_text(REGRESSION_SENSEZ_TOML)


def run_full_scans(sensez: Path, cache: Path, target: Target, out: Path) -> None:
    repo = scenario_repo(cache, target)
    try:
        default = run_json([sensez, "noze", str(repo), "--json"], ROOT)
        dump_norm(out / "default.noze.json", default, repo, target)
        default_capped = run_json([sensez, "noze", str(repo), "--max", "5", "--json"], ROOT)
        dump_norm(out / "default.noze.max5.json", default_capped, repo, target)
        full = run_json([sensez, "noze", str(repo), "--all", "--json"], ROOT)
        dump_norm(out / "full.noze.json", full, repo, target)
        threshold = run_json(
            [sensez, "noze", str(repo), "--all", "--threshold", "40", "--json"], ROOT
        )
        dump_norm(out / "full.noze.threshold40.json", threshold, repo, target)
        capped = run_json([sensez, "noze", str(repo), "--all", "--max", "5", "--json"], ROOT)
        dump_norm(out / "full.noze.max5.json", capped, repo, target)
    finally:
        cleanup_repo(repo)


def run_mcp_scenarios(
    sensez: Path,
    config: RegressionConfig,
    target: Target,
    cache: Path,
    out: Path,
) -> None:
    repo = scenario_repo(cache, target)
    fixture = config["profiles"][target["profile"]]["dead_code_fixture"]
    client = McpClient(sensez)
    try:
        init = client.request("initialize")["result"]
        assert init["serverInfo"]["name"] == "sensez"
        tools = client.request("tools/list")["result"]
        dump_norm(out / "mcp.tools.json", tools, repo, target)
        # The "full" scan feeds brainz metrics — no per-pillar cap, so the
        # `reported_by_detector` counts every detector that fired.
        scan = text_json(
            client.call_tool("noze_sniff", {"path": str(repo), "diff": False})
        )
        dump_norm(out / "mcp.full.noze.json", scan, repo, target)
        # The "limited" scan is the agent-facing shape: per-pillar cap so
        # the response stays small enough for the model's context window.
        # `record: false` keeps it out of `reported_by_detector` so the
        # shape preview doesn't double-count.
        limited = text_json(
            client.call_tool(
                "noze_sniff",
                {"path": str(repo), "limit": 20, "diff": False, "record": False},
            )
        )
        dump_norm(out / "mcp.limited.noze.json", limited, repo, target)
        report = text_json(client.call_tool("brainz_report", {"path": str(repo)}))
        dump_norm(out / "brainz.after-full.json", report, repo, target)
        assert_brainz_totals_reported(report, target["name"])
        apply_fixture(repo, fixture, fixture["text"])
        diff = text_json(client.call_tool("noze_sniff", {"path": str(repo), "diff": True}))
        dump_norm(out / "diff.noze.json", diff, repo, target)
        gate = text_json(client.call_tool("noze_gate", {"path": str(repo)}))
        dump_norm(out / "gate.block.json", gate, repo, target)
        assert_gate_blocks(gate, target["name"])
        # Second gate call with the same content: the signature dedups
        # against the last block, so the gate allows without the host
        # needing to set `stop_hook_active`. This is the agent-friendly
        # UX: one block per real complaint, not one per turn.
        allow_same = text_json(
            client.call_tool("noze_gate", {"path": str(repo)})
        )
        dump_norm(out / "gate.allow-same-content.json", allow_same, repo, target)
        assert_gate_allows(allow_same, "signature dedup")
        allow = text_json(
            client.call_tool("noze_gate", {"path": str(repo), "stop_hook_active": True})
        )
        dump_norm(out / "gate.allow.json", allow, repo, target)
        assert_gate_allows(allow, "stop_hook_active")
        # Fix / reintroduction must run BEFORE the triage scenario: a
        # "debt" triage masks the disappearance from the resolved tally
        # (intentional debt is not a fix). The order is: fix → scan →
        # reintroduce → scan, with assertions after each brainz report.
        apply_fixture(repo, fixture, fixture["fix_text"])
        client.call_tool("noze_sniff", {"path": str(repo), "limit": 20})
        fixed = text_json(client.call_tool("brainz_report", {"path": str(repo)}))
        dump_norm(out / "brainz.after-gate-fix.json", fixed, repo, target)
        assert_finding_resolved(fixed, fixture["detector"], target["name"])
        assert_exact_transition_count(
            fixed,
            fixture["detector"],
            target["name"],
            resolved=1,
            reintroduced=0,
        )
        # Reintroduce the same fixture: the next scan should count it as
        # a reintroduction (previously-resolved fingerprint came back).
        apply_fixture(repo, fixture, fixture["text"])
        client.call_tool("noze_sniff", {"path": str(repo), "limit": 20})
        reintroduced = text_json(client.call_tool("brainz_report", {"path": str(repo)}))
        dump_norm(out / "brainz.after-reintro.json", reintroduced, repo, target)
        assert_finding_reintroduced(
            reintroduced, fixture["detector"], target["name"]
        )
        assert_exact_transition_count(
            reintroduced,
            fixture["detector"],
            target["name"],
            resolved=1,
            reintroduced=1,
        )
        if "triage" in target.get("scenarios", []):
            triage(client, repo, fixture)
        # Past `repeat_limit`, the finding is auto-deferred and the gate
        # allows (the report has zero findings after suppression). The
        # agent already saw the block on the first call; nagging again
        # would be a no-op. Run this last so the auto-deferral does not
        # mask the fix / reintroduction flows above.
        deferred = text_json(client.call_tool("noze_gate", {"path": str(repo)}))
        dump_norm(out / "gate.defer.json", deferred, repo, target)
        assert_gate_allows(deferred, "auto-deferred past repeat_limit")
        dump_metrics_schema(out / "metrics-files.schema.json", repo)
        branch_switch = assert_reported_stable_across_branch_switches(
            client, repo, target["name"]
        )
        dump_norm(out / "brainz.after-branch-switch.json", branch_switch, repo, target)
    finally:
        client.close()
        cleanup_repo(repo)


def run_branch_metric_scenarios(
    sensez: Path,
    config: RegressionConfig,
    target: Target,
    out: Path,
) -> None:
    fixture = config["profiles"][target["profile"]]["dead_code_fixture"]
    run_branch_metric_case(
        sensez,
        target,
        fixture,
        out,
        "brainz.branch-colleague-main.json",
        assert_colleague_main_issue_is_not_reintroduced,
    )
    run_branch_metric_case(
        sensez,
        target,
        fixture,
        out,
        "brainz.branch-return-feature-fixed.json",
        assert_return_to_fixed_branch_stays_resolved,
    )
    run_branch_metric_case(
        sensez,
        target,
        fixture,
        out,
        "brainz.branch-same-branch-revert.json",
        assert_same_branch_revert_is_reintroduced,
    )
    run_branch_metric_case(
        sensez,
        target,
        fixture,
        out,
        "brainz.branch-detached-scan.json",
        assert_detached_scan_does_not_change_transitions,
    )


def run_gate_reblock_scenario(
    sensez: Path,
    config: RegressionConfig,
    target: Target,
    cache: Path,
    out: Path,
) -> None:
    repo = scenario_repo(cache, target)
    fixture = config["profiles"][target["profile"]]["dead_code_fixture"]
    client = McpClient(sensez)
    try:
        client.request("initialize")
        apply_fixture(repo, fixture, fixture["text"])
        first = text_json(client.call_tool("noze_gate", {"path": str(repo)}))
        assert_gate_blocks(first, target["name"])

        extra_symbol = extra_symbol_for(fixture)
        extra_fixture = extra_dead_code_fixture(fixture, extra_symbol)
        apply_fixture(repo, extra_fixture, extra_fixture["text"])
        second = text_json(client.call_tool("noze_gate", {"path": str(repo)}))
        dump_norm(out / "gate.block-new-only.json", second, repo, target)
        assert_gate_blocks(second, target["name"])
        assert_gate_mentions_new_only(second, extra_symbol, fixture["symbol"], target["name"])
    finally:
        client.close()
        cleanup_repo(repo)


def run_branch_metric_case(
    sensez: Path,
    target: Target,
    fixture: DeadCodeFixture,
    out: Path,
    artifact: str,
    scenario,
) -> None:
    repo = branch_metric_repo(target["name"], fixture)
    client = McpClient(sensez)
    try:
        client.request("initialize")
        report = scenario(BranchCase(client, repo, fixture, target["name"]))
        dump_norm(out / artifact, report, repo, target)
    finally:
        client.close()
        cleanup_repo(repo)


def cleanup_repo(repo: Path) -> None:
    shutil.rmtree(repo.parent, ignore_errors=True)


def assert_brainz_totals_reported(report: object, target_name: str) -> None:
    """The brainz report must carry non-zero reported counts for the
    detectors that fire on a real repo. A regression that drops the
    counters (or the scan that fills them) would silently break the
    report's totals.
    """
    reported = _json_path(report, ("all_time", "reported_by_detector"))
    if not isinstance(reported, dict) or not reported:
        raise AssertionError(
            f"{target_name}: brainz report has empty reported_by_detector"
        )
    non_zero = {
        detector: count
        for detector, count in reported.items()
        if isinstance(count, int) and count > 0
    }
    if not non_zero:
        raise AssertionError(
            f"{target_name}: no detector reported any findings: {reported}"
        )


def assert_gate_blocks(response: object, target_name: str) -> None:
    """A gate response that the baseline calls a `block` must really
    carry `decision == block`. Catches a regression where the gate
    silently allows on a finding that should nag the agent.
    """
    decision = _json_path(response, ("decision",))
    if decision != "block":
        raise AssertionError(
            f"{target_name}: gate expected to block, got {response!r}"
        )


def assert_gate_allows(response: object, reason: str) -> None:
    """A gate response that the baseline calls an `allow` must be the
    empty-JSON `{}` payload (or at least not a block). Catches a
    regression where the gate re-blocks on unchanged content.
    """
    if response != {} and _json_path(response, JsonPath(("decision",))) == "block":
        raise AssertionError(f"gate expected to allow ({reason}), got {response!r}")


def assert_gate_mentions_new_only(
    response: object,
    new_symbol: str,
    old_symbol: str,
    target_name: str,
) -> None:
    reason = _json_path(response, ("reason",))
    if not isinstance(reason, str):
        raise AssertionError(f"{target_name}: missing gate reason in {response!r}")
    if "1 diff finding(s)" not in reason:
        raise AssertionError(f"{target_name}: expected one new gate finding: {reason}")
    if new_symbol not in reason:
        raise AssertionError(f"{target_name}: new finding missing from gate reason: {reason}")
    if old_symbol in reason:
        raise AssertionError(f"{target_name}: unchanged finding was re-listed: {reason}")


def assert_finding_resolved(report: object, detector: str, target_name: str) -> None:
    """After the agent fixes a finding, the brainz totals must count it
    as resolved under the fixture's detector. A regression where the
    fix-recapture loop stops banking resolutions would distort fix
    reintroduction tracking and starve precision of its denominator.
    """
    resolved = _json_path(
        report, JsonPath(("all_time", "resolved_by_detector", detector))
    )
    count = resolved.get("count") if isinstance(resolved, dict) else None
    if not isinstance(count, int) or count < 1:
        raise AssertionError(
            f"{target_name}: expected {detector} to be resolved, got {resolved!r}"
        )


def assert_finding_reintroduced(report: object, detector: str, target_name: str) -> None:
    """After the agent reintroduces a previously-fixed finding, the
    brainz totals must count it as a reintroduction. A regression here
    would silently drop the fix reintroduction signal and let noisy detectors
    skate past calibration.
    """
    reintroduced = _json_path(
        report, JsonPath(("all_time", "reintroduced_by_detector", detector))
    )
    count = reintroduced.get("count") if isinstance(reintroduced, dict) else None
    if not isinstance(count, int) or count < 1:
        raise AssertionError(
            f"{target_name}: expected {detector} to be reintroduced, got {reintroduced!r}"
        )


def _json_path(value: object, keys: JsonPathLike) -> object:
    current = value
    segments = keys.segments if isinstance(keys, JsonPath) else keys
    for key in segments:
        if not isinstance(current, dict) or key not in current:
            return None
        current = current[key]
    return current


def triage(client: McpClient, repo: Path, fixture: DeadCodeFixture) -> None:
    client.call_tool(
        "brainz_triage",
        {
            "path": str(repo),
            "pillar": "dead_code",
            "match": fixture["symbol"],
            "verdict": "debt",
            "note": "regression fixture",
        },
    )


def apply_fixture(repo: Path, fixture: DeadCodeFixture, text: str) -> None:
    path = repo / fixture["path"]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    write_fixture_consumer(repo, fixture)


def extra_dead_code_fixture(fixture: DeadCodeFixture, symbol: str) -> DeadCodeFixture:
    path = Path(fixture["path"])
    extra_path = path.with_name(f"{path.stem}_extra{path.suffix}")
    if path.suffix == ".ts":
        text = f"function {symbol}(): number {{\n  return 84;\n}}\n"
    else:
        text = f"def {symbol}():\n    return 84\n"
    return {
        "path": str(extra_path),
        "symbol": symbol,
        "detector": fixture["detector"],
        "text": text,
        "fix_text": "",
    }


def extra_symbol_for(fixture: DeadCodeFixture) -> str:
    if Path(fixture["path"]).suffix == ".ts":
        return "sensezNewGateHelper"
    return "sensez_new_gate_helper"


def write_fixture_consumer(repo: Path, fixture: DeadCodeFixture) -> None:
    path = Path(fixture["path"])
    live = live_symbol_for(path)
    if path.suffix == ".ts":
        consumer = path.with_name(f"{path.stem}-consumer{path.suffix}")
        module = f"./{path.with_suffix('').name}"
        text = f'import {{ {live} }} from "{module}";\nconsole.log({live});\n'
    else:
        consumer = path.with_name(f"{path.stem}_consumer{path.suffix}")
        module = path.with_suffix("").as_posix().replace("/", ".")
        text = f"from {module} import {live}\n\nprint({live}())\n"
    target = repo / consumer
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text)


def live_symbol_for(path: Path) -> str:
    if path.suffix == ".ts":
        return "sensezRegressionLiveHelper"
    return "sensez_regression_live_helper"


def dump_metrics_schema(path: Path, repo: Path) -> None:
    metric_dir = repo / ".sensez" / "local-metrics"
    files = sorted(p.name for p in metric_dir.glob("*") if p.is_file())
    events = []
    event_log = metric_dir / "events.jsonl"
    if event_log.exists():
        for line in event_log.read_text().splitlines():
            events.append(json.loads(line).get("event"))
    dump_json(path, {"files": files, "events": sorted(set(events))})


def dump_norm(path: Path, value: object, repo: Path, target: Target) -> None:
    dump_json(path, normalize_artifact(value, repo, target["name"]))


def run_json(cmd: Sequence[CommandArg], cwd: Path) -> object:
    output = run(cmd, cwd, capture=True)
    if output is None:
        raise RuntimeError("command produced no output")
    return json.loads(output)


def run(
    cmd: Sequence[CommandArg],
    cwd: Path,
    capture: bool = False,
    check: bool = True,
) -> str | None:
    text_cmd = [str(part) for part in cmd]
    proc = subprocess.run(
        text_cmd,
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE,
    )
    if check and proc.returncode != 0:
        raise RuntimeError(
            f"command failed ({proc.returncode}): {' '.join(text_cmd)}\n{proc.stderr}"
        )
    return proc.stdout


if __name__ == "__main__":
    raise SystemExit(main())
