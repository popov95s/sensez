from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Mapping, Optional


class ConfigError(Exception):
    pass


@dataclass(frozen=True)
class Config:
    workspace: Path
    path: Path
    version: str
    threshold: str
    with_comments: bool
    fail_on_new: str
    level: str
    token: str
    event_path: Optional[Path]
    repository: str
    api_url: str
    server_url: str

    @classmethod
    def from_env(cls, env: Mapping[str, str]) -> "Config":
        workspace = Path(env.get("SENSEZ_WORKSPACE") or env.get("GITHUB_WORKSPACE") or ".")
        level = env.get("INPUT_LEVEL", "warning").strip().lower() or "warning"
        if level not in {"notice", "warning", "error"}:
            raise ConfigError("level must be one of: notice, warning, error")

        fail_on_new = env.get("INPUT_FAIL_ON_NEW", "").strip().lower().replace("-", "_")
        if fail_on_new and fail_on_new not in {"must_fix", "warning", "advisory", "info"}:
            raise ConfigError("fail-on-new must be one of: must_fix, warning, advisory, info")

        path = Path(env.get("INPUT_PATH", ".").strip() or ".")
        if not path.is_absolute():
            path = workspace / path

        event = env.get("GITHUB_EVENT_PATH", "").strip()
        return cls(
            workspace=workspace,
            path=path,
            version=env.get("INPUT_VERSION", "latest").strip() or "latest",
            threshold=env.get("INPUT_THRESHOLD", "").strip(),
            with_comments=_truthy(env.get("INPUT_WITH_COMMENTS", "false")),
            fail_on_new=fail_on_new,
            level=level,
            token=env.get("GITHUB_TOKEN", "").strip(),
            event_path=Path(event) if event else None,
            repository=env.get("GITHUB_REPOSITORY", "").strip(),
            api_url=env.get("GITHUB_API_URL", "https://api.github.com").rstrip("/"),
            server_url=env.get("GITHUB_SERVER_URL", "https://github.com").rstrip("/"),
        )


def _truthy(value: str) -> bool:
    return value.strip().lower() in {"1", "true", "yes", "y", "on"}
