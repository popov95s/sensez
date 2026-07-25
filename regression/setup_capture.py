from __future__ import annotations

import os
import pty
import re
import select
import subprocess
import time
from pathlib import Path
from typing import Sequence


ANSI = re.compile(r"\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])")


def run(
    command: Sequence[str | Path],
    cwd: Path,
    home: Path,
    paths: dict[str, str],
    remove_home: bool = False,
) -> object:
    text_command = [str(part) for part in command]
    proc = subprocess.run(
        text_command,
        cwd=cwd,
        env=_environment(home, remove_home),
        text=True,
        capture_output=True,
    )
    return {
        "command": [_normalize(part, paths) for part in text_command],
        "exit_code": proc.returncode,
        "stdout": _normalize(proc.stdout, paths),
        "stderr": _normalize(proc.stderr, paths),
    }


def run_pty(
    command: Sequence[str | Path],
    cwd: Path,
    home: Path,
    answer: str,
    paths: dict[str, str],
) -> object:
    master, slave = pty.openpty()
    text_command = [str(part) for part in command]
    proc = subprocess.Popen(
        text_command,
        cwd=cwd,
        env=_environment(home),
        stdin=slave,
        stdout=slave,
        stderr=slave,
        close_fds=True,
    )
    os.close(slave)
    transcript = bytearray()
    answered = False
    deadline = time.monotonic() + 10
    try:
        while time.monotonic() < deadline:
            ready, _, _ = select.select([master], [], [], 0.1)
            if ready:
                try:
                    transcript.extend(os.read(master, 4096))
                except OSError:
                    break
            if not answered and b"Install Sensez globally" in transcript:
                os.write(master, answer.encode())
                answered = True
            if proc.poll() is not None and not ready:
                break
        if proc.poll() is None:
            proc.kill()
            raise RuntimeError("interactive init scenario timed out")
    finally:
        os.close(master)
    return {
        "command": [_normalize(part, paths) for part in text_command],
        "exit_code": proc.wait(),
        "terminal": _normalize(transcript.decode(errors="replace"), paths),
    }


def artifact(
    scenario: str,
    runs: list[object],
    repo: Path,
    home: Path,
    paths: dict[str, str],
) -> object:
    return {
        "scenario": scenario,
        "runs": runs,
        "repo_files": _snapshot(repo, paths),
        "home_files": _snapshot(home, paths),
    }


def replacements(
    sensez: Path, repo: Path, home: Path, cwd: Path
) -> dict[str, str]:
    pairs = {
        str(sensez): "<sensez>",
        str(sensez.resolve()): "<sensez>",
        str(repo): "<repo>",
        str(repo.resolve()): "<repo>",
        str(home): "<home>",
        str(home.resolve()): "<home>",
        str(cwd): "<cwd>",
        str(cwd.resolve()): "<cwd>",
    }
    for source, target in list(pairs.items()):
        if source.startswith("/private/var/"):
            pairs[source.replace("/private/var/", "/var/", 1)] = target
    return dict(sorted(pairs.items(), key=lambda item: len(item[0]), reverse=True))


def seed_global_config(home: Path, agent: str) -> None:
    standard = '{"mcpServers":{"existing":{"command":"/bin/existing","args":[]}}}'
    seeds = {
        "claude-code": (".claude.json", standard),
        "cursor": (".cursor/mcp.json", standard),
        "cline": (".cline/data/settings/cline_mcp_settings.json", standard),
        "codex": (
            ".codex/config.toml",
            '[mcp_servers.existing]\ncommand = "/bin/existing"\nargs = []\n',
        ),
        "opencode": (
            ".config/opencode/opencode.json",
            '{"mcp":{"existing":{"type":"local","command":["/bin/existing"]}}}',
        ),
        "pi": (".pi/agent/mcp.json", standard),
    }
    seed = seeds.get(agent)
    if seed is None:
        return
    path = home / seed[0]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(seed[1])


def git_init(repo: Path) -> None:
    subprocess.run(
        ["git", "init", "-q"],
        cwd=repo,
        text=True,
        capture_output=True,
        check=True,
    )


def _snapshot(root: Path, paths: dict[str, str]) -> dict[str, str]:
    files: dict[str, str] = {}
    for path in sorted(root.rglob("*")):
        rel = path.relative_to(root)
        if ".git" in rel.parts or not path.is_file():
            continue
        files[str(rel)] = _normalize(path.read_text(errors="replace"), paths)
    return files


def _environment(home: Path, remove_home: bool = False) -> dict[str, str]:
    env = os.environ.copy()
    env["NO_COLOR"] = "1"
    if remove_home:
        env.pop("HOME", None)
        env.pop("USERPROFILE", None)
    else:
        env["HOME"] = str(home)
        env.pop("USERPROFILE", None)
    return env


def _normalize(text: str, paths: dict[str, str]) -> str:
    normalized = ANSI.sub("", text).replace("\r\n", "\n").replace("\r", "\n")
    for source, target in paths.items():
        normalized = normalized.replace(source, target)
    return normalized
