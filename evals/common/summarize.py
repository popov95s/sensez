#!/usr/bin/env python3
"""Summarize Sensez A/B evaluation result directories."""

from __future__ import annotations

import json
import sys
from collections import defaultdict
from pathlib import Path
from statistics import mean

from models import ScanPayload
from quality_score import score_payload
from summary_models import GroupSummary, MetricRow


def load_metrics(root: Path) -> list[MetricRow]:
    rows = []
    for path in sorted(root.glob("*/*/run_*/metrics.json")):
        row = MetricRow.parse(json.loads(path.read_text()))
        diff_path = path.parent / "sensez_diff.json"
        if diff_path.exists():
            envelope = json.loads(diff_path.read_text())
            payload = envelope.get("json", envelope)
            row = row.with_quality(score_payload(ScanPayload.parse(payload)))
        rows.append(row)
    return rows


def average(values: list[float]) -> float:
    return round(mean(values), 3) if values else 0.0


def summarize_group(rows: list[MetricRow]) -> GroupSummary:
    tested = [row for row in rows if row.test_returncode is not None]
    return GroupSummary(
        runs=len(rows),
        agent_success_rate=average(
            [float(row.agent_returncode == 0) for row in rows]
        ),
        test_success_rate=average(
            [float(row.test_returncode == 0) for row in tested]
        ),
        avg_elapsed_seconds=average(
            [row.agent_elapsed_seconds for row in rows]
        ),
        avg_diff_total=average(
            [float(row.counts("sensez_diff").total) for row in rows]
        ),
        avg_qual_score=average(
            [float(row.integer("quality_regression_score")) for row in rows]
        ),
        avg_new_qual_score=average(
            [float(row.integer("new_quality_score")) for row in rows]
        ),
        avg_existing_qual_score=average(
            [float(row.integer("existing_quality_score")) for row in rows]
        ),
        avg_inherent_qual_score=average(
            [float(row.integer("inherent_quality_score")) for row in rows]
        ),
        avg_delta_total=average(
            [float(row.integer("sensez_delta_total")) for row in rows]
        ),
        avg_after_findings=average(
            [float(row.counts("sensez_after").total) for row in rows]
        ),
        avg_tool_calls=average(
            [float(row.integer("sensez_tool_calls")) for row in rows]
        ),
        avg_input_tokens=average(
            [float(row.integer("input_tokens")) for row in rows]
        ),
        avg_output_tokens=average(
            [float(row.integer("output_tokens")) for row in rows]
        ),
        avg_total_tokens=average(
            [
                float(
                    row.integer("input_tokens")
                    + row.integer("output_tokens")
                )
                for row in rows
            ]
        ),
        avg_tokens_per_line=average(
            [
                (
                    row.integer("input_tokens")
                    + row.integer("output_tokens")
                )
                / max(1, row.diff_stats.lines_added)
                for row in rows
            ]
        ),
        avg_files_touched=average(
            [float(row.diff_stats.files_touched) for row in rows]
        ),
        avg_lines_added=average(
            [float(row.diff_stats.lines_added) for row in rows]
        ),
        avg_lines_deleted=average(
            [float(row.diff_stats.lines_deleted) for row in rows]
        ),
        avg_clone_tokens=average(
            [float(row.severity.clone_total_tokens) for row in rows]
        ),
        avg_clone_new_tokens=average(
            [float(row.severity.clone_new_tokens) for row in rows]
        ),
        avg_complexity_max=average(
            [float(row.severity.complexity_max) for row in rows]
        ),
    )


def print_table(grouped: dict[str, list[MetricRow]]) -> None:
    columns = [
        ("runs", "runs"),
        ("agent_ok%", "agent_success_rate"),
        ("sec", "avg_elapsed_seconds"),
        ("qual_new", "avg_new_qual_score"),
        ("qual_ex", "avg_existing_qual_score"),
        ("qual_in", "avg_inherent_qual_score"),
        ("tok_in", "avg_input_tokens"),
        ("tok_out", "avg_output_tokens"),
        ("tok_tot", "avg_total_tokens"),
        ("tok/line", "avg_tokens_per_line"),
        ("tool_calls", "avg_tool_calls"),
        ("+lines", "avg_lines_added"),
        ("clone_tok", "avg_clone_tokens"),
        ("clone_new", "avg_clone_new_tokens"),
        ("complx", "avg_complexity_max"),
    ]
    headers = ["variant", *(label for label, _ in columns)]
    print("| " + " | ".join(headers) + " |")
    print("| " + " | ".join(["---"] * len(headers)) + " |")
    for variant, rows in sorted(grouped.items()):
        summary = summarize_group(rows)
        values = [variant, *(str(getattr(summary, field)) for _, field in columns)]
        print("| " + " | ".join(values) + " |")


def print_pairs(rows: list[MetricRow]) -> None:
    by_pair = defaultdict(dict)
    for row in rows:
        by_pair[(row.task_id, row.run)][row.variant] = row
    print("\n## Paired Deltas\n")
    print("| task | run | qual_new | clone_tok | clone_new | complx | tok_delta | +lines | tool_calls |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    for (task_id, run), pair in sorted(by_pair.items()):
        if "control" not in pair or "sensez" not in pair:
            continue
        control, sensez = pair["control"], pair["sensez"]
        deltas = (
            sensez.integer("new_quality_score")
            - control.integer("new_quality_score"),
            sensez.severity.clone_total_tokens
            - control.severity.clone_total_tokens,
            sensez.severity.clone_new_tokens
            - control.severity.clone_new_tokens,
            sensez.severity.complexity_max
            - control.severity.complexity_max,
            sensez.integer("input_tokens")
            + sensez.integer("output_tokens")
            - control.integer("input_tokens")
            - control.integer("output_tokens"),
            sensez.diff_stats.lines_added - control.diff_stats.lines_added,
            sensez.integer("sensez_tool_calls")
            - control.integer("sensez_tool_calls"),
        )
        rendered = " | ".join(f"{value:+d}" for value in deltas)
        print(f"| {task_id} | {run} | {rendered} |")


def print_per_task_detail(rows: list[MetricRow]) -> None:
    print("\n## Per-Task Detail\n")
    for row in sorted(rows, key=lambda item: (item.task_id, item.variant, item.run)):
        pillars = []
        for name in ("dead_code", "duplication", "cycles", "boundaries", "smells"):
            score = row.pillar(name)
            if score.total:
                pillars.append(
                    f"{name}(tot:{score.total},new:{score.new},"
                    f"ex:{score.existing},inh:{score.inherent})"
                )
        tokens = row.integer("input_tokens") + row.integer("output_tokens")
        print(
            f"  {row.task_id}/{row.variant}/run_{row.run}: "
            f"qual_new={row.integer('new_quality_score')} "
            f"qual_inh={row.integer('inherent_quality_score')} "
            f"clone_tok={row.severity.clone_total_tokens} "
            f"clone_new={row.severity.clone_new_tokens} "
            f"cmplx={row.severity.complexity_max} "
            f"tok/line={tokens / max(1, row.diff_stats.lines_added):.0f} "
            f"[{', '.join(pillars)}]"
        )


def print_duplication_breakdown(rows: list[MetricRow]) -> None:
    print("\n## Duplication Breakdown (per clone)\n")
    print("| variant | task | run | tok | copies | provenance | hint |")
    print("| --- | --- | --- | --- | --- | --- | --- |")
    for row in sorted(rows, key=lambda item: (item.variant, item.task_id, item.run)):
        for detail in row.pillar("duplication").details:
            short_hint = detail.hint[:90]
            if len(detail.hint) > 90:
                short_hint += "..."
            print(
                f"| {row.variant} | {row.task_id} | {row.run} | "
                f"{detail.token_length} | {detail.copies} | "
                f"{detail.provenance} | {short_hint} |"
            )


def print_cost_summary(rows: list[MetricRow]) -> None:
    print("\n## Cost Summary\n")
    by_variant = defaultdict(list)
    for row in rows:
        by_variant[row.variant].append(row)
    print("| variant | total tok_in | total tok_out | total tokens | lines added | tok/line |")
    print("| --- | --- | --- | --- | --- | --- |")
    totals = {}
    for variant, variant_rows in sorted(by_variant.items()):
        input_tokens = sum(row.integer("input_tokens") for row in variant_rows)
        output_tokens = sum(row.integer("output_tokens") for row in variant_rows)
        lines = sum(row.diff_stats.lines_added for row in variant_rows)
        totals[variant] = input_tokens + output_tokens
        print(
            f"| {variant} | {input_tokens} | {output_tokens} | "
            f"{totals[variant]} | {lines} | {totals[variant] / max(1, lines):.0f} |"
        )
    if "control" in totals and "sensez" in totals:
        overhead = totals["sensez"] - totals["control"]
        percent = overhead * 100 / max(1, totals["control"])
        print(f"\nSensez overhead: **+{overhead} tokens** ({percent:.0f}% more than control)")


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: summarize.py <results-dir>")
    rows = load_metrics(Path(sys.argv[1]))
    grouped = defaultdict(list)
    for row in rows:
        grouped[row.variant].append(row)
    print("# Sensez A/B Summary\n")
    print_table(grouped)
    print_pairs(rows)
    print_per_task_detail(rows)
    print_duplication_breakdown(rows)
    print_cost_summary(rows)


if __name__ == "__main__":
    main()
