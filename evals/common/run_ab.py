#!/usr/bin/env python3
"""Run paired control vs Sensez agent attempts on prepared benchmark workspaces."""

from __future__ import annotations

import argparse
from pathlib import Path

from models import TaskSpec
from run_models import (
    BenchmarkMetrics,
    Environment,
    JsonDocument,
    RunContext,
    json_value,
)
from quality_score import score_payload
from run_support import (
    build_prompt,
    count_sensez_tool_calls,
    git_diff,
    load_tasks,
    parse_tokens,
    render,
    run_command,
    sense_scan,
    write_json,
)
from scan_metrics import count_findings, diff_stats
from workspace import assert_clean_start, git_state


def run_one(args: argparse.Namespace, task: TaskSpec, variant: str, run: int) -> None:
    workspace = Path(
        args.workspace_template.format(task_id=task.id, variant=variant, run=run)
    )
    out_dir = Path(args.results_dir) / task.id / variant / f"run_{run}"
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
    write_json(out_dir / "task.json", JsonDocument(json_value(task)))

    if args.prepare_command_template:
        command = render(args.prepare_command_template, ctx, prompt_file)
        prepared = run_command(command, None, args.prepare_timeout)
        write_json(
            out_dir / "prepare.json", JsonDocument(json_value(prepared))
        )

    start_state = git_state(workspace)
    write_json(
        out_dir / "workspace_before.json",
        JsonDocument(json_value(start_state)),
    )
    if not args.allow_dirty_start:
        assert_clean_start(workspace, start_state)

    before = sense_scan(args.sense_bin, workspace, diff=False)
    write_json(
        out_dir / "sensez_before.json", JsonDocument(json_value(before))
    )

    agent_env = Environment(
        {"XDG_CONFIG_HOME": oc_config_home, "XDG_DATA_HOME": oc_data_home}
    )
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
    write_json(
        out_dir / "agent.json", JsonDocument(json_value(agent_result))
    )

    after = sense_scan(args.sense_bin, workspace, diff=False)
    diff_scan = sense_scan(args.sense_bin, workspace, diff=True)
    write_json(
        out_dir / "workspace_after.json",
        JsonDocument(json_value(git_state(workspace))),
    )
    write_json(
        out_dir / "sensez_after.json", JsonDocument(json_value(after))
    )
    write_json(
        out_dir / "sensez_diff.json", JsonDocument(json_value(diff_scan))
    )

    (out_dir / "patch.diff").write_text(git_diff(workspace))
    stats = diff_stats(workspace)

    before_counts = count_findings(before.payload)
    after_counts = count_findings(after.payload)

    test_result = None
    if args.test_command_template:
        command = render(args.test_command_template, ctx, prompt_file)
        test_result = run_command(command, workspace, args.test_timeout)
        write_json(
            out_dir / "test.json", JsonDocument(json_value(test_result))
        )

    tokens = parse_tokens(agent_result.stdout)
    quality = score_payload(diff_scan.payload)
    metrics = BenchmarkMetrics(
        task_id=task.id,
        variant=variant,
        run=run,
        agent_returncode=agent_result.returncode,
        agent_elapsed_seconds=agent_result.elapsed_seconds,
        agent_timed_out=agent_result.timed_out,
        sensez_before=before_counts,
        sensez_after=after_counts,
        sensez_diff=count_findings(diff_scan.payload),
        sensez_delta_total=after_counts.total - before_counts.total,
        quality=quality,
        sensez_tool_calls=count_sensez_tool_calls(agent_result.stdout),
        input_tokens=tokens.input,
        output_tokens=tokens.output,
        reasoning_tokens=tokens.reasoning,
        diff_stats=stats,
        test_returncode=test_result.returncode if test_result else None,
        test_timed_out=test_result.timed_out if test_result else None,
    )
    write_json(
        out_dir / "metrics.json", JsonDocument(json_value(metrics))
    )


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
        run_parallel(args, tasks)
    else:
        run_serial(args, tasks)


def run_parallel(args: argparse.Namespace, tasks: list[TaskSpec]) -> None:
    from concurrent.futures import ThreadPoolExecutor, as_completed

    jobs = [
        (task, variant, run_num)
        for task in tasks
        for run_num in range(1, args.runs + 1)
        for variant in args.variants
    ]
    with ThreadPoolExecutor(max_workers=len(args.variants)) as executor:
        futures = {
            executor.submit(run_one, args, task, variant, run_num): (
                task.id,
                variant,
                run_num,
            )
            for task, variant, run_num in jobs
        }
        for future in as_completed(futures):
            task_id, variant, run_num = futures[future]
            try:
                future.result()
                print(f"  OK  {task_id}/{variant}/run_{run_num}")
            except Exception as error:
                print(f"  FAIL {task_id}/{variant}/run_{run_num}: {error}")


def run_serial(args: argparse.Namespace, tasks: list[TaskSpec]) -> None:
    for task in tasks:
        for run_num in range(1, args.runs + 1):
            for variant in args.variants:
                run_one(args, task, variant, run_num)


if __name__ == "__main__":
    main()
