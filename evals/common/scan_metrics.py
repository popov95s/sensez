#!/usr/bin/env python3
"""Helpers for comparing Sensez scans."""

from __future__ import annotations

import subprocess
from pathlib import Path

from models import DiffStats, FindingCounts, ScanPayload


def count_findings(scan: ScanPayload) -> FindingCounts:
    return FindingCounts.from_scan(scan)


def diff_stats(workspace: Path) -> DiffStats:
    proc = subprocess.run(
        ["git", "diff", "--numstat"],
        cwd=workspace,
        text=True,
        capture_output=True,
        check=False,
    )
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
    return DiffStats(tuple(files), len(files), added, deleted)
