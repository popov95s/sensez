from __future__ import annotations

import shutil
import tempfile
from pathlib import Path

from .analyze import accept_tree, compare_tree
from .normalize import dump_json
from .setup_capture import (
    artifact,
    git_init,
    replacements,
    run,
    run_pty,
    seed_global_config,
)


AGENTS = ("claude-code", "cursor", "cline", "codex", "opencode", "pi", "other")


def run_setup_regressions(
    sensez: Path, results: Path, baselines: Path, accept: bool
) -> None:
    if results.exists():
        shutil.rmtree(results)
    results.mkdir(parents=True)
    for agent in AGENTS:
        dump_json(
            results / f"init.project.{agent}.json",
            _project_scenario(sensez, agent),
        )
        dump_json(
            results / f"init.global.{agent}.json",
            _global_scenario(sensez, agent),
        )
    for name, result in _edge_scenarios(sensez):
        dump_json(results / f"{name}.json", result)
    if accept:
        accept_tree(results, baselines)
        print("accepted baselines for setup")
        return
    failures = compare_tree(results, baselines)
    if failures:
        raise RuntimeError("\n\n".join(failures))


def _project_scenario(sensez: Path, agent: str) -> object:
    with tempfile.TemporaryDirectory(prefix="sensez-init-project-") as tmp:
        root = Path(tmp)
        repo = root / "repo"
        home = root / "home"
        repo.mkdir()
        home.mkdir()
        git_init(repo)
        command = [sensez, "init", repo, "--agent", agent, "--yes"]
        paths = replacements(sensez, repo=repo, home=home, cwd=root)
        runs = [run(command, root, home, paths) for _ in range(2)]
        return artifact("project", runs, repo, home, paths)


def _global_scenario(sensez: Path, agent: str) -> object:
    with tempfile.TemporaryDirectory(prefix="sensez-init-global-") as tmp:
        root = Path(tmp)
        cwd = root / "outside-git"
        home = root / "home"
        cwd.mkdir()
        home.mkdir()
        seed_global_config(home, agent)
        command = [sensez, "init", cwd, "--agent", agent, "--yes"]
        paths = replacements(sensez, repo=cwd, home=home, cwd=root)
        runs = [run(command, root, home, paths) for _ in range(2)]
        return artifact("global", runs, cwd, home, paths)


def _edge_scenarios(sensez: Path) -> list[tuple[str, object]]:
    scenarios = [
        ("init.global.interactive-accept", _interactive_global(sensez, "y\n")),
        ("init.global.interactive-decline", _interactive_global(sensez, "n\n")),
        ("init.global.no-confirmation", _no_confirmation(sensez)),
        ("init.global.missing-home", _missing_home(sensez)),
        ("init.nested-repository", _nested_repository(sensez)),
        ("init.git-file-repository", _git_file_repository(sensez)),
        ("init.global.claude-gate", _global_claude_gate(sensez)),
        ("init.unsupported-gate", _unsupported_gate(sensez)),
        ("init.unknown-agent", _unknown_agent(sensez)),
    ]
    return scenarios


def _interactive_global(sensez: Path, answer: str) -> object:
    with tempfile.TemporaryDirectory(prefix="sensez-init-interactive-") as tmp:
        root = Path(tmp)
        cwd = root / "outside-git"
        home = root / "home"
        cwd.mkdir()
        home.mkdir()
        command = [sensez, "init", cwd, "--agent", "codex"]
        paths = replacements(sensez, repo=cwd, home=home, cwd=root)
        result = run_pty(command, root, home, answer, paths)
        return artifact("interactive-global", [result], cwd, home, paths)


def _no_confirmation(sensez: Path) -> object:
    return _single_outside(sensez, ["--agent", "codex"], "no-confirmation")


def _missing_home(sensez: Path) -> object:
    return _single_outside(
        sensez,
        ["--agent", "codex", "--yes"],
        "missing-home",
        remove_home=True,
    )


def _global_claude_gate(sensez: Path) -> object:
    return _single_outside(
        sensez,
        ["--agent", "claude-code", "--gate", "--yes"],
        "global-claude-gate",
    )


def _single_outside(
    sensez: Path,
    args: list[str],
    scenario: str,
    remove_home: bool = False,
) -> object:
    with tempfile.TemporaryDirectory(prefix="sensez-init-outside-") as tmp:
        root = Path(tmp)
        cwd = root / "outside-git"
        home = root / "home"
        cwd.mkdir()
        home.mkdir()
        command: list[str | Path] = [sensez, "init", cwd, *args]
        paths = replacements(sensez, repo=cwd, home=home, cwd=root)
        result = run(command, root, home, paths, remove_home=remove_home)
        return artifact(scenario, [result], cwd, home, paths)


def _nested_repository(sensez: Path) -> object:
    with tempfile.TemporaryDirectory(prefix="sensez-init-nested-") as tmp:
        root = Path(tmp)
        repo = root / "repo"
        nested = repo / "nested"
        home = root / "home"
        nested.mkdir(parents=True)
        home.mkdir()
        git_init(repo)
        command = [sensez, "init", nested, "--agent", "codex", "--yes"]
        paths = replacements(sensez, repo=repo, home=home, cwd=root)
        result = run(command, root, home, paths)
        return artifact("nested-repository", [result], repo, home, paths)


def _git_file_repository(sensez: Path) -> object:
    with tempfile.TemporaryDirectory(prefix="sensez-init-git-file-") as tmp:
        root = Path(tmp)
        repo = root / "repo"
        home = root / "home"
        repo.mkdir()
        home.mkdir()
        (repo / ".git").write_text("gitdir: <fixture>\n")
        command = [sensez, "init", repo, "--agent", "codex", "--yes"]
        paths = replacements(sensez, repo=repo, home=home, cwd=root)
        result = run(command, root, home, paths)
        return artifact("git-file-repository", [result], repo, home, paths)


def _unsupported_gate(sensez: Path) -> object:
    return _single_project(sensez, ["--agent", "codex", "--gate", "--yes"])


def _unknown_agent(sensez: Path) -> object:
    return _single_project(sensez, ["--agent", "unknown", "--yes"])


def _single_project(sensez: Path, args: list[str]) -> object:
    with tempfile.TemporaryDirectory(prefix="sensez-init-edge-project-") as tmp:
        root = Path(tmp)
        repo = root / "repo"
        home = root / "home"
        repo.mkdir()
        home.mkdir()
        git_init(repo)
        command: list[str | Path] = [sensez, "init", repo, *args]
        paths = replacements(sensez, repo=repo, home=home, cwd=root)
        result = run(command, root, home, paths)
        return artifact("project-edge", [result], repo, home, paths)
