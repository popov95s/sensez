from __future__ import annotations

import json
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Iterator

from ..harness.artifacts import dump_normalized
from ..harness.assertions import (
    FixtureIdentity,
    assert_gate_allows,
    assert_gate_blocks,
    assert_gate_mentions_new_only,
    assert_session_gate_isolated,
)
from ..harness.commands import run
from ..harness.fixtures import (
    apply_fixture,
    extra_dead_code_fixture,
    extra_symbol_for,
)
from ..harness.models import (
    DeadCodeFixture,
    FixtureSession,
    RegressionRun,
)
from ..harness.repositories import cleanup_repo, scenario_repo
from ..mcp_client import McpClient, text_json


@dataclass(frozen=True)
class GateSession(FixtureSession):
    def block_initial_fixture(self) -> None:
        apply_fixture(self.repo, self.fixture, self.fixture["text"])
        response = text_json(
            self.client.call_tool(
                "noze_gate",
                {"path": str(self.repo)},
            )
        )
        assert_gate_blocks(response, self.target_name)


@contextmanager
def gate_session(context: RegressionRun) -> Iterator[GateSession]:
    repo = scenario_repo(context.cache, context.target)
    client = McpClient(context.sensez)
    try:
        client.request("initialize")
        yield GateSession(
            client=client,
            repo=repo,
            fixture=context.fixture,
            target_name=context.target["name"],
        )
    finally:
        client.close()
        cleanup_repo(repo)


def run_gate_reblock_scenario(context: RegressionRun) -> None:
    with gate_session(context) as session:
        session.block_initial_fixture()
        new_symbol = extra_symbol_for(session.fixture)
        extra = extra_dead_code_fixture(session.fixture, new_symbol)
        apply_fixture(session.repo, extra, extra["text"])
        response = text_json(
            session.client.call_tool(
                "noze_gate",
                {"path": str(session.repo)},
            )
        )
        _dump(context, session.repo, "gate.block-new-only.json", response)
        assert_gate_blocks(response, session.target_name)
        assert_gate_mentions_new_only(
            response,
            new_symbol,
            session.fixture["symbol"],
            session.target_name,
        )


def run_gate_detached_scenario(context: RegressionRun) -> None:
    with gate_session(context) as session:
        session.block_initial_fixture()
        same = _gate(session.client, session.repo)
        assert_gate_allows(same, "same branch")

        run(["git", "checkout", "--detach"], session.repo)
        detached = _gate(session.client, session.repo)
        _dump(context, session.repo, "gate.allow-detached.json", detached)
        assert_gate_allows(detached, "detached HEAD")

        run(
            ["git", "checkout", "sensez-regression-worktree"],
            session.repo,
        )
        attached = _gate(session.client, session.repo)
        assert_gate_allows(attached, "reattached branch")


def run_shared_worktree_gate_scenario(context: RegressionRun) -> None:
    repo = scenario_repo(context.cache, context.target)
    first_fixture = context.fixture
    second_fixture = extra_dead_code_fixture(
        first_fixture,
        extra_symbol_for(first_fixture),
    )
    client = McpClient(context.sensez)
    try:
        client.request("initialize")
        apply_fixture(repo, first_fixture, first_fixture["text"])
        apply_fixture(repo, second_fixture, second_fixture["text"])
        first_transcript = _write_transcript(
            repo,
            first_fixture["path"],
            "session-a.jsonl",
        )
        second_transcript = _write_transcript(
            repo,
            second_fixture["path"],
            "session-b.jsonl",
        )
        first = _scoped_gate(
            client,
            repo,
            "claude-session-a",
            first_transcript,
        )
        second = _scoped_gate(
            client,
            repo,
            "claude-session-b",
            second_transcript,
        )
        _assert_isolated(first, first_fixture, second_fixture)
        _assert_isolated(second, second_fixture, first_fixture)
        _dump(
            context,
            repo,
            "gate.shared-worktree.session-a.json",
            first,
        )
        _dump(
            context,
            repo,
            "gate.shared-worktree.session-b.json",
            second,
        )
        repeated = _scoped_gate(
            client,
            repo,
            "claude-session-a",
            first_transcript,
        )
        assert_gate_allows(repeated, "same transcript-scoped finding")
        _dump(
            context,
            repo,
            "gate.shared-worktree.session-a-repeat.json",
            repeated,
        )
    finally:
        client.close()
        cleanup_repo(repo)


def _write_transcript(repo: Path, relative: str, name: str) -> Path:
    path = repo / ".sensez-regression" / name
    path.parent.mkdir(parents=True, exist_ok=True)
    entry = {
        "message": {
            "content": [
                {
                    "type": "tool_use",
                    "name": "Write",
                    "input": {"file_path": str(repo / relative)},
                }
            ]
        }
    }
    path.write_text(json.dumps(entry) + "\n")
    return path


def _scoped_gate(
    client: McpClient,
    repo: Path,
    session_id: str,
    transcript: Path,
) -> object:
    return text_json(
        client.call_tool(
            "noze_gate",
            {
                "path": str(repo),
                "session_id": session_id,
                "transcript_path": str(transcript),
            },
        )
    )


def _gate(client: McpClient, repo: Path) -> object:
    return text_json(client.call_tool("noze_gate", {"path": str(repo)}))


def _assert_isolated(
    response: object,
    owned: DeadCodeFixture,
    other: DeadCodeFixture,
) -> None:
    assert_session_gate_isolated(
        response,
        FixtureIdentity(owned["path"], owned["symbol"]),
        FixtureIdentity(other["path"], other["symbol"]),
    )


def _dump(
    context: RegressionRun,
    repo: Path,
    artifact: str,
    value: object,
) -> None:
    dump_normalized(
        context.out / artifact,
        value,
        repo,
        context.target,
    )
