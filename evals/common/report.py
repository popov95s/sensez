#!/usr/bin/env python3
"""Render a detailed benchmark report from a Sensez A/B results tree."""

from __future__ import annotations

import json
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path

from models import (
    Finding,
    FindingCounts,
    JsonFields,
    PILLARS,
    ScanPayload,
)

MISSING_TEXT = ""


def _integer(value: object, default: int = 0) -> int:
    return value if isinstance(value, int) else default


def _text(value: object) -> str:
    return value if isinstance(value, str) else MISSING_TEXT


@dataclass(frozen=True)
class AgentUsage:
    input_tokens: int = 0
    output_tokens: int = 0
    reasoning_output_tokens: int = 0

    @classmethod
    def parse(cls, value: object) -> AgentUsage:
        data = JsonFields(value)
        return cls(
            _integer(data.get("input_tokens")),
            _integer(data.get("output_tokens")),
            _integer(data.get("reasoning_output_tokens")),
        )


@dataclass(frozen=True)
class ReportRun:
    variant: str
    agent_stdout: str
    quality_regression_score: int
    diff_total: int
    sensez_diff: FindingCounts
    smells: tuple[Finding, ...]

    @classmethod
    def parse(
        cls, metrics_value: object, agent_value: object, diff_value: object
    ) -> ReportRun:
        metrics = JsonFields(metrics_value)
        agent = JsonFields(agent_value)
        diff = JsonFields(diff_value)
        payload = ScanPayload.parse(diff.get("json"))
        counts = JsonFields(metrics.get("sensez_diff"))
        finding_counts = FindingCounts(
            *(_integer(counts.get(pillar)) for pillar in PILLARS)
        )
        return cls(
            variant=_text(metrics.get("variant")),
            agent_stdout=_text(agent.get("stdout")),
            quality_regression_score=_integer(
                metrics.get("quality_regression_score")
            ),
            diff_total=_integer(counts.get("total"), finding_counts.total),
            sensez_diff=finding_counts,
            smells=payload.smells,
        )


@dataclass(frozen=True)
class ReportSummary:
    by_variant: dict[str, list[ReportRun]]
    smell_kinds: dict[str, Counter]


def load_runs(root: Path) -> list[ReportRun]:
    runs = []
    for metrics_path in sorted(root.glob("*/*/run_*/metrics.json")):
        run_dir = metrics_path.parent
        runs.append(
            ReportRun.parse(
                json.loads(metrics_path.read_text()),
                json.loads((run_dir / "agent.json").read_text()),
                json.loads((run_dir / "sensez_diff.json").read_text()),
            )
        )
    return runs


def usage(agent_stdout: str) -> AgentUsage:
    for line in reversed(agent_stdout.splitlines()):
        if '"usage":' not in line:
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        return AgentUsage.parse(event.get("usage"))
    return AgentUsage()


def summarize(runs: list[ReportRun]) -> ReportSummary:
    by_variant = defaultdict(list)
    smell_kinds = defaultdict(Counter)
    for run in runs:
        by_variant[run.variant].append(run)
        for smell in run.smells:
            smell_kinds[run.variant][smell.kind or "unknown"] += 1
    return ReportSummary(dict(by_variant), dict(smell_kinds))


def pillar_totals(runs: list[ReportRun], variant: str) -> FindingCounts:
    selected = [run.sensez_diff for run in runs if run.variant == variant]
    return FindingCounts(
        cycles=sum(counts.cycles for counts in selected),
        dead_code=sum(counts.dead_code for counts in selected),
        boundaries=sum(counts.boundaries for counts in selected),
        duplication=sum(counts.duplication for counts in selected),
        smells=sum(counts.smells for counts in selected),
    )


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: report.py <results-dir>")
    runs = load_runs(Path(sys.argv[1]))
    summary = summarize(runs)

    print("# Sensez A/B Report\n")
    print("## Totals by Variant\n")
    print("| variant | runs | input_tokens | output_tokens | reasoning_tokens | quality_score | diff_findings |")
    print("| --- | --- | --- | --- | --- | --- | --- |")
    for variant, variant_runs in sorted(summary.by_variant.items()):
        usages = [usage(run.agent_stdout) for run in variant_runs]
        inputs = sum(item.input_tokens for item in usages)
        outputs = sum(item.output_tokens for item in usages)
        reasoning = sum(item.reasoning_output_tokens for item in usages)
        quality = sum(run.quality_regression_score for run in variant_runs)
        diffs = sum(run.diff_total for run in variant_runs)
        print(
            f"| {variant} | {len(variant_runs)} | {inputs} | {outputs} | "
            f"{reasoning} | {quality} | {diffs} |"
        )

    print("\n## Diff Findings by Pillar\n")
    print("| variant | cycles | dead_code | boundaries | duplication | smells |")
    print("| --- | --- | --- | --- | --- | --- |")
    for variant in sorted(summary.by_variant):
        totals = pillar_totals(runs, variant)
        print(
            f"| {variant} | {totals.cycles} | {totals.dead_code} | "
            f"{totals.boundaries} | {totals.duplication} | {totals.smells} |"
        )

    print("\n## Top Smell Kinds\n")
    print("| variant | smell_kind | count |")
    print("| --- | --- | --- |")
    for variant, counter in sorted(summary.smell_kinds.items()):
        for smell_kind, count in counter.most_common(10):
            print(f"| {variant} | {smell_kind} | {count} |")


if __name__ == "__main__":
    main()
