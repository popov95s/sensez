from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path

from .commands import run
from .models import Target
from .paths import ROOT


REGRESSION_SENSEZ_TOML = """\
[dead_code]
entry_points = [
  "**/packages/zod/**",
  "**/src/flask/app.py",
]
unused_imports = true
unused_methods = true
unused_variables = true

[smells.rules.nested_loop]
enabled = true

[smells.rules.n_plus_one_call]
enabled = true
"""


def ensure_cache(root_text: str, target: Target) -> Path:
    root = Path(root_text)
    root.mkdir(parents=True, exist_ok=True)
    destination = root / target["name"]
    if not (destination / ".git").exists():
        seed = Path("/tmp/bench-targets") / target["name"]
        source = str(seed) if (seed / ".git").exists() else target["url"]
        clone_args = ["git", "clone"]
        if source == str(seed):
            clone_args.append("--local")
        run([*clone_args, source, str(destination)], ROOT)
    run(
        ["git", "fetch", "--depth", "1", "origin", target["commit"]],
        destination,
        check=False,
    )
    run(["git", "checkout", "--force", target["commit"]], destination)
    run(["git", "clean", "-ffd"], destination)
    if target.get("setup") and not (destination / "node_modules").exists():
        run(target["setup"], destination)
    return destination


def scenario_repo(cache: Path, target: Target) -> Path:
    temporary = Path(tempfile.mkdtemp(prefix=f"sensez-{target['name']}-"))
    destination = temporary / target["name"]
    run(["git", "clone", "--local", str(cache), str(destination)], ROOT)
    head = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=cache, text=True
    ).strip()
    run(["git", "checkout", "--force", head], destination)
    run(["git", "checkout", "-B", "sensez-regression-worktree"], destination)
    _install_cached_setup(cache, destination, target)
    (destination / "sensez.toml").write_text(REGRESSION_SENSEZ_TOML)
    return destination


def cleanup_repo(repo: Path) -> None:
    shutil.rmtree(repo.parent, ignore_errors=True)


def _install_cached_setup(cache: Path, destination: Path, target: Target) -> None:
    if not target.get("setup"):
        return
    cached_modules = cache / "node_modules"
    if cached_modules.exists():
        shutil.copytree(
            cached_modules,
            destination / "node_modules",
            symlinks=True,
        )
        return
    run(target["setup"], destination)
