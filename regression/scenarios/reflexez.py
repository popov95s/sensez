"""Repository-level changed-test selection regressions for Reflexez."""

from dataclasses import asdict, dataclass
from pathlib import Path
from typing import cast

from ..harness.artifacts import dump_normalized
from ..harness.commands import run, run_json
from ..harness.models import RegressionRun
from ..harness.repositories import cleanup_repo, scenario_repo
from .reflexez_randomized import run_randomized_source_scenario


@dataclass(frozen=True)
class Fixture:
    feature: str
    feature_body: str
    changed: str
    related: str
    related_body: str
    unrelated_source: str
    unrelated_source_body: str
    unrelated_test: str
    unrelated_test_body: str
    global_file: str


@dataclass(frozen=True)
class SelectedTest:
    file: str
    runner: str
    reason: str
    distance: int


@dataclass(frozen=True)
class PlanMetadata:
    discovered_tests: int
    full_suite: bool
    fallback_reasons: tuple[str, ...]
    unresolved_dynamic_imports: int
    runner_kinds: tuple[str, ...]


@dataclass(frozen=True)
class ImpactPlan(PlanMetadata):
    selected: tuple[SelectedTest, ...]


@dataclass(frozen=True)
class ImpactSnapshot(PlanMetadata):
    selected_count: int
    selected_fixtures: tuple[SelectedTest, ...]


def run_reflexez_scenario(context: RegressionRun) -> None:
    repo = scenario_repo(context.cache, context.target)
    fixture = _fixture(context.target["profile"])
    try:
        _write_fixture(repo, fixture)
        _commit_fixture(repo)
        feature = repo / fixture.feature
        feature.write_text(fixture.changed)
        selective = _plan(context, repo)
        _assert_selective(selective, fixture)

        manifest = repo / fixture.global_file
        manifest.write_text(manifest.read_text() + "\n")
        fallback = _plan(context, repo)
        _assert_fallback(fallback)

        dump_normalized(
            context.out / "reflexez.impact.json",
            {
                "selective": asdict(_snapshot(selective, repo)),
                "global_change": asdict(_snapshot(fallback, repo)),
            },
            repo,
            context.target,
        )
    finally:
        cleanup_repo(repo)
    run_randomized_source_scenario(context)


def _plan(context: RegressionRun, repo: Path) -> ImpactPlan:
    result = run_json(
        [context.sensez, "reflexez", str(repo), "--plan", "--json"],
        repo,
    )
    if not isinstance(result, dict):
        raise AssertionError("reflexez JSON plan must be an object")
    payload = cast(dict[str, object], result)
    selected = tuple(
        SelectedTest(
            file=str(item["file"]),
            runner=str(item["runner"]),
            reason=str(item["reason"]),
            distance=int(item["distance"]),
        )
        for item in cast(list[dict[str, object]], payload["selected"])
    )
    runners = cast(list[dict[str, object]], payload["runners"])
    return ImpactPlan(
        discovered_tests=int(cast(int, payload["discovered_tests"])),
        selected=selected,
        full_suite=bool(payload["full_suite"]),
        fallback_reasons=tuple(cast(list[str], payload["fallback_reasons"])),
        unresolved_dynamic_imports=int(cast(int, payload["unresolved_dynamic_imports"])),
        runner_kinds=tuple(sorted({str(runner["kind"]) for runner in runners})),
    )


def _write_fixture(repo: Path, fixture: Fixture) -> None:
    files = (
        (fixture.feature, fixture.feature_body),
        (fixture.related, fixture.related_body),
        (fixture.unrelated_source, fixture.unrelated_source_body),
        (fixture.unrelated_test, fixture.unrelated_test_body),
    )
    for name, body in files:
        path = repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body)


def _commit_fixture(repo: Path) -> None:
    run(["git", "add", "."], repo)
    run(
        [
            "git",
            "-c",
            "user.name=Sensez Regression",
            "-c",
            "user.email=sensez@example.test",
            "commit",
            "-m",
            "reflexez regression fixture",
        ],
        repo,
    )


def _assert_selective(plan: ImpactPlan, fixture: Fixture) -> None:
    assert plan.full_suite is False, plan.fallback_reasons
    selected = {Path(item.file).name: item for item in plan.selected}
    related = Path(fixture.related).name
    unrelated = Path(fixture.unrelated_test).name
    assert related in selected, "computed dynamic import did not select its test"
    assert selected[related].reason == "dynamic_import"
    assert unrelated not in selected, "unrelated test was selected"


def _assert_fallback(plan: ImpactPlan) -> None:
    assert plan.full_suite is True
    assert plan.selected, "global changes must select the full suite"
    assert any(
        "manifest" in reason or "fixture" in reason
        for reason in plan.fallback_reasons
    )


def _snapshot(plan: ImpactPlan, repo: Path) -> ImpactSnapshot:
    resolved_repo = repo.resolve()
    selected = [
        SelectedTest(
            file=str(Path(item.file).resolve().relative_to(resolved_repo)),
            runner=item.runner,
            reason=item.reason,
            distance=item.distance,
        )
        for item in plan.selected
        if "sensez_reflexez" in Path(item.file).name
        or "sensez-reflexez" in Path(item.file).name
    ]
    return ImpactSnapshot(
        discovered_tests=plan.discovered_tests,
        selected_count=len(plan.selected),
        selected_fixtures=tuple(selected),
        full_suite=plan.full_suite,
        fallback_reasons=plan.fallback_reasons,
        unresolved_dynamic_imports=plan.unresolved_dynamic_imports,
        runner_kinds=plan.runner_kinds,
    )


def _fixture(profile: str) -> Fixture:
    if profile == "py":
        return Fixture(
            feature="src/flask/sensez_reflexez_feature.py",
            feature_body="def value():\n    return 1\n",
            changed="def value():\n    return 2\n",
            related="tests/test_sensez_reflexez_dynamic.py",
            related_body=(
                'import importlib\nMODULE = "flask.sensez_reflexez_feature"\n'
                "feature = importlib.import_module(MODULE)\n"
                "def test_sensez_reflexez_dynamic():\n    assert feature.value() == 1\n"
            ),
            unrelated_source="src/flask/sensez_reflexez_unrelated.py",
            unrelated_source_body="def value():\n    return 7\n",
            unrelated_test="tests/test_sensez_reflexez_unrelated.py",
            unrelated_test_body=(
                "from flask.sensez_reflexez_unrelated import value\n"
                "def test_sensez_reflexez_unrelated():\n    assert value() == 7\n"
            ),
            global_file="pyproject.toml",
        )
    return Fixture(
        feature="packages/zod/src/sensez-reflexez-feature.ts",
        feature_body="export const value = 1;\n",
        changed="export const value = 2;\n",
        related="packages/zod/src/sensez-reflexez-dynamic.test.ts",
        related_body=(
            'const modulePath = "./sensez-reflexez-feature";\n'
            "test('dynamic', async () => {\n"
            "  const feature = await import(modulePath);\n"
            "  expect(feature.value).toBe(1);\n});\n"
        ),
        unrelated_source="packages/zod/src/sensez-reflexez-unrelated.ts",
        unrelated_source_body="export const unrelated = 7;\n",
        unrelated_test="packages/zod/src/sensez-reflexez-unrelated.test.ts",
        unrelated_test_body=(
            'import { unrelated } from "./sensez-reflexez-unrelated";\n'
            "test('unrelated', () => expect(unrelated).toBe(7));\n"
        ),
        global_file="package.json",
    )

