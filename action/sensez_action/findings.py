from __future__ import annotations

import hashlib
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Optional

from .diff import ChangedLines


ACTION_ORDER = {"must_fix": 0, "warning": 1, "advisory": 2, "info": 3}


@dataclass(frozen=True)
class Finding:
    file: str
    start_line: int
    end_line: int
    message: str
    token_length: int
    action: str
    marker: str


def flatten_duplication(
    report: dict[str, Any], workspace: Path, changed: Optional[ChangedLines] = None
) -> list[Finding]:
    findings: list[Finding] = []
    for clone_class in report.get("duplication", []):
        occurrences = clone_class.get("occurrences", [])
        token_length = int(clone_class.get("token_length") or 0)
        action = str(clone_class.get("action") or "advisory")
        for occurrence in occurrences:
            path = _relative_path(occurrence.get("file", ""), workspace)
            start = int(occurrence.get("start_row") or 1)
            end = int(occurrence.get("end_row") or start)
            if changed is not None and not _touches_changed(path, start, end, changed):
                continue
            peers = [_peer_text(peer, workspace) for peer in occurrences if peer is not occurrence]
            message = _message(token_length, peers)
            marker = _marker(path, start, end, token_length, peers)
            findings.append(Finding(path, start, end, message, token_length, action, marker))
    return findings


def should_fail(report: dict[str, Any], fail_on_new: str) -> bool:
    if not fail_on_new:
        return False
    threshold = ACTION_ORDER[fail_on_new]
    for clone_class in report.get("duplication", []):
        action = str(clone_class.get("action") or "advisory")
        if ACTION_ORDER.get(action, ACTION_ORDER["advisory"]) <= threshold:
            return True
    return False


def _relative_path(value: str, workspace: Path) -> str:
    path = Path(value)
    if path.is_absolute():
        try:
            return path.relative_to(workspace).as_posix()
        except ValueError:
            return path.as_posix()
    return path.as_posix()


def _peer_text(occurrence: dict[str, Any], workspace: Path) -> str:
    path = _relative_path(occurrence.get("file", ""), workspace)
    start = int(occurrence.get("start_row") or 1)
    end = int(occurrence.get("end_row") or start)
    return f"{path}:{start}-{end}"


def _message(token_length: int, peers: list[str]) -> str:
    suffix = ""
    if peers:
        suffix = " Also appears at " + ", ".join(peers[:5])
        if len(peers) > 5:
            suffix += f", and {len(peers) - 5} more"
    return f"Structural duplication detected ({token_length} tokens).{suffix}"


def _marker(path: str, start: int, end: int, token_length: int, peers: list[str]) -> str:
    raw = "|".join([path, str(start), str(end), str(token_length), *sorted(peers)])
    digest = hashlib.sha256(raw.encode("utf-8")).hexdigest()[:16]
    return f"<!-- sensez:duplication:{digest} -->"


def _touches_changed(path: str, start: int, end: int, changed: ChangedLines) -> bool:
    lines = changed.get(path)
    if not lines:
        return False
    return any(line in lines for line in range(start, end + 1))
