from pathlib import Path

from ..branch_metrics.assertions import assert_exact_transition_count
from ..branch_metrics.stability import (
    reported_stable_across_branch_switches,
)
from ..harness.artifacts import dump_metrics_schema, dump_normalized
from ..harness.assertions import (
    assert_brainz_totals_reported,
    assert_finding_reintroduced,
    assert_finding_resolved,
    assert_gate_allows,
    assert_gate_blocks,
)
from ..harness.fixtures import apply_fixture
from ..harness.models import DeadCodeFixture, RegressionRun
from ..harness.repositories import cleanup_repo, scenario_repo
from ..mcp_client import McpClient, text_json


def run_mcp_scenarios(context: RegressionRun) -> None:
    repo = scenario_repo(context.cache, context.target)
    client = McpClient(context.sensez)
    try:
        _verify_protocol(client, context, repo)
        _record_full_and_limited_scans(client, context, repo)
        _exercise_gate_lifecycle(client, context, repo)
        _record_metrics_outputs(client, context, repo)
    finally:
        client.close()
        cleanup_repo(repo)


def _verify_protocol(client: McpClient, context: RegressionRun, repo: Path) -> None:
    initialized = client.request("initialize")["result"]
    assert initialized["serverInfo"]["name"] == "sensez"
    tools = client.request("tools/list")["result"]
    _dump(context, repo, "mcp.tools.json", tools)


def _record_full_and_limited_scans(
    client: McpClient, context: RegressionRun, repo: Path
) -> None:
    full = text_json(
        client.call_tool(
            "noze_sniff",
            {"path": str(repo), "diff": False},
        )
    )
    _dump(context, repo, "mcp.full.noze.json", full)
    limited = text_json(
        client.call_tool(
            "noze_sniff",
            {
                "path": str(repo),
                "limit": 20,
                "diff": False,
                "record": False,
            },
        )
    )
    _dump(context, repo, "mcp.limited.noze.json", limited)
    report = _brainz_report(client, repo)
    _dump(context, repo, "brainz.after-full.json", report)
    assert_brainz_totals_reported(report, context.target["name"])


def _exercise_gate_lifecycle(
    client: McpClient, context: RegressionRun, repo: Path
) -> None:
    fixture = context.fixture
    apply_fixture(repo, fixture, fixture["text"])
    diff = text_json(client.call_tool("noze_sniff", {"path": str(repo), "diff": True}))
    _dump(context, repo, "diff.noze.json", diff)

    blocked = text_json(client.call_tool("noze_gate", {"path": str(repo)}))
    _dump(context, repo, "gate.block.json", blocked)
    assert_gate_blocks(blocked, context.target["name"])

    same = text_json(client.call_tool("noze_gate", {"path": str(repo)}))
    _dump(context, repo, "gate.allow-same-content.json", same)
    assert_gate_allows(same, "signature dedup")

    active = text_json(
        client.call_tool(
            "noze_gate",
            {"path": str(repo), "stop_hook_active": True},
        )
    )
    _dump(context, repo, "gate.allow.json", active)
    assert_gate_allows(active, "stop_hook_active")

    _exercise_fix_and_reintroduction(client, context, repo, fixture)
    if "triage" in context.target.get("scenarios", []):
        _triage(client, repo, fixture)

    deferred = text_json(client.call_tool("noze_gate", {"path": str(repo)}))
    _dump(context, repo, "gate.defer.json", deferred)
    assert_gate_allows(deferred, "auto-deferred past repeat_limit")


def _exercise_fix_and_reintroduction(
    client: McpClient,
    context: RegressionRun,
    repo: Path,
    fixture: DeadCodeFixture,
) -> None:
    apply_fixture(repo, fixture, fixture["fix_text"])
    client.call_tool("noze_sniff", {"path": str(repo), "limit": 20})
    fixed = _brainz_report(client, repo)
    _dump(context, repo, "brainz.after-gate-fix.json", fixed)
    assert_finding_resolved(
        fixed,
        fixture["detector"],
        context.target["name"],
    )
    assert_exact_transition_count(
        fixed,
        fixture["detector"],
        context.target["name"],
        resolved=1,
        reintroduced=0,
    )

    apply_fixture(repo, fixture, fixture["text"])
    client.call_tool("noze_sniff", {"path": str(repo), "limit": 20})
    reintroduced = _brainz_report(client, repo)
    _dump(context, repo, "brainz.after-reintro.json", reintroduced)
    assert_finding_reintroduced(
        reintroduced,
        fixture["detector"],
        context.target["name"],
    )
    assert_exact_transition_count(
        reintroduced,
        fixture["detector"],
        context.target["name"],
        resolved=1,
        reintroduced=1,
    )


def _record_metrics_outputs(
    client: McpClient, context: RegressionRun, repo: Path
) -> None:
    dump_metrics_schema(context.out / "metrics-files.schema.json", repo)
    report = reported_stable_across_branch_switches(
        client,
        repo,
        context.target["name"],
    )
    _dump(context, repo, "brainz.after-branch-switch.json", report)


def _brainz_report(client: McpClient, repo: Path) -> object:
    return text_json(client.call_tool("brainz_report", {"path": str(repo)}))


def _triage(client: McpClient, repo: Path, fixture: DeadCodeFixture) -> None:
    client.call_tool(
        "brainz_triage",
        {
            "path": str(repo),
            "pillar": "dead_code",
            "match": fixture["symbol"],
            "verdict": "debt",
            "note": "regression fixture",
        },
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
