#!/usr/bin/env python3
"""Verify the Sensez docs examples.

For each ``docs/examples/<kind>/`` (a smell by snake_case kind or one of the
four pillars: ``cycles``/``dead_code``/``boundaries``/``duplication``) the
verifier copies the folder into a temp scan root alongside the unified
``docs/examples/sensez.toml`` and runs the local ``sensez`` binary in two
stages per language suffix:

* bad   — files not prefixed ``fixed``: named finding must fire for both
  ``.py`` and ``.ts`` (collateral findings are tolerated).
* fixed — only files whose top-level path component starts with ``fixed``:
  no finding from any pillar may emit on ``fixed.<ext>``.

Run via uv with no project install: ``uv run docs/verify_examples.py``.
``SENSEZ_BIN`` overrides the binary; rebuilt from source if missing. This
script is the docs CD gate (see ``.github/workflows/docs.yml``).
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from finding_types import SmellTerm
from rust_metadata import smell_kinds

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "docs/examples"
CONFIG = EXAMPLES / "sensez.toml"
DEFAULT_BIN = ROOT / "target/debug/sensez"
BIN = Path(os.environ.get("SENSEZ_BIN", DEFAULT_BIN))
SUFFIXES = (".py", ".ts")
JsonReport = dict[str, object]
FileList = list[str]
KindSet = set[str]
FailureList = list[str]
ExampleDirList = list[Path]

PILLAR_KINDS = {"cycles", "dead_code", "boundaries", "duplication"}
IGNORED_EXAMPLE_DIRS = {"node_modules", ".ruff_cache"}
PILLAR_BLOCK_KIND = dict.fromkeys(PILLAR_KINDS)


def ensure_binary() -> None:
    if BIN.exists():
        return
    print(f"building sensez binary at {BIN} ...", file=sys.stderr)
    subprocess.run(["cargo", "build"], cwd=ROOT, check=True)
    if not BIN.exists():
        raise SystemExit(f"expected binary at {BIN}, not found after build")


def run_noze(path: Path) -> JsonReport:
    cmd = [str(BIN), "noze", str(path), "--json", "--all"]
    proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=False)
    if proc.returncode != 0:
        raise RuntimeError(
            proc.stderr.strip() or proc.stdout.strip() or f"unknown error: {cmd}",
        )
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            f"could not parse JSON from {cmd}\n--- stdout ---\n{proc.stdout}",
        ) from exc


def stage_files(src_folder: Path, stage: str, dst: Path, suffix: str) -> None:
    """Copy one language's files under ``dst/<box>/`` per the stage rules.

    bad   : everything not prefixed ``fixed`` (and not a per-folder sensez.toml).
    fixed : only files whose first path component starts ``fixed``.

    Files are nested under ``dst/<box>/`` so sensez file paths keep the folder
    name as a path component (matching the path filters and preserving
    package-style imports like ``god_module.dep_a``). One language per scan so
    two files with colliding module names (``example``) do not graph-collapse.
    """
    box = src_folder.name
    base = dst / box
    for path in src_folder.rglob("*"):
        if path.is_dir() or path.suffix != suffix:
            continue
        rel = path.relative_to(src_folder)
        first = rel.parts[0]
        if first == "sensez.toml":
            continue
        if stage == "bad" and first.startswith("fixed"):
            continue
        if stage == "fixed" and not first.startswith("fixed"):
            continue
        target = base / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)


def build_scan_root(folder: Path, stage: str, suffix: str) -> Path:
    tmp = Path(tempfile.mkdtemp(prefix=f"sensez-{stage}-{folder.name}-{suffix[1:]}-"))
    shutil.copy2(CONFIG, tmp / "sensez.toml")
    stage_files(folder, stage, tmp, suffix)
    return tmp


def _norm(path: str) -> str:
    return path.replace("\\", "/")


def _smell_files(report: JsonReport) -> FileList:
    return [_norm(f.get("file", "")) for f in report.get("smells", [])]


def _block_files(report: JsonReport, block: str) -> FileList:
    """Flatten a pillar block to file paths (cycles nests edges[*].file,
    duplication nests occurrences[*].file, others expose a top-level file)."""
    files: FileList = []
    for finding in report.get(block, []):
        if block == "cycles":
            files.extend(_norm(e.get("file", "")) for e in finding.get("edges", []))
        elif block == "duplication":
            files.extend(
                _norm(o.get("file", "")) for o in finding.get("occurrences", [])
            )
        else:
            files.append(_norm(finding.get("file", "")))
    return files


def _endswith(files: FileList, folder: str, name: str) -> FileList:
    wanted = f"/{folder}/{name}"
    return [p for p in files if p.endswith(wanted)]


def _in_box(files: FileList, folder: str, suffix: str) -> FileList:
    return [p for p in files if f"/{folder}/" in p and p.endswith(suffix)]


def _smell_kinds(report: JsonReport, files: FileList) -> KindSet:
    return {
        f["kind"] for f in report.get("smells", []) if _norm(f.get("file", "")) in files
    }


def _fixed_pillar_leaks(
    report: JsonReport,
    folder: Path,
    suffix: str,
    failures: FailureList,
) -> None:
    box = folder.name
    for block in ("cycles", "dead_code", "boundaries", "duplication"):
        leaks = _endswith(_block_files(report, block), box, f"fixed{suffix}")
        if leaks:
            failures.append(
                f"{box}/fixed{suffix}: must emit no {block} pillar, saw {len(leaks)} finding(s)",
            )


def verify_smell_folder(folder: Path, smell_kind: str, failures: FailureList) -> None:
    box = folder.name
    for suffix in SUFFIXES:
        bad = build_scan_root(folder, "bad", suffix)
        bad_report = run_noze(bad)
        bad_files = _endswith(_smell_files(bad_report), box, f"example{suffix}")
        bad_kinds = _smell_kinds(bad_report, bad_files)
        if smell_kind not in bad_kinds:
            failures.append(
                f"{box}/example{suffix}: expected {smell_kind}, "
                f"saw {sorted(bad_kinds) or 'no smells'}",
            )
        shutil.rmtree(bad, ignore_errors=True)

        fixed = build_scan_root(folder, "fixed", suffix)
        fixed_report = run_noze(fixed)
        fixed_files = _endswith(_smell_files(fixed_report), box, f"fixed{suffix}")
        if fixed_files:
            failures.append(
                f"{box}/fixed{suffix}: must emit no smell, saw {sorted(_smell_kinds(fixed_report, fixed_files))}",
            )
        _fixed_pillar_leaks(fixed_report, folder, suffix, failures)
        shutil.rmtree(fixed, ignore_errors=True)


def verify_pillar_folder(folder: Path, pillar: str, failures: FailureList) -> None:
    box = folder.name
    for suffix in SUFFIXES:
        bad = build_scan_root(folder, "bad", suffix)
        bad_report = run_noze(bad)
        if not _in_box(_block_files(bad_report, pillar), box, suffix):
            failures.append(
                f"{box}: expected pillar {pillar} on a {suffix} example file, saw nothing",
            )
        shutil.rmtree(bad, ignore_errors=True)

        fixed = build_scan_root(folder, "fixed", suffix)
        fixed_report = run_noze(fixed)
        fixed_files = _endswith(_smell_files(fixed_report), box, f"fixed{suffix}")
        if fixed_files:
            failures.append(
                f"{box}/fixed{suffix}: must emit no smell, saw {sorted(_smell_kinds(fixed_report, fixed_files))}",
            )
        _fixed_pillar_leaks(fixed_report, folder, suffix, failures)
        shutil.rmtree(fixed, ignore_errors=True)


def iter_example_dirs() -> ExampleDirList:
    out: ExampleDirList = []
    for child in sorted(EXAMPLES.iterdir()):
        if child.is_dir() and child.name not in IGNORED_EXAMPLE_DIRS | {"smells"}:
            out.append(child)
    smells_root = EXAMPLES / "smells"
    if smells_root.is_dir():
        out.extend(sorted(c for c in smells_root.iterdir() if c.is_dir()))
    return out


def main() -> int:
    ensure_binary()
    failures: FailureList = []
    smells = smell_kinds()
    for folder in iter_example_dirs():
        if folder.name in PILLAR_KINDS:
            verify_pillar_folder(folder, folder.name, failures)
        else:
            try:
                smell = SmellTerm(folder.name)
            except ValueError:
                print(f"warning: skipping unrecognized example folder: {folder.name}")
                continue
            if smell not in smells:
                print(f"warning: skipping unrecognized example folder: {folder.name}")
                continue
            verify_smell_folder(folder, folder.name, failures)
    if failures:
        print("Docs example verification failed:")
        for failure in failures:
            print(f"  - {failure}")
        return 1
    print(
        "All docs examples verify: named findings fire on the bad example, "
        "and fixed.* emits no finding.",
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
