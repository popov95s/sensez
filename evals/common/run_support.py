"""I/O and parsing support for the A/B evaluation runner."""

from __future__ import annotations

import json
import os
import re
import shlex
import subprocess
import time
from pathlib import Path

from models import ScanPayload, TaskSpec, TokenUsage
from run_models import (
    CommandResult,
    Environment,
    JsonDocument,
    RunContext,
    ScanResult,
)

MISSING_TEXT = ""


def load_tasks(path: Path, limit: int | None) -> list[TaskSpec]:
    tasks = [
        TaskSpec.parse(json.loads(line))
        for line in path.read_text().splitlines()
        if line.strip()
    ]
    return tasks[:limit] if limit else tasks


def render(template: str, context: RunContext, prompt_file: Path) -> str:
    return template.format(
        workspace=str(context.workspace),
        prompt_file=str(prompt_file),
        task_id=context.task.id,
        repo=context.task.repo,
        base_commit=context.task.base_commit,
        test_command=context.task.test_command,
        variant=context.variant,
        run=context.run,
        env_config_home=context.oc_config_home,
        env_data_home=context.oc_data_home,
    )


def load_prompt_text(prompt_file: Path) -> str:
    lines = prompt_file.read_text().splitlines()
    if lines and lines[0].startswith("@include "):
        include_target = lines[0].split(maxsplit=1)[1].strip()
        base_file = (prompt_file.parent / include_target).resolve()
        base_text = base_file.read_text().rstrip()
        remainder = "\n".join(lines[1:]).lstrip("\n").rstrip()
        return (
            base_text + "\n\n" + remainder + "\n"
            if remainder
            else base_text + "\n"
        )
    return prompt_file.read_text().rstrip() + "\n"


def run_command(
    command: str,
    cwd: Path | None,
    timeout: int,
    stdin: str | None = None,
    extra_env: Environment | None = None,
    use_shell: bool = False,
) -> CommandResult:
    started = time.monotonic()
    merged_env = os.environ.copy()
    if extra_env:
        merged_env.update(extra_env.values)
    try:
        process = subprocess.run(
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
        return CommandResult(
            command,
            process.returncode,
            round(time.monotonic() - started, 3),
            process.stdout,
            process.stderr,
            False,
        )
    except subprocess.TimeoutExpired as error:
        return CommandResult(
            command,
            None,
            round(time.monotonic() - started, 3),
            _timeout_text(error.stdout),
            _timeout_text(error.stderr),
            True,
        )


def _timeout_text(value: str | bytes | None) -> str:
    if isinstance(value, bytes):
        return value.decode(errors="replace")
    return MISSING_TEXT if value is None else value


def count_sensez_tool_calls(agent_stdout: str) -> int:
    patterns = [
        r'\\?"tool\\?"\s*:\s*\\?"sensez_noze_sniff\\?"',
        r'\\?"tool\\?"\s*:\s*\\?"sensez_noze_gate\\?"',
    ]
    return sum(
        len(re.findall(pattern, agent_stdout)) for pattern in patterns
    )


def parse_tokens(agent_stdout: str) -> TokenUsage:
    input_tokens = output_tokens = reasoning_tokens = total_tokens = 0
    for line in agent_stdout.splitlines():
        if '"step_finish"' not in line and '"type":"step_finish"' not in line:
            continue
        try:
            tokens = json.loads(line).get("part", {}).get("tokens", {})
        except json.JSONDecodeError:
            continue
        input_tokens += tokens.get("input", 0)
        output_tokens += tokens.get("output", 0)
        reasoning_tokens += tokens.get("reasoning", 0)
        total_tokens = max(total_tokens, tokens.get("total", 0))
    return TokenUsage(input_tokens, output_tokens, reasoning_tokens, total_tokens)


def sense_scan(sense_bin: str, workspace: Path, diff: bool) -> ScanResult:
    args = [sense_bin, "noze", str(workspace), "--json"]
    if diff:
        args.append("--diff")
    process = subprocess.run(args, text=True, capture_output=True, check=False)
    try:
        payload = ScanPayload.parse(json.loads(process.stdout))
        stdout = None
    except json.JSONDecodeError:
        payload = ScanPayload()
        stdout = process.stdout
    return ScanResult(
        tuple(args), process.returncode, process.stderr, payload, stdout
    )


def git_diff(workspace: Path) -> str:
    process = subprocess.run(
        ["git", "diff", "--binary"],
        cwd=workspace,
        text=True,
        capture_output=True,
        check=False,
    )
    return process.stdout


def write_json(path: Path, document: JsonDocument) -> None:
    document.write(path)


def build_prompt(
    base_prompt: Path, task: TaskSpec, destination: Path
) -> None:
    body = [
        load_prompt_text(base_prompt).rstrip(),
        "",
        "Benchmark task:",
        f"- id: {task.id}",
        f"- repo: {task.repo}",
        f"- category: {task.category}",
        f"- summary: {task.summary}",
    ]
    destination.write_text("\n".join(body) + "\n")
