from __future__ import annotations

import json
import os
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


@dataclass(frozen=True)
class CommandEnvironment:
    values: tuple[tuple[str, str], ...]

    def merged(self):
        return {**os.environ, **dict(self.values)}


def run_json(
    command: Sequence[CommandArg],
    cwd: Path,
    env: CommandEnvironment | None = None,
) -> object:
    output = run(command, cwd, capture=True, env=env)
    if output is None:
        raise RuntimeError("command produced no output")
    return json.loads(output)


def run_captured(
    command: Sequence[CommandArg],
    cwd: Path,
    env: CommandEnvironment | None = None,
) -> CommandOutput:
    proc = subprocess.run(
        [str(part) for part in command],
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=None if env is None else env.merged(),
    )
    return CommandOutput(proc.returncode, proc.stdout, proc.stderr)


def run(
    command: Sequence[CommandArg],
    cwd: Path,
    capture: bool = False,
    check: bool = True,
    env: CommandEnvironment | None = None,
) -> str | None:
    text_command = [str(part) for part in command]
    proc = subprocess.run(
        text_command,
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE,
        env=None if env is None else env.merged(),
    )
    if check and proc.returncode != 0:
        rendered = " ".join(text_command)
        raise RuntimeError(
            f"command failed ({proc.returncode}): {rendered}\n{proc.stderr}"
        )
    return proc.stdout
