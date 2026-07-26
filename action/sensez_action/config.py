from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Optional


class ConfigError(Exception):
    pass


DEFAULT_RELATIVE_PATH = "."
DEFAULT_VERSION = "latest"


@dataclass(frozen=True)
class ActionEnvironment:
    values: dict[str, str]

    def text(self, key: str) -> str | None:
        value = self.values.get(key)
        if value is None:
            return None
        text = value.strip()
        return text if text else None


class AnnotationLevel(str, Enum):
    NOTICE = "notice"
    WARNING = "warning"
    ERROR = "error"

    @classmethod
    def parse(cls, value: str | None) -> "AnnotationLevel":
        normalized = (value or cls.WARNING.value).strip().lower() or cls.WARNING.value
        try:
            return cls(normalized)
        except ValueError as error:
            raise ConfigError(
                "level must be one of: notice, warning, error"
            ) from error


class FailureLevel(str, Enum):
    DISABLED = ""
    MUST_FIX = "must_fix"
    WARNING = "warning"
    ADVISORY = "advisory"
    INFO = "info"

    @classmethod
    def parse(cls, value: str | None) -> "FailureLevel":
        normalized = (value or "").strip().lower().replace("-", "_")
        try:
            return cls(normalized)
        except ValueError as error:
            raise ConfigError(
                "fail-on-new must be one of: must_fix, warning, advisory, info"
            ) from error


@dataclass(frozen=True)
class Config:
    workspace: Path
    path: Path
    version: str
    threshold: str
    with_comments: bool
    fail_on_new: FailureLevel
    level: AnnotationLevel
    token: str
    event_path: Optional[Path]
    repository: str
    api_url: str
    server_url: str

    @classmethod
    def from_env(cls, env: ActionEnvironment) -> "Config":
        workspace = _workspace_from_env(env)
        level = AnnotationLevel.parse(env.text("INPUT_LEVEL"))
        fail_on_new = FailureLevel.parse(env.text("INPUT_FAIL_ON_NEW"))

        path = _path_from_env(env)
        if not path.is_absolute():
            path = workspace / path

        event = env.text("GITHUB_EVENT_PATH")
        return cls(
            workspace=workspace,
            path=path,
            version=_version_from_env(env),
            threshold=env.text("INPUT_THRESHOLD") or "",
            with_comments=_truthy(env.text("INPUT_WITH_COMMENTS")),
            fail_on_new=fail_on_new,
            level=level,
            token=env.text("GITHUB_TOKEN") or "",
            event_path=Path(event) if event else None,
            repository=env.text("GITHUB_REPOSITORY") or "",
            api_url=(env.text("GITHUB_API_URL") or "https://api.github.com").rstrip("/"),
            server_url=(env.text("GITHUB_SERVER_URL") or "https://github.com").rstrip("/"),
        )


def _truthy(value: str | None) -> bool:
    if value is None:
        return False
    return value.strip().lower() in {"1", "true", "yes", "y", "on"}


def _workspace_from_env(env: ActionEnvironment) -> Path:
    value = env.text("SENSEZ_WORKSPACE")
    if value is None:
        value = env.text("GITHUB_WORKSPACE")
    return Path(value) if value is not None else Path(DEFAULT_RELATIVE_PATH)


def _path_from_env(env: ActionEnvironment) -> Path:
    value = env.text("INPUT_PATH")
    path_text = value if value is not None else DEFAULT_RELATIVE_PATH
    return Path(path_text)


def _version_from_env(env: ActionEnvironment) -> str:
    value = env.text("INPUT_VERSION")
    return value if value is not None else DEFAULT_VERSION
