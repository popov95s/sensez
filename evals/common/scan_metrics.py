#!/usr/bin/env python3
"""Helpers for comparing Sensez scans."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Any


PILLARS = ("cycles", "dead_code", "boundaries", "duplication", "smells")


def count_findings(scan: dict[str, Any]) -> dict[str, int]:
    payload = scan.get("json") or {}
    counts = {pillar: len(payload.get(pillar) or []) for pillar in PILLARS}
    counts["total"] = sum(counts.values())
    return counts


def diff_stats(workspace: Path) -> dict[str, Any]:
    proc = subprocess.run(["git", "diff", "--numstat"], cwd=workspace, text=True, capture_output=True, check=False)
    files = []
    added = deleted = 0
    for line in proc.stdout.splitlines():
        parts = line.split("\t")
        if len(parts) != 3:
            continue
        add, delete, file_name = parts
        files.append(file_name)
        added += 0 if add == "-" else int(add)
        deleted += 0 if delete == "-" else int(delete)
    return {"files": files, "files_touched": len(files), "lines_added": added, "lines_deleted": deleted}
