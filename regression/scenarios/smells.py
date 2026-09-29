from pathlib import Path
from typing import cast

from ..harness.commands import run_json
from ..harness.models import RegressionRun
from ..harness.paths import ROOT
from ..harness.repositories import cleanup_repo, scenario_repo


def run_smell_regressions(context: RegressionRun) -> None:
    repo = scenario_repo(context.cache, context.target)
    try:
        config = repo / "sensez.toml"
        config.write_text(
            config.read_text()
            + "\n[smells.rules.weak_test_oracle]\nenabled = true\n"
            + "\n[smells.rules.redundant_wrapper_chain]\nenabled = true\n"
        )
        fixtures = _fixtures(context.target["profile"])
        for relative, source in fixtures.items():
            path = repo / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(source)

        report = run_json(
            [context.sensez, "noze", repo, "--all", "--json"],
            ROOT,
        )
        if not isinstance(report, dict):
            raise AssertionError("noze JSON report must be an object")
        findings = cast(list[dict[str, object]], report["smells"])
        _assert_smell(findings, fixtures, "bad_test", "weak_test_oracle", True)
        _assert_smell(findings, fixtures, "fixed_test", "weak_test_oracle", False)
        _assert_smell(findings, fixtures, "bad_wrappers", "redundant_wrapper_chain", True)
        _assert_smell(findings, fixtures, "fixed_wrappers", "redundant_wrapper_chain", False)
    finally:
        cleanup_repo(repo)


def _assert_smell(
    findings: list[dict[str, object]],
    fixtures: dict[str, str],
    fixture_name: str,
    kind: str,
    expected: bool,
) -> None:
    relative = next(path for path in fixtures if path.startswith(fixture_name + "/"))
    matches = [
        finding
        for finding in findings
        if finding.get("kind") == kind
        and str(finding.get("file", "")).endswith(relative)
    ]
    if bool(matches) != expected:
        raise AssertionError(
            f"{relative}: expected {kind}={expected}, found {len(matches)}"
        )


def _fixtures(profile: str) -> dict[str, str]:
    if profile == "py":
        return {
            "bad_test/tests/test_sensez_regression_oracle.py": (
                "def test_sensez_regression_oracle():\n"
                "    value = 1 + 1\n"
                "    print(value)\n"
            ),
            "fixed_test/tests/test_sensez_regression_oracle_fixed.py": (
                "def test_sensez_regression_oracle_fixed():\n"
                "    value = 1 + 1\n"
                "    assert value == 2\n"
            ),
            "bad_wrappers/src/sensez_regression_wrappers.py": (
                "def sensez_regression_outer(value):\n"
                "    return sensez_regression_middle(value)\n\n"
                "def sensez_regression_middle(value):\n"
                "    return sensez_regression_terminal(value)\n\n"
                "def sensez_regression_terminal(value):\n"
                "    return value + 1\n"
            ),
            "fixed_wrappers/src/sensez_regression_wrappers_fixed.py": (
                "def sensez_regression_outer(value):\n"
                "    return value + 1\n"
            ),
        }
    if profile == "ts":
        return {
            "bad_test/tests/sensez-regression-oracle.test.ts": (
                'import test from "node:test";\n\n'
                'test("runs without checking behavior", () => {\n'
                "  const value = 1 + 1;\n"
                "  console.log(value);\n"
                "});\n"
            ),
            "fixed_test/tests/sensez-regression-oracle-fixed.test.ts": (
                'import assert from "node:assert/strict";\n'
                'import test from "node:test";\n\n'
                'test("checks behavior", () => {\n'
                "  const value = 1 + 1;\n"
                "  assert.equal(value, 2);\n"
                "});\n"
            ),
            "bad_wrappers/src/sensez-regression-wrappers.ts": (
                "function sensezRegressionOuter(value: number): number {\n"
                "  return sensezRegressionMiddle(value);\n"
                "}\n\n"
                "function sensezRegressionMiddle(value: number): number {\n"
                "  return sensezRegressionTerminal(value);\n"
                "}\n\n"
                "function sensezRegressionTerminal(value: number): number {\n"
                "  return value + 1;\n"
                "}\n"
            ),
            "fixed_wrappers/src/sensez-regression-wrappers-fixed.ts": (
                "function sensezRegressionOuter(value: number): number {\n"
                "  return value + 1;\n"
                "}\n"
            ),
        }
    raise AssertionError(f"unsupported smell regression profile: {profile}")
