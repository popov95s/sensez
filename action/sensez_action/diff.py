from __future__ import annotations

import re
from collections import defaultdict


ChangedLines = dict[str, set[int]]


HUNK_RE = re.compile(r"@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@")
FILE_RE = re.compile(r"^\+\+\+ b/(.+)$")


def changed_lines_from_files(files: list[dict]) -> ChangedLines:
    changed: ChangedLines = defaultdict(set)
    for file_info in files:
        path = file_info.get("filename")
        patch = file_info.get("patch")
        if not path or not patch:
            continue
        for line in added_lines(patch):
            changed[path].add(line)
    return dict(changed)


def changed_lines_from_git_diff(diff_text: str) -> ChangedLines:
    changed: ChangedLines = defaultdict(set)
    current_file = ""
    current_patch: list[str] = []
    for row in diff_text.splitlines():
        match = FILE_RE.match(row)
        if match:
            _flush_patch(changed, current_file, current_patch)
            current_file = match.group(1)
            current_patch = []
            continue
        if current_file:
            current_patch.append(row)
    _flush_patch(changed, current_file, current_patch)
    return dict(changed)


def added_lines(patch: str) -> list[int]:
    result: list[int] = []
    new_line = 0
    for row in patch.splitlines():
        match = HUNK_RE.match(row)
        if match:
            new_line = int(match.group(1))
            continue
        if row.startswith("+") and not row.startswith("+++"):
            result.append(new_line)
            new_line += 1
        elif row.startswith("-") and not row.startswith("---"):
            continue
        elif new_line:
            new_line += 1
    return result


def _flush_patch(changed: ChangedLines, path: str, patch: list[str]) -> None:
    if not path or path == "/dev/null":
        return
    for line in added_lines("\n".join(patch)):
        changed[path].add(line)
