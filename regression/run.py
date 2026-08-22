from __future__ import annotations

import argparse
import os
import shutil
import sys
import tomllib
from pathlib import Path
from typing import cast

from .analyze import accept_tree, compare_tree
from .harness.models import RegressionConfig, RegressionRun, Target
from .harness.paths import BASELINES, CONFIG, RESULTS, ROOT
from .harness.repositories import ensure_cache
from .scenarios.branch_metrics import run_branch_metric_scenarios
from .scenarios.cache_impact import run_cache_impact_scenario
from .scenarios.full_scan import run_full_scans
from .scenarios.gates import (
    run_gate_detached_scenario,
    run_gate_reblock_scenario,
    run_shared_worktree_gate_scenario,
)
from .scenarios.mcp_flow import run_mcp_scenarios
from .scenarios.reflexez import run_reflexez_scenario
from .setup_regressions import run_setup_regressions


def main() -> int:
    args = parse_args()
    config = cast(RegressionConfig, tomllib.loads(CONFIG.read_text()))
    targets = [] if args.setup_only else select_targets(config["targets"], args)
    sensez = args.sensez.resolve()
    if not sensez.exists():
        print(f"missing release binary: {sensez}", file=sys.stderr)
        print("build it with: cargo build --release --all-features")
        return 2

    failures = _run_setup(sensez, args.accept) if not args.scenario else []
    for target in targets:
        try:
            run_target(config, target, sensez, args.accept, set(args.scenario))
        except Exception as error:
            failures.append(f"{target['name']}: {error}")
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", action="append", default=[])
    parser.add_argument("--profile", action="append", default=[])
    parser.add_argument(
        "--scenario",
        action="append",
        choices=["full", "cache", "reflexez", "mcp", "gate", "branch"],
        default=[],
    )
    parser.add_argument("--all", action="store_true")
    parser.add_argument("--setup-only", action="store_true")
    parser.add_argument("--ci", action="store_true")
    parser.add_argument("--accept", action="store_true")
    parser.add_argument(
        "--sensez",
        type=Path,
        default=ROOT / "target/release/sensez",
    )
    args = parser.parse_args()
    if args.accept and args.ci and not os.getenv("SENSEZ_ACCEPT_BASELINE"):
        parser.error("--accept in CI requires SENSEZ_ACCEPT_BASELINE=1")
    return args


def select_targets(
    targets: list[Target], args: argparse.Namespace
) -> list[Target]:
    if args.all or (not args.target and not args.profile):
        return targets
    names = set(args.target)
    profiles = set(args.profile)
    selected = [
        target
        for target in targets
        if target["name"] in names or target["profile"] in profiles
    ]
    missing = names - {target["name"] for target in selected}
    if missing:
        raise SystemExit(f"unknown target(s): {', '.join(sorted(missing))}")
    return selected


def run_target(
    config: RegressionConfig,
    target: Target,
    sensez: Path,
    accept: bool,
    scenarios: set[str],
) -> None:
    name = target["name"]
    print(f"== {name} ==")
    cache = ensure_cache(config["cache_root"], target)
    output = RESULTS / name
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    context = RegressionRun(sensez, config, target, cache, output)

    enabled = lambda name: not scenarios or name in scenarios
    if enabled("full"):
        run_full_scans(context)
    if enabled("cache"):
        run_cache_impact_scenario(context)
    if enabled("reflexez"):
        run_reflexez_scenario(context)
    if enabled("mcp"):
        run_mcp_scenarios(context)
    if enabled("gate"):
        run_gate_reblock_scenario(context)
        run_shared_worktree_gate_scenario(context)
        run_gate_detached_scenario(context)
    if enabled("branch"):
        run_branch_metric_scenarios(context)
    _compare_or_accept(output, BASELINES / name, name, accept, not scenarios)


def _run_setup(sensez: Path, accept: bool) -> list[str]:
    try:
        print("== setup ==")
        run_setup_regressions(
            sensez,
            RESULTS / "setup",
            BASELINES / "setup",
            accept,
        )
        return []
    except Exception as error:
        return [f"setup: {error}"]


def _compare_or_accept(
    output: Path,
    baseline: Path,
    name: str,
    accept: bool,
    require_complete: bool = True,
) -> None:
    if accept:
        accept_tree(output, baseline)
        print(f"accepted baselines for {name}")
        return
    failures = compare_tree(output, baseline, require_complete=require_complete)
    if failures:
        raise RuntimeError("\n\n".join(failures))


if __name__ == "__main__":
    raise SystemExit(main())
