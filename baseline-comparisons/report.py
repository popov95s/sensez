#!/usr/bin/env python3
"""Build the benchmark dashboard from results/runs.jsonl + saved tool outputs.

A thin entrypoint: load runs → build comparison rows → render. All logic lives
in the `bench` package (one layer per responsibility). Run after bench.sh:

    uv run python report.py
"""

import os
import sys

from bench.compare import build_rows
from bench.loading import load_runs
from bench.render import render_html, render_terminal

RESULTS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "results")


def main():
    latest, order = load_runs(os.path.join(RESULTS, "runs.jsonl"))
    if not latest:
        sys.exit("no results/runs.jsonl — run ./bench.sh <path> first")
    rows = build_rows(latest, order)
    if not rows:
        sys.exit("no sensez runs found in results/runs.jsonl")
    render_terminal(rows)
    out = os.path.join(RESULTS, "report.html")
    with open(out, "w", encoding="utf-8") as fh:
        fh.write(render_html(rows))
    print(f"\nwrote {out}")


if __name__ == "__main__":
    main()
