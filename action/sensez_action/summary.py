from __future__ import annotations

import os

from .config import Config
from .findings import Finding


def write_summary(findings: list[Finding], config: Config) -> None:
    path = os.environ.get("GITHUB_STEP_SUMMARY")
    if not path:
        return
    lines = [
        "## Sensez",
        "",
        f"Duplication annotations: {len(findings)}",
        f"Inline comments: {'enabled' if config.with_comments else 'disabled'}",
    ]
    if findings:
        lines.extend(["", "| File | Lines |", "|---|---|"])
        for finding in findings[:20]:
            lines.append(f"| `{finding.file}` | {finding.start_line}-{finding.end_line} |")
        if len(findings) > 20:
            lines.append(f"| ... | {len(findings) - 20} more |")
    with open(path, "a", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")
