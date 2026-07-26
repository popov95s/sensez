from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import NotRequired, TypedDict

from ..mcp_client import McpClient


class Target(TypedDict):
    name: str
    profile: str
    url: str
    commit: str
    scenarios: list[str]
    setup: NotRequired[list[str]]


class DeadCodeFixture(TypedDict):
    path: str
    symbol: str
    detector: str
    text: str
    fix_text: str


class ProfileConfig(TypedDict):
    dead_code_fixture: DeadCodeFixture


class RegressionConfig(TypedDict):
    cache_root: str
    targets: list[Target]
    profiles: dict[str, ProfileConfig]


@dataclass(frozen=True)
class RegressionRun:
    sensez: Path
    config: RegressionConfig
    target: Target
    cache: Path
    out: Path

    @property
    def fixture(self) -> DeadCodeFixture:
        profile = self.config["profiles"][self.target["profile"]]
        return profile["dead_code_fixture"]


@dataclass(frozen=True)
class FixtureSession:
    client: McpClient
    repo: Path
    fixture: DeadCodeFixture
    target_name: str
