#!/usr/bin/env python3
"""Relative L2 performance guard: one-file edit versus a forced-cold scan."""

from __future__ import annotations

import argparse
import os
import re
import shutil
import statistics
import subprocess
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path


REUSE = re.compile(r"parse-cache reused=(\d+)/(\d+) bytes=(\d+)/(\d+)")
EXTENSIONS = {".py", ".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx", ".rs"}


@dataclass(frozen=True)
class BenchmarkResult:
    cold_median_s: float
    l2_median_s: float
    ratio: float
    reused_bytes: int
    total_bytes: int


def run(binary: Path, root: Path, cache: bool) -> tuple[float, str]:
    env = {
        **os.environ,
        "SENSEZ_ANALYSIS_CACHE": "true" if cache else "false",
        "SENSEZ_TIMING": "true",
    }
    started = time.perf_counter()
    process = subprocess.run(
        [binary, "noze", root, "--all", "--json"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    elapsed = time.perf_counter() - started
    if process.returncode:
        raise RuntimeError(process.stderr)
    return elapsed, process.stderr


def source_to_edit(root: Path) -> Path:
    candidates = [
        path
        for path in root.rglob("*")
        if path.is_file()
        and path.suffix in EXTENSIONS
        and ".git" not in path.parts
        and ".sensez" not in path.parts
    ]
    if not candidates:
        raise RuntimeError(f"no supported source files in {root}")
    # A small edit is the common incremental case and gives stable parse work.
    return min(candidates, key=lambda path: (path.stat().st_size, str(path)))


def toggle_marker(path: Path, present: bool) -> None:
    marker = b"\n// sensez cache benchmark\n" if path.suffix != ".py" else b"\n# sensez cache benchmark\n"
    content = path.read_bytes()
    if present:
        path.write_bytes(content.removesuffix(marker) + marker)
    else:
        path.write_bytes(content.removesuffix(marker))


def benchmark(binary: Path, source: Path, runs: int) -> BenchmarkResult:
    with tempfile.TemporaryDirectory(prefix="sensez-l2-bench-") as temporary:
        root = Path(temporary) / source.name
        shutil.copytree(source, root, ignore=shutil.ignore_patterns(".sensez"))
        run(binary, root, cache=True)
        target = source_to_edit(root)
        cold: list[float] = []
        incremental: list[float] = []
        reused_bytes = total_bytes = 0
        for index in range(runs):
            toggle_marker(target, index % 2 == 0)
            cold.append(run(binary, root, cache=False)[0])
            elapsed, stderr = run(binary, root, cache=True)
            match = REUSE.search(stderr)
            if match is None:
                raise RuntimeError("L2 run did not report parse-cache reuse")
            reused, total, reused_bytes, total_bytes = map(int, match.groups())
            if reused == 0 or total == 0:
                raise RuntimeError("L2 run reused no parsed files")
            incremental.append(elapsed)
        cold_median = statistics.median(cold)
        l2_median = statistics.median(incremental)
        return BenchmarkResult(
            cold_median_s=cold_median,
            l2_median_s=l2_median,
            ratio=l2_median / cold_median,
            reused_bytes=reused_bytes,
            total_bytes=total_bytes,
        )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sensez", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--threshold", type=float, default=1.25)
    parser.add_argument("targets", nargs="+", help="name=path")
    args = parser.parse_args()
    failed = False
    for value in args.targets:
        name, raw_path = value.split("=", 1)
        result = benchmark(args.sensez.resolve(), Path(raw_path), args.runs)
        status = "REGRESSION" if result.ratio > args.threshold else "ok"
        print(
            f"  {name}: L2 {result.l2_median_s:.3f}s / "
            f"cold {result.cold_median_s:.3f}s = {result.ratio:.2f}x; "
            f"reused {result.reused_bytes}/{result.total_bytes} bytes [{status}]"
        )
        failed |= result.ratio > args.threshold
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
