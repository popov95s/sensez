"""Deterministic randomized source-impact execution regression."""

from __future__ import annotations

import random
import shlex
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from ..harness.commands import run, run_json
from ..harness.models import RegressionRun


SOURCE_COUNT = 12
CHANGED_COUNT = 4


@dataclass(frozen=True)
class RandomizedResult:
    seed: int
    source_files: int
    changed_source_files: int
    expected_tests: int
    selected_tests: int
    executed_tests: int
    exact_selection: bool
    exact_execution: bool
    maximum_distance: int


def run_randomized_source_scenario(context: RegressionRun) -> None:
    profile = context.target["profile"]
    seed = 1847 if profile == "py" else 2903
    with tempfile.TemporaryDirectory(prefix=f"reflexez-random-{profile}-") as value:
        root = Path(value)
        marker = root / "runner.capture"
        _write_project(root, marker, profile)
        _commit(root)
        changed = _mutate_random_sources(root, profile, seed)
        expected = {_test_path(root, profile, index) for index in changed}
        command = _command(context.sensez, root, profile, changed)
        plan = cast(dict[str, object], run_json([*command, "--plan", "--json"], root))
        selected_items = cast(list[dict[str, object]], plan["selected"])
        selected = {Path(str(item["file"])).resolve() for item in selected_items}
        distances = [int(item["distance"]) for item in selected_items]

        run(command, root)
        executed = _captured_tests(marker, profile)
        result = RandomizedResult(
            seed=seed,
            source_files=SOURCE_COUNT,
            changed_source_files=len(changed),
            expected_tests=len(expected),
            selected_tests=len(selected),
            executed_tests=len(executed),
            exact_selection=selected == expected,
            exact_execution=executed == expected,
            maximum_distance=max(distances, default=0),
        )
        assert result.exact_selection, (selected, expected)
        assert result.exact_execution, (executed, expected)
        assert result.maximum_distance >= 2


def _write_project(root: Path, marker: Path, profile: str) -> None:
    if profile == "py":
        _write(root, "pyproject.toml", '[tool.pytest.ini_options]\ntestpaths=["tests"]\n')
        _fake_runner(root / ".venv/bin/pytest", marker)
    else:
        _write(root, "package.json", '{"devDependencies":{"vitest":"4"}}\n')
        _fake_runner(root / "node_modules/.bin/vitest", marker)
    for index in range(SOURCE_COUNT):
        if profile == "py":
            _write(root, f"leaf_{index}.py", f"value = {index}\n")
            _write(
                root,
                f"middle_{index}.py",
                f"from leaf_{index} import value\nresult = value\n",
            )
            _write(
                root,
                f"tests/test_{index}.py",
                f"from middle_{index} import result\ndef test_value(): assert result >= 0\n",
            )
        else:
            _write(root, f"src/leaf_{index}.ts", f"export const value = {index};\n")
            _write(
                root,
                f"src/middle_{index}.ts",
                f"import {{ value }} from './leaf_{index}';\nexport const result = value;\n",
            )
            _write(
                root,
                f"tests/value_{index}.test.ts",
                f"import {{ result }} from '../src/middle_{index}';\ntest('value', () => result);\n",
            )


def _fake_runner(path: Path, marker: Path) -> None:
    script = (
        "#!/bin/sh\n"
        f": > {shlex.quote(str(marker))}\n"
        f"for arg in \"$@\"; do printf '%s\\n' \"$arg\" >> {shlex.quote(str(marker))}; done\n"
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(script)
    path.chmod(0o755)


def _mutate_random_sources(root: Path, profile: str, seed: int) -> list[int]:
    chosen = sorted(random.Random(seed).sample(range(SOURCE_COUNT), CHANGED_COUNT))
    for index in chosen:
        path = _source_path(root, profile, index)
        marker = "# changed\n" if profile == "py" else "// changed\n"
        path.write_text(path.read_text() + marker)
    return chosen


def _command(sensez: Path, root: Path, profile: str, changed: list[int]) -> list[Path | str]:
    command: list[Path | str] = [sensez, "reflexez", root]
    for index in changed:
        command.extend(["--changed-file", _source_path(root, profile, index)])
    return command


def _source_path(root: Path, profile: str, index: int) -> Path:
    name = f"leaf_{index}.py" if profile == "py" else f"src/leaf_{index}.ts"
    return (root / name).resolve()


def _test_path(root: Path, profile: str, index: int) -> Path:
    name = f"tests/test_{index}.py" if profile == "py" else f"tests/value_{index}.test.ts"
    return (root / name).resolve()


def _captured_tests(marker: Path, profile: str) -> set[Path]:
    suffix = ".py" if profile == "py" else ".ts"
    return {
        Path(line).resolve()
        for line in marker.read_text().splitlines()
        if line.endswith(suffix)
    }


def _write(root: Path, relative: str, source: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(source)


def _commit(root: Path) -> None:
    run(["git", "init", "-q"], root)
    run(["git", "config", "user.email", "regression@example.test"], root)
    run(["git", "config", "user.name", "Sensez Regression"], root)
    run(["git", "add", "."], root)
    run(["git", "commit", "-qm", "randomized source graph"], root)
