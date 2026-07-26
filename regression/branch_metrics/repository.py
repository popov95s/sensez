from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path

from ..harness.fixtures import write_fixture_consumer
from ..harness.models import DeadCodeFixture


BRANCH_METRICS_CONFIG = """\
[dead_code]
unused_imports = true
unused_methods = true
unused_variables = true
"""


def branch_metric_repo(target_name: str, fixture: DeadCodeFixture) -> Path:
    temporary = Path(tempfile.mkdtemp(prefix=f"sensez-{target_name}-branch-metrics-"))
    repo = temporary / "repo"
    repo.mkdir()
    git(repo, "init")
    (repo / "sensez.toml").write_text(BRANCH_METRICS_CONFIG)
    write_base_source(repo, fixture)
    commit_all(repo, "base")
    return repo


def apply_branch_fixture(repo: Path, fixture: DeadCodeFixture, text: str) -> None:
    path = repo / fixture["path"]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    write_fixture_consumer(repo, fixture)


def write_base_source(repo: Path, fixture: DeadCodeFixture) -> None:
    path = Path(fixture["path"])
    if path.suffix == ".ts":
        (repo / "base.ts").write_text(
            "const sensezBranchBase = 1;\nconsole.log(sensezBranchBase);\n"
        )
    else:
        (repo / "base.py").write_text("print('base')\n")


def commit_all(repo: Path, message: str) -> None:
    git(repo, "add", ".")
    git(
        repo,
        "-c",
        "user.email=sensez@example.test",
        "-c",
        "user.name=Sensez",
        "commit",
        "-m",
        message,
    )


def git(repo: Path, *args: str) -> None:
    proc = subprocess.run(
        ["git", *args],
        cwd=repo,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if proc.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed: {proc.stderr}")
