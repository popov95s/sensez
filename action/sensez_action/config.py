from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Mapping, Optional


class ConfigError(Exception):
    pass


DEFAULT_RELATIVE_PATH = "."
DEFAULT_VERSION = "latest"


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
        workspace = _workspace_from_env(env)
        level = env.get("INPUT_LEVEL", "warning").strip().lower() or "warning"
        if level not in {"notice", "warning", "error"}:
            raise ConfigError("level must be one of: notice, warning, error")

        fail_on_new = env.get("INPUT_FAIL_ON_NEW", "").strip().lower().replace("-", "_")
        if fail_on_new and fail_on_new not in {"must_fix", "warning", "advisory", "info"}:
            raise ConfigError("fail-on-new must be one of: must_fix, warning, advisory, info")

        path = _path_from_env(env)
        if not path.is_absolute():
            path = workspace / path

        event = env.get("GITHUB_EVENT_PATH", "").strip()
        return cls(
            workspace=workspace,
            path=path,
            version=_version_from_env(env),
            threshold=env.get("INPUT_THRESHOLD", "").strip(),
            with_comments=_truthy(_first_text(env, "INPUT_WITH_COMMENTS")),
            fail_on_new=fail_on_new,
            level=level,
            token=env.get("GITHUB_TOKEN", "").strip(),
            event_path=Path(event) if event else None,
            repository=env.get("GITHUB_REPOSITORY", "").strip(),
            api_url=env.get("GITHUB_API_URL", "https://api.github.com").rstrip("/"),
            server_url=env.get("GITHUB_SERVER_URL", "https://github.com").rstrip("/"),
        )


def _truthy(value: str | None) -> bool:
    if value is None:
        return False
    return value.strip().lower() in {"1", "true", "yes", "y", "on"}


def _workspace_from_env(env: Mapping[str, str]) -> Path:
    value = _first_text(env, "SENSEZ_WORKSPACE")
    if value is None:
        value = _first_text(env, "GITHUB_WORKSPACE")
    return Path(value) if value is not None else Path(DEFAULT_RELATIVE_PATH)


def _path_from_env(env: Mapping[str, str]) -> Path:
    value = _first_text(env, "INPUT_PATH")
    path_text = value if value is not None else DEFAULT_RELATIVE_PATH
    return Path(path_text)


def _version_from_env(env: Mapping[str, str]) -> str:
    value = _first_text(env, "INPUT_VERSION")
    return value if value is not None else DEFAULT_VERSION


def _first_text(env: Mapping[str, str], key: str) -> str | None:
    value = env.get(key)
    if value is None:
        return None
    text = value.strip()
    return text if text else None
