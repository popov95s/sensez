from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence


CommandArg = str | Path | int


@dataclass(frozen=True)
class CommandOutput:
    returncode: int
    stdout: str
    stderr: str


def run_json(command: Sequence[CommandArg], cwd: Path) -> object:
    output = run(command, cwd, capture=True)
    if output is None:
        raise RuntimeError("command produced no output")
    return json.loads(output)


def run_captured(command: Sequence[CommandArg], cwd: Path) -> CommandOutput:
    proc = subprocess.run(
        [str(part) for part in command],
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return CommandOutput(proc.returncode, proc.stdout, proc.stderr)


def run(
    command: Sequence[CommandArg],
    cwd: Path,
    capture: bool = False,
    check: bool = True,
) -> str | None:
    text_command = [str(part) for part in command]
    proc = subprocess.run(
        text_command,
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE,
    )
    if check and proc.returncode != 0:
        rendered = " ".join(text_command)
        raise RuntimeError(
            f"command failed ({proc.returncode}): {rendered}\n{proc.stderr}"
        )
    return proc.stdout
