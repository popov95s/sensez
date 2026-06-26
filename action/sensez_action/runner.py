from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass
from typing import Any, Optional

from .config import Config
from .context import pull_request_from_event
from .diff import ChangedLines, changed_lines_from_git_diff


class SensezError(Exception):
    pass


@dataclass(frozen=True)
class ScanResult:
    report: dict[str, Any]
    changed_lines: Optional[ChangedLines]


def run_sensez(config: Config) -> ScanResult:
    diff_text = _pr_diff(config)
    command = [
        "uvx",
        "--from",
        _sensez_package(config.version),
        "sensez",
        "noze",
        str(config.path),
        "--duplicates",
        "--json",
        "--all",
    ]
    if diff_text is None:
        command.append("--diff")
    else:
        command.extend(["--diff-from", "-"])
    if config.threshold:
        command.extend(["--threshold", config.threshold])

    completed = subprocess.run(
        command,
        cwd=config.workspace,
        check=False,
        text=True,
        input=diff_text,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if completed.returncode != 0:
        raise SensezError(completed.stderr.strip() or "sensez scan failed")
    try:
        report = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise SensezError(f"sensez returned invalid JSON: {error}") from error
    changed = changed_lines_from_git_diff(diff_text) if diff_text is not None else None
    return ScanResult(report=report, changed_lines=changed)


def _sensez_package(version: str) -> str:
    if not version or version == "latest":
        return "sensez"
    if version.startswith(("sensez", ".", "/", "git+", "http://", "https://")):
        return version
    return f"sensez=={version}"


def _pr_diff(config: Config) -> Optional[str]:
    pull = pull_request_from_event(config.event_path)
    if pull is None:
        return None
    completed = subprocess.run(
        ["git", "diff", "--unified=0", f"{pull.base_sha}...{pull.head_sha}"],
        cwd=config.workspace,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or "git diff failed"
        raise SensezError(
            f"could not compute pull-request diff: {detail}. "
            "Use actions/checkout with fetch-depth: 0."
        )
    return completed.stdout
