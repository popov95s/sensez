#!/usr/bin/env python3
"""Summarize Sensez A/B eval result directories.

Re-computes quality scores from raw sensez_diff.json so that improvements
to quality_score.py apply retroactively without re-running benchmarks.
"""

from __future__ import annotations

import json
import os
import sys
from collections import defaultdict
from pathlib import Path
from statistics import mean
from typing import Any

# Allow import from same directory regardless of cwd
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from quality_score import score_payload


def load_metrics(root: Path) -> list[dict[str, Any]]:
    metrics = []
    for path in sorted(root.glob("*/*/run_*/metrics.json")):
        row = json.loads(path.read_text())
        # Re-compute quality scores from the raw diff JSON so newer
        # scoring heuristics apply retroactively.
        diff_path = path.parent / "sensez_diff.json"
        if diff_path.exists():
            diff = json.loads(diff_path.read_text())
            quality = score_payload(diff.get("json", diff))
            row["quality_regression_score"] = quality["quality_regression_score"]
            row["new_quality_score"] = quality["new_quality_score"]
            row["existing_quality_score"] = quality["existing_quality_score"]
            row["inherent_quality_score"] = quality["inherent_quality_score"]
            row["quality_severity"] = quality["severity"]
            row["quality_by_pillar"] = quality["by_pillar"]
        metrics.append(row)
    return metrics


def average(values: list[float]) -> float:
    return round(mean(values), 3) if values else 0.0


def summarize_group(rows: list[dict[str, Any]]) -> dict[str, Any]:
    tested = [r for r in rows if "test_returncode" in r]
    return {
        "runs": len(rows),
        "agent_success_rate": average(
            [1.0 if r["agent_returncode"] == 0 else 0.0 for r in rows]
        ),
        "test_success_rate": average(
            [1.0 if r["test_returncode"] == 0 else 0.0 for r in tested]
        ),
        "avg_elapsed_seconds": average([r["agent_elapsed_seconds"] for r in rows]),
        "avg_diff_total": average([r["sensez_diff"]["total"] for r in rows]),
        "avg_qual_score": average(
            [r.get("quality_regression_score", 0) for r in rows]
        ),
        "avg_new_qual_score": average(
            [r.get("new_quality_score", 0) for r in rows]
        ),
        "avg_existing_qual_score": average(
            [r.get("existing_quality_score", 0) for r in rows]
        ),
        "avg_inherent_qual_score": average(
            [r.get("inherent_quality_score", 0) for r in rows]
        ),
        "avg_delta_total": average([r.get("sensez_delta_total", 0) for r in rows]),
        "avg_after_findings": average([r["sensez_after"]["total"] for r in rows]),
        "avg_tool_calls": average([r.get("sensez_tool_calls", 0) for r in rows]),
        "avg_input_tokens": average([r.get("input_tokens", 0) for r in rows]),
        "avg_output_tokens": average([r.get("output_tokens", 0) for r in rows]),
        "avg_total_tokens": average(
            [r.get("input_tokens", 0) + r.get("output_tokens", 0) for r in rows]
        ),
        "avg_tokens_per_line": average(
            [
                (r.get("input_tokens", 0) + r.get("output_tokens", 0))
                / max(1, r["diff_stats"]["lines_added"])
                for r in rows
            ]
        ),
        "avg_files_touched": average([r["diff_stats"]["files_touched"] for r in rows]),
        "avg_lines_added": average([r["diff_stats"]["lines_added"] for r in rows]),
        "avg_lines_deleted": average([r["diff_stats"]["lines_deleted"] for r in rows]),
        "avg_clone_tokens": average(
            [
                r.get("quality_severity", {}).get("clone_total_tokens", 0)
                for r in rows
            ]
        ),
        "avg_clone_new_tokens": average(
            [
                r.get("quality_severity", {}).get("clone_new_tokens", 0)
                for r in rows
            ]
        ),
        "avg_complexity_max": average(
            [
                r.get("quality_severity", {}).get("complexity_max", 0)
                for r in rows
            ]
        ),
    }


def print_table(grouped: dict[str, list[dict[str, Any]]]) -> None:
    headers = [
        "variant",
        "runs",
        "agent_ok%",
        "sec",
        "qual_new",
        "qual_ex",
        "qual_in",
        "tok_in",
        "tok_out",
        "tok_tot",
        "tok/line",
        "tool_calls",
        "+lines",
        "clone_tok",
        "clone_new",
        "complx",
    ]
    key_map = {
        "runs": "runs",
        "agent_ok%": "agent_success_rate",
        "sec": "avg_elapsed_seconds",
        "qual_new": "avg_new_qual_score",
        "qual_ex": "avg_existing_qual_score",
        "qual_in": "avg_inherent_qual_score",
        "tok_in": "avg_input_tokens",
        "tok_out": "avg_output_tokens",
        "tok_tot": "avg_total_tokens",
        "tok/line": "avg_tokens_per_line",
        "tool_calls": "avg_tool_calls",
        "+lines": "avg_lines_added",
        "clone_tok": "avg_clone_tokens",
        "clone_new": "avg_clone_new_tokens",
        "complx": "avg_complexity_max",
    }
    print("| " + " | ".join(headers) + " |")
    print("| " + " | ".join(["---"] * len(headers)) + " |")
    for variant, rows in sorted(grouped.items()):
        summary = summarize_group(rows)
        values = [variant] + [str(summary[key_map[h]]) for h in headers[1:]]
        print("| " + " | ".join(values) + " |")


def print_pairs(rows: list[dict[str, Any]]) -> None:
    by_pair: dict[tuple[str, int], dict[str, dict[str, Any]]] = defaultdict(dict)
    for row in rows:
        by_pair[(row["task_id"], row["run"])][row["variant"]] = row
    print("\n## Paired Deltas\n")
    print(
        "| task | run | qual_new | clone_tok | clone_new | "
        "complx | tok_delta | +lines | tool_calls |"
    )
    print("| --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    for (task_id, run), pair in sorted(by_pair.items()):
        if "control" not in pair or "sensez" not in pair:
            continue
        c = pair["control"]
        s = pair["sensez"]
        nq_delta = s.get("new_quality_score", 0) - c.get("new_quality_score", 0)
        ct_delta = (
            s.get("quality_severity", {}).get("clone_total_tokens", 0)
            - c.get("quality_severity", {}).get("clone_total_tokens", 0)
        )
        cn_delta = (
            s.get("quality_severity", {}).get("clone_new_tokens", 0)
            - c.get("quality_severity", {}).get("clone_new_tokens", 0)
        )
        cp_delta = (
            s.get("quality_severity", {}).get("complexity_max", 0)
            - c.get("quality_severity", {}).get("complexity_max", 0)
        )
        tok_delta = (
            s.get("input_tokens", 0) + s.get("output_tokens", 0)
        ) - (
            c.get("input_tokens", 0) + c.get("output_tokens", 0)
        )
        l_delta = (
            s["diff_stats"]["lines_added"] - c["diff_stats"]["lines_added"]
        )
        t_delta = s.get("sensez_tool_calls", 0) - c.get("sensez_tool_calls", 0)
        print(
            f"| {task_id} | {run} | {nq_delta:+d} | {ct_delta:+d} | "
            f"{cn_delta:+d} | {cp_delta:+d} | {tok_delta:+d} | {l_delta:+d} | {t_delta:+d} |"
        )


def print_per_task_detail(rows: list[dict[str, Any]]) -> None:
    """Print per-task detail showing new vs existing quality split."""
    print("\n## Per-Task Detail\n")
    for row in sorted(rows, key=lambda r: (r["task_id"], r["variant"], r["run"])):
        qp = row.get("quality_by_pillar", {})
        pillars_detail = []
        for pillar in ["dead_code", "duplication", "cycles", "boundaries", "smells"]:
            p = qp.get(pillar, {})
            if p.get("total", 0) > 0:
                parts = [f"tot:{p['total']}"]
                if p.get("new", 0):
                    parts.append(f"new:{p['new']}")
                if p.get("existing", 0):
                    parts.append(f"ex:{p['existing']}")
                if p.get("inherent", 0):
                    parts.append(f"inh:{p['inherent']}")
                pillars_detail.append(f"{pillar}({','.join(parts)})")
        sev = row.get("quality_severity", {})
        tok_in = row.get("input_tokens", 0)
        tok_out = row.get("output_tokens", 0)
        tok_line = (tok_in + tok_out) / max(1, row["diff_stats"]["lines_added"])
        print(
            f"  {row['task_id']}/{row['variant']}/run_{row['run']}: "
            f"qual_new={row.get('new_quality_score',0)} "
            f"qual_inh={row.get('inherent_quality_score',0)} "
            f"clone_tok={sev.get('clone_total_tokens',0)} "
            f"clone_new={sev.get('clone_new_tokens',0)} "
            f"cmplx={sev.get('complexity_max',0)} "
            f"tok_in={tok_in} tok_out={tok_out} tok/line={tok_line:.0f} "
            f"[{', '.join(pillars_detail)}]"
        )


def print_duplication_breakdown(rows: list[dict[str, Any]]) -> None:
    """Show per-clone provenance and token sizes."""
    print("\n## Duplication Breakdown (per clone)\n")
    print("| variant | task | run | tok | copies | provenance | hint |")
    print("| --- | --- | --- | --- | --- | --- | --- |")

    for row in sorted(rows, key=lambda r: (r["variant"], r["task_id"], r["run"])):
        qp = row.get("quality_by_pillar", {})
        dup = qp.get("duplication", {})
        for detail in dup.get("details", []):
            short_hint = detail["hint"][:90] + ("..." if len(detail["hint"]) > 90 else "")
            print(
                f"| {row['variant']} | {row['task_id']} | {row['run']} | "
                f"{detail['token_length']} | {detail['copies']} | "
                f"{detail['provenance']} | {short_hint} |"
            )


def print_cost_summary(rows: list[dict[str, Any]]) -> None:
    """Summarize the token cost of using Sensez vs control."""
    print("\n## Cost Summary\n")

    by_variant: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        by_variant[row["variant"]].append(row)

    print("| variant | total tok_in | total tok_out | total tokens | lines added | tok/line |")
    print("| --- | --- | --- | --- | --- | --- |")
    for variant in sorted(by_variant):
        rs = by_variant[variant]
        tok_in = sum(r.get("input_tokens", 0) for r in rs)
        tok_out = sum(r.get("output_tokens", 0) for r in rs)
        tot = tok_in + tok_out
        lines = sum(r["diff_stats"]["lines_added"] for r in rs)
        tpl = tot / max(1, lines)
        print(f"| {variant} | {tok_in} | {tok_out} | {tot} | {lines} | {tpl:.0f} |")

    if "control" in by_variant and "sensez" in by_variant:
        c_tot = sum(
            r.get("input_tokens", 0) + r.get("output_tokens", 0)
            for r in by_variant["control"]
        )
        s_tot = sum(
            r.get("input_tokens", 0) + r.get("output_tokens", 0)
            for r in by_variant["sensez"]
        )
        overhead = s_tot - c_tot
        pct = overhead * 100 / max(1, c_tot)
        print(
            f"\nSensez overhead: **+{overhead} tokens** "
            f"({pct:.0f}% more than control)"
        )
        print(
            "Driven by MCP tool calls: agents send scan results "
            "back to the model for analysis and fix iteration."
        )


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: summarize.py <results-dir>")
    root = Path(sys.argv[1])
    rows = load_metrics(root)
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        grouped[row["variant"]].append(row)
    print("# Sensez A/B Summary\n")
    print_table(grouped)
    print_pairs(rows)
    print_per_task_detail(rows)
    print_duplication_breakdown(rows)
    print_cost_summary(rows)


if __name__ == "__main__":
    main()
