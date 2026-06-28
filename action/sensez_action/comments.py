from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

from .config import Config
from .context import pull_request_from_event
from .diff import changed_lines_from_files
from .findings import Finding
from .github import GitHubClient, GitHubError


class CommentError(Exception):
    pass


@dataclass(frozen=True)
class CommentMarkers:
    values: frozenset[str]

    def __contains__(self, marker: object) -> bool:
        return isinstance(marker, str) and marker in self.values


@dataclass(frozen=True)
class ChangedLineSet:
    values: frozenset[int]

    def __contains__(self, line: object) -> bool:
        return isinstance(line, int) and line in self.values


def post_comments(findings: list[Finding], config: Config) -> None:
    pull = pull_request_from_event(config.event_path)
    if pull is None:
        print("Sensez comments requested, but this workflow is not running on a pull request.")
        return
    if not config.token or not config.repository:
        raise CommentError("with-comments requires GITHUB_TOKEN and GITHUB_REPOSITORY")

    client = GitHubClient(config.api_url, config.repository, config.token)
    try:
        files = client.paged(f"pulls/{pull.number}/files?per_page=100")
        changed = changed_lines_from_files(files)
        existing = _existing_markers(client, pull.number)
        for finding in findings:
            line = _comment_line(
                finding,
                ChangedLineSet(frozenset(changed.get(finding.file, ()))),
            )
            if line is None or finding.marker in existing:
                continue
            client.post(
                f"pulls/{pull.number}/comments",
                {
                    "body": _comment_body(finding),
                    "commit_id": pull.head_sha,
                    "path": finding.file,
                    "line": line,
                    "side": "RIGHT",
                },
            )
    except GitHubError as error:
        raise CommentError(str(error)) from error


def _existing_markers(client: GitHubClient, pull_number: int) -> CommentMarkers:
    comments = client.paged(f"pulls/{pull_number}/comments?per_page=100")
    markers = frozenset(
        line.strip()
        for comment in comments
        for line in str(comment.get("body", "")).splitlines()
        if line.strip().startswith("<!-- sensez:")
    )
    return CommentMarkers(markers)


def _comment_line(finding: Finding, changed: ChangedLineSet) -> Optional[int]:
    for line in range(finding.start_line, finding.end_line + 1):
        if line in changed:
            return line
    return None


def _comment_body(finding: Finding) -> str:
    return "\n".join(
        [
            finding.marker,
            "Sensez found duplicated structure here.",
            "",
            finding.message,
            "",
            "Resolve the duplication before merging, or adjust the Sensez "
            "threshold if this is intentional.",
        ]
    )
