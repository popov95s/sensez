from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


@dataclass(frozen=True)
class PullRequest:
    number: int
    base_sha: str
    head_sha: str


def pull_request_from_event(event_path: Optional[Path]) -> Optional[PullRequest]:
    if event_path is None or not event_path.exists():
        return None
    event = json.loads(event_path.read_text())
    pull = event.get("pull_request")
    if not pull:
        return None
    return PullRequest(
        number=int(pull["number"]),
        base_sha=pull["base"]["sha"],
        head_sha=pull["head"]["sha"],
    )
