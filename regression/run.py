from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path
from typing import NotRequired, Sequence, TypedDict, cast

from .analyze import accept_tree, compare_tree
from .mcp_client import McpClient, text_json
from .normalize import dump_json, normalize_artifact


ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "regression" / "targets.toml"
RESULTS = ROOT / "regression" / "results"
BASELINES = ROOT / "regression" / "baselines"
CommandArg = str | Path | int


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


def main() -> int:
    args = parse_args()
    config = cast(RegressionConfig, tomllib.loads(CONFIG.read_text()))
    targets = select_targets(config["targets"], args)
    sense = args.sense.resolve()
    if not sense.exists():
        print(f"missing release binary: {sense}", file=sys.stderr)
        print("build it with: cargo build --release --features mcp,all-langs")
        return 2
    failures: list[str] = []
    for target in targets:
        try:
            run_target(config, target, sense, args.accept)
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
    parser.add_argument("--sense", type=Path, default=ROOT / "target/release/sensez")
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


def run_target(config: RegressionConfig, target: Target, sense: Path, accept: bool) -> None:
    name = target["name"]
    print(f"== {name} ==")
    cache = ensure_cache(config["cache_root"], target)
    out = RESULTS / name
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    run_full_scans(sense, cache, target, out)
    run_mcp_scenarios(sense, config, target, cache, out)
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
    if target.get("setup"):
        cached_modules = cache / "node_modules"
        if cached_modules.exists():
            shutil.copytree(cached_modules, dest / "node_modules", symlinks=True)
        else:
            run(target["setup"], dest)
    return dest


def run_full_scans(sense: Path, cache: Path, target: Target, out: Path) -> None:
    repo = scenario_repo(cache, target)
    try:
        default = run_json([sense, "noze", str(repo), "--json"], ROOT)
        dump_norm(out / "default.noze.json", default, repo, target)
        default_capped = run_json([sense, "noze", str(repo), "--max", "5", "--json"], ROOT)
        dump_norm(out / "default.noze.max5.json", default_capped, repo, target)
        full = run_json([sense, "noze", str(repo), "--all", "--json"], ROOT)
        dump_norm(out / "full.noze.json", full, repo, target)
        threshold = run_json(
            [sense, "noze", str(repo), "--all", "--threshold", "40", "--json"], ROOT
        )
        dump_norm(out / "full.noze.threshold40.json", threshold, repo, target)
        capped = run_json([sense, "noze", str(repo), "--all", "--max", "5", "--json"], ROOT)
        dump_norm(out / "full.noze.max5.json", capped, repo, target)
    finally:
        cleanup_repo(repo)


def run_mcp_scenarios(
    sense: Path,
    config: RegressionConfig,
    target: Target,
    cache: Path,
    out: Path,
) -> None:
    repo = scenario_repo(cache, target)
    fixture = config["profiles"][target["profile"]]["dead_code_fixture"]
    client = McpClient(sense)
    try:
        init = client.request("initialize")["result"]
        assert init["serverInfo"]["name"] == "sensez"
        tools = client.request("tools/list")["result"]
        dump_norm(out / "mcp.tools.json", tools, repo, target)
        scan = text_json(client.call_tool("noze_sniff", {"path": str(repo), "limit": 20}))
        dump_norm(out / "mcp.full.noze.json", scan, repo, target)
        report = text_json(client.call_tool("brainz_report", {"path": str(repo)}))
        dump_norm(out / "brainz.after-full.json", report, repo, target)
        apply_fixture(repo, fixture, fixture["text"])
        diff = text_json(client.call_tool("noze_sniff", {"path": str(repo), "diff": True}))
        dump_norm(out / "diff.noze.json", diff, repo, target)
        gate = text_json(client.call_tool("noze_gate", {"path": str(repo)}))
        dump_norm(out / "gate.block.json", gate, repo, target)
        allow = text_json(
            client.call_tool("noze_gate", {"path": str(repo), "stop_hook_active": True})
        )
        dump_norm(out / "gate.allow.json", allow, repo, target)
        if "triage" in target.get("scenarios", []):
            triage(client, repo, fixture)
        apply_fixture(repo, fixture, fixture["fix_text"])
        client.call_tool("noze_sniff", {"path": str(repo), "limit": 20})
        fixed = text_json(client.call_tool("brainz_report", {"path": str(repo)}))
        dump_norm(out / "brainz.after-gate-fix.json", fixed, repo, target)
        dump_metrics_schema(out / "metrics-files.schema.json", repo)
    finally:
        client.close()
        cleanup_repo(repo)


def cleanup_repo(repo: Path) -> None:
    shutil.rmtree(repo.parent, ignore_errors=True)


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
    return json.loads(output)


def run(cmd: Sequence[CommandArg], cwd: Path, capture: bool = False, check: bool = True) -> str:
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
    return proc.stdout or ""


if __name__ == "__main__":
    raise SystemExit(main())
