from __future__ import annotations

import re
from collections import defaultdict
from dataclasses import dataclass, field
from typing import TypedDict


ChangedLines = dict[str, tuple[int, ...]]


class GitHubFile(TypedDict, total=False):
    filename: str
    patch: str


@dataclass
class ChangedLineIndex:
    files: dict[str, set[int]] = field(default_factory=lambda: defaultdict(set))

    def add_patch(self, path: str, patch: str) -> None:
        if not path or path == "/dev/null":
            return
        for line in added_lines(patch):
            self.files[path].add(line)

    def freeze(self) -> ChangedLines:
        return {path: tuple(sorted(lines)) for path, lines in self.files.items()}


HUNK_RE = re.compile(r"@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@")
FILE_RE = re.compile(r"^\+\+\+ b/(.+)$")


def changed_lines_from_files(files: list[GitHubFile]) -> ChangedLines:
    changed = ChangedLineIndex()
    for file_info in files:
        path = file_info.get("filename")
        patch = file_info.get("patch")
        if not path or not patch:
            continue
        changed.add_patch(path, patch)
    return changed.freeze()


def changed_lines_from_git_diff(diff_text: str) -> ChangedLines:
    changed = ChangedLineIndex()
    current_file = ""
    current_patch: list[str] = []
    for row in diff_text.splitlines():
        match = FILE_RE.match(row)
        if match:
            changed.add_patch(current_file, "\n".join(current_patch))
            current_file = match.group(1)
            current_patch = []
            continue
        if current_file:
            current_patch.append(row)
    changed.add_patch(current_file, "\n".join(current_patch))
    return changed.freeze()


def added_lines(patch: str) -> tuple[int, ...]:
    result: set[int] = set()
    new_line = 0
    for row in patch.splitlines():
        match = HUNK_RE.match(row)
        if match:
            new_line = int(match.group(1))
            continue
        if row.startswith("+") and not row.startswith("+++"):
            result.add(new_line)
            new_line += 1
        elif row.startswith("-") and not row.startswith("---"):
            continue
        elif new_line:
            new_line += 1
    return tuple(sorted(result))

