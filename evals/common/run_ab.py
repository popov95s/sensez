#!/usr/bin/env python3
"""Run paired control vs Sensez agent attempts on prepared benchmark workspaces."""

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from workspace import assert_clean_start, git_state
from quality_score import score_payload
from scan_metrics import count_findings, diff_stats


@dataclass(frozen=True)
class RunContext:
    task: dict[str, Any]
    variant: str
    run: int
    workspace: Path
    out_dir: Path
    oc_config_home: str
    oc_data_home: str


def load_tasks(path: Path, limit: int | None) -> list[dict[str, Any]]:
    tasks = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    return tasks[:limit] if limit else tasks


def render(template: str, ctx: RunContext, prompt_file: Path) -> str:
    return template.format(
        workspace=str(ctx.workspace),
        prompt_file=str(prompt_file),
        task_id=ctx.task["id"],
        repo=ctx.task["repo"],
        base_commit=ctx.task["base_commit"],
        test_command=ctx.task.get("test_command", ""),
        variant=ctx.variant,
        run=ctx.run,
        env_config_home=ctx.oc_config_home,
        env_data_home=ctx.oc_data_home,
    )


def load_prompt_text(prompt_file: Path) -> str:
    lines = prompt_file.read_text().splitlines()
    if lines and lines[0].startswith("@include "):
        include_target = lines[0].split(maxsplit=1)[1].strip()
        base_file = (prompt_file.parent / include_target).resolve()
        base_text = base_file.read_text().rstrip()
        remainder = "\n".join(lines[1:]).lstrip("\n").rstrip()
        if remainder:
            return base_text + "\n\n" + remainder + "\n"
        return base_text + "\n"
    return prompt_file.read_text().rstrip() + "\n"


def run_command(
    command: str,
    cwd: Path | None,
    timeout: int,
    stdin: str | None = None,
    extra_env: dict[str, str] | None = None,
    use_shell: bool = False,
) -> dict[str, Any]:
    started = time.monotonic()
    merged_env = os.environ.copy()
    if extra_env:
        merged_env.update(extra_env)
    try:
        proc = subprocess.run(
            command if use_shell else shlex.split(command),
            cwd=cwd,
            text=True,
            capture_output=True,
            input=stdin,
            timeout=timeout,
            check=False,
            env=merged_env,
            shell=use_shell,
        )
        return {
            "command": command,
            "returncode": proc.returncode,
            "elapsed_seconds": round(time.monotonic() - started, 3),
            "stdout": proc.stdout,
            "stderr": proc.stderr,
            "timed_out": False,
        }
    except subprocess.TimeoutExpired as exc:
        return {
            "command": command,
            "returncode": None,
            "elapsed_seconds": round(time.monotonic() - started, 3),
            "stdout": exc.stdout or "",
            "stderr": exc.stderr or "",
            "timed_out": True,
        }


def count_sensez_tool_calls(agent_stdout: str) -> int:
    """Count how many times the agent invoked Sensez MCP tools.

    JSON in agent stdout may use escaped or raw quotes depending on
    whether it was double-serialized.
    """
    patterns = [
        r'\\?"tool\\?"\s*:\s*\\?"sensez_noze_sniff\\?"',
        r'\\?"tool\\?"\s*:\s*\\?"sensez_noze_gate\\?"',
    ]
    count = 0
    for pat in patterns:
        count += len(re.findall(pat, agent_stdout))
    return count


def parse_tokens(agent_stdout: str) -> dict[str, int]:
    """Extract token usage from opencode JSON output lines.

    Each step_finish event carries session-cumulative counts in `part.tokens.total`
    plus per-step breakdowns in `part.tokens.input/output/reasoning`.
    We take the max total seen (last cumulative) and sum per-step breakdowns.
    """
    tokens = {"input": 0, "output": 0, "reasoning": 0, "total": 0}
    for line in agent_stdout.splitlines():
        if '"step_finish"' not in line and '"type":"step_finish"' not in line:
            continue
        try:
            data = json.loads(line)
            t = data.get("part", {}).get("tokens", {})
            if not t:
                continue
            tokens["input"] += t.get("input", 0)
            tokens["output"] += t.get("output", 0)
            tokens["reasoning"] += t.get("reasoning", 0)
            if t.get("total", 0) > tokens["total"]:
                tokens["total"] = t["total"]
        except json.JSONDecodeError:
            continue
    return tokens


def sense_scan(sense_bin: str, workspace: Path, diff: bool) -> dict[str, Any]:
    args = [sense_bin, "noze", str(workspace), "--json"]
    if diff:
        args.append("--diff")
    proc = subprocess.run(args, text=True, capture_output=True, check=False)
    result: dict[str, Any] = {
        "command": args,
        "returncode": proc.returncode,
        "stderr": proc.stderr,
    }
    try:
        result["json"] = json.loads(proc.stdout)
    except json.JSONDecodeError:
        result["stdout"] = proc.stdout
        result["json"] = {}
    return result


def git_diff(workspace: Path) -> str:
    proc = subprocess.run(
        ["git", "diff", "--binary"],
        cwd=workspace,
        text=True,
        capture_output=True,
        check=False,
    )
    return proc.stdout


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def build_prompt(base_prompt: Path, task: dict[str, Any], destination: Path) -> None:
    text = load_prompt_text(base_prompt).rstrip()
    body = [
        text,
        "",
        "Benchmark task:",
        f"- id: {task['id']}",
        f"- repo: {task['repo']}",
        f"- category: {task['category']}",
        f"- summary: {task['summary']}",
    ]
    destination.write_text("\n".join(body) + "\n")


def run_one(args: argparse.Namespace, task: dict[str, Any], variant: str, run: int) -> None:
    workspace = Path(
        args.workspace_template.format(task_id=task["id"], variant=variant, run=run)
    )
    out_dir = Path(args.results_dir) / task["id"] / variant / f"run_{run}"
    out_dir.mkdir(parents=True, exist_ok=True)

    oc_cfg = Path(args.oc_home_template.format(variant=variant))
    oc_config_home = str(oc_cfg / "config")
    oc_data_home = str(oc_cfg / "data")

    ctx = RunContext(
        task=task,
        variant=variant,
        run=run,
        workspace=workspace,
        out_dir=out_dir,
        oc_config_home=oc_config_home,
        oc_data_home=oc_data_home,
    )

    prompt_source = Path(args.sensez_prompt if variant == "sensez" else args.control_prompt)
    prompt_file = out_dir / "prompt.md"
    build_prompt(prompt_source, task, prompt_file)
    write_json(out_dir / "task.json", task)

    if args.prepare_command_template:
        command = render(args.prepare_command_template, ctx, prompt_file)
        write_json(out_dir / "prepare.json", run_command(command, None, args.prepare_timeout))

    start_state = git_state(workspace)
    write_json(out_dir / "workspace_before.json", start_state)
    if not args.allow_dirty_start:
        assert_clean_start(workspace, start_state)

    before = sense_scan(args.sense_bin, workspace, diff=False)
    write_json(out_dir / "sensez_before.json", before)

    agent_env = {
        "XDG_CONFIG_HOME": oc_config_home,
        "XDG_DATA_HOME": oc_data_home,
    }
    agent_template = (
        args.sensez_agent_command_template
        if variant == "sensez"
        else args.control_agent_command_template
    )
    command = render(agent_template, ctx, prompt_file)
    prompt_stdin = None
    if args.agent_prompt_stdin:
        prompt_stdin = prompt_file.read_text()
    elif args.stdin_message:
        prompt_stdin = args.stdin_message
    agent_result = run_command(command, workspace, args.agent_timeout, prompt_stdin, agent_env)
    write_json(out_dir / "agent.json", agent_result)

    after = sense_scan(args.sense_bin, workspace, diff=False)
    diff_scan = sense_scan(args.sense_bin, workspace, diff=True)
    write_json(out_dir / "workspace_after.json", git_state(workspace))
    write_json(out_dir / "sensez_after.json", after)
    write_json(out_dir / "sensez_diff.json", diff_scan)

    (out_dir / "patch.diff").write_text(git_diff(workspace))
    stats = diff_stats(workspace)

    before_counts = count_findings(before)
    after_counts = count_findings(after)

    test_result = None
    if args.test_command_template:
        command = render(args.test_command_template, ctx, prompt_file)
        test_result = run_command(command, workspace, args.test_timeout)
        write_json(out_dir / "test.json", test_result)

    tokens = parse_tokens(agent_result["stdout"])
    quality = score_payload(diff_scan.get("json") or {})

    metrics = {
        "task_id": task["id"],
        "variant": variant,
        "run": run,
        "agent_returncode": agent_result["returncode"],
        "agent_elapsed_seconds": agent_result["elapsed_seconds"],
        "agent_timed_out": agent_result["timed_out"],
        "sensez_before": before_counts,
        "sensez_after": after_counts,
        "sensez_diff": count_findings(diff_scan),
        "sensez_delta_total": after_counts["total"] - before_counts["total"],
        "quality_regression_score": quality["quality_regression_score"],
        "new_quality_score": quality["new_quality_score"],
        "existing_quality_score": quality["existing_quality_score"],
        "inherent_quality_score": quality["inherent_quality_score"],
        "quality_severity": quality["severity"],
        "quality_by_pillar": quality["by_pillar"],
        "sensez_tool_calls": count_sensez_tool_calls(agent_result["stdout"]),
        "input_tokens": tokens["input"],
        "output_tokens": tokens["output"],
        "reasoning_tokens": tokens["reasoning"],
        "diff_stats": stats,
    }
    if test_result:
        metrics["test_returncode"] = test_result["returncode"]
        metrics["test_timed_out"] = test_result["timed_out"]
    write_json(out_dir / "metrics.json", metrics)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tasks", type=Path, required=True)
    parser.add_argument("--workspace-template", required=True)
    parser.add_argument("--oc-home-template", required=True)
    parser.add_argument("--agent-command-template")
    parser.add_argument("--control-agent-command-template")
    parser.add_argument("--sensez-agent-command-template")
    parser.add_argument("--prepare-command-template")
    parser.add_argument("--test-command-template")
    parser.add_argument("--agent-prompt-stdin", action="store_true")
    parser.add_argument("--stdin-message")
    parser.add_argument("--allow-dirty-start", action="store_true")
    parser.add_argument("--results-dir", default="evals/sensez_ab/results")
    parser.add_argument("--sense-bin", default="sense")
    parser.add_argument("--control-prompt", default="evals/prompts/control.md")
    parser.add_argument("--sensez-prompt", default="evals/prompts/sensez.md")
    parser.add_argument("--parallel", action="store_true")
    parser.add_argument("--runs", type=int, default=1)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--variants", nargs="+", default=["control", "sensez"])
    parser.add_argument("--agent-timeout", type=int, default=3600)
    parser.add_argument("--prepare-timeout", type=int, default=1800)
    parser.add_argument("--test-timeout", type=int, default=1800)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if not args.agent_command_template and not args.control_agent_command_template:
        raise SystemExit(
            "provide --agent-command-template or --control-agent-command-template"
        )
    if not args.control_agent_command_template:
        args.control_agent_command_template = args.agent_command_template
    if not args.sensez_agent_command_template:
        args.sensez_agent_command_template = args.agent_command_template
    tasks = load_tasks(args.tasks, args.limit)

    if args.parallel:
        from concurrent.futures import ThreadPoolExecutor, as_completed
        jobs = [
            (task, variant, run_num)
            for task in tasks
            for run_num in range(1, args.runs + 1)
            for variant in args.variants
        ]
        with ThreadPoolExecutor(max_workers=len(args.variants)) as ex:
            futures = {
                ex.submit(run_one, args, t, v, r): (t["id"], v, r)
                for t, v, r in jobs
            }
            for future in as_completed(futures):
                tid, v, r = futures[future]
                try:
                    future.result()
                    print(f"  OK  {tid}/{v}/run_{r}")
                except Exception as e:
                    print(f"  FAIL {tid}/{v}/run_{r}: {e}")
    else:
        for task in tasks:
            for run_num in range(1, args.runs + 1):
                for variant in args.variants:
                    run_one(args, task, variant, run_num)


if __name__ == "__main__":
    main()
