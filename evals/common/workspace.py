#!/usr/bin/env python3
"""Workspace checks for agent eval runs."""

from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class WorkspaceState:
    head: str
    status: tuple[str, ...]
    head_returncode: int
    status_returncode: int
    head_stderr: str
    status_stderr: str


def _git(workspace: Path, args: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=workspace,
        text=True,
        capture_output=True,
        check=False,
    )


def git_state(workspace: Path) -> WorkspaceState:
    head = _git(workspace, ["rev-parse", "HEAD"])
    status = _git(workspace, ["status", "--short"])
    return WorkspaceState(
        head=head.stdout.strip(),
        status=tuple(status.stdout.splitlines()),
        head_returncode=head.returncode,
        status_returncode=status.returncode,
        head_stderr=head.stderr,
        status_stderr=status.stderr,
    )


def assert_clean_start(workspace: Path, state: WorkspaceState) -> None:
    if state.head_returncode != 0 or state.status_returncode != 0:
        raise SystemExit(f"{workspace} is not a readable git workspace")
    if state.status:
        changed = "\n".join(state.status)
        raise SystemExit(f"{workspace} is dirty before agent run:\n{changed}")
