from __future__ import annotations

from .config import Config
from .context import pull_request_from_event
from .diff import changed_lines_from_files
from .findings import Finding
from .github import GitHubClient, GitHubError


class CommentError(Exception):
    pass


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
            line = _comment_line(finding, changed.get(finding.file, set()))
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

def _existing_markers(client: GitHubClient, pull_number: int) -> set[str]:
    comments = client.paged(f"pulls/{pull_number}/comments?per_page=100")
    return {
        marker
        for comment in comments
        for marker in _markers_in(comment.get("body", ""))
    }


def _markers_in(body: str) -> list[str]:
    return [line.strip() for line in body.splitlines() if line.strip().startswith("<!-- sensez:")]


def _comment_line(finding: Finding, changed: set[int]) -> Optional[int]:
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
