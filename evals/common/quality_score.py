#!/usr/bin/env python3
"""Compute a quality-regression score from Sensez diff JSON.

Splits findings into newly-introduced vs pre-existing (touched) using
the `reason`/`hint` fields that the diff filter sets. Also extracts
severity-level metrics (clone sizes, cognitive complexity) for
granular comparison between control and Sensez variants.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


WEIGHTS = {
    "boundaries": 5,
    "cycles": 4,
    "duplication": 3,
    "dead_code": 2,
    "smells": 1,
}


def _is_new_dead_code(finding: dict[str, Any]) -> bool:
    return finding.get("reason", "") == "added_unreferenced"


def _is_new_duplication(finding: dict[str, Any]) -> bool:
    hint = finding.get("hint", "")
    return "N copies written in this change" in hint or "copies written in this change" in hint


def _is_pre_existing_duplication(finding: dict[str, Any]) -> bool:
    hint = finding.get("hint", "")
    return "clone of existing code" in hint


def _is_inherent_duplication(finding: dict[str, Any]) -> bool:
    """Heuristic: duplication between distant line ranges in the same file
    is likely a sync/async or architectural mirror, not a lazy copy-paste.

    Only applies to clones where ALL occurrences are within the diff
    (otherwise `hint` would say 'clone of existing code').
    """
    if _is_pre_existing_duplication(finding):
        return False
    occurrences = finding.get("occurrences", [])
    if len(occurrences) != 2:
        return False
    files = {o.get("file", "") for o in occurrences}
    if len(files) != 1:
        return False
    lines = sorted(o.get("start_row", 0) for o in occurrences)
    # If the two copies are >200 lines apart in the same file, they're
    # almost certainly separate methods (e.g. generate vs agenerate).
    return (lines[1] - lines[0]) > 200


def score_payload(payload: dict[str, Any]) -> dict[str, Any]:
    by_pillar = {}
    total_new = 0
    total_existing = 0
    total_inherent = 0

    for pillar, weight in WEIGHTS.items():
        items = payload.get(pillar) or []
        new_count = 0
        existing_count = 0
        inherent_count = 0

        if pillar == "dead_code":
            new_count = sum(1 for f in items if _is_new_dead_code(f))
            existing_count = len(items) - new_count
        elif pillar == "duplication":
            inherent_count = sum(1 for f in items if _is_inherent_duplication(f))
            new_count = sum(
                1 for f in items
                if _is_new_duplication(f) and not _is_inherent_duplication(f)
            )
            existing_count = len(items) - new_count - inherent_count
        elif pillar == "smells":
            existing_count = len(items)
        else:
            existing_count = len(items)

        pillar_data = {
            "total": len(items),
            "new": new_count,
            "existing": existing_count,
            "inherent": inherent_count,
            "weight": weight,
            "new_score": new_count * weight,
            "existing_score": existing_count * weight,
            "inherent_score": inherent_count * weight,
        }
        if pillar == "duplication":
            dup_details = []
            for f in items:
                if _is_inherent_duplication(f):
                    provenance = "inherent"
                elif _is_new_duplication(f):
                    provenance = "new"
                elif _is_pre_existing_duplication(f):
                    provenance = "pre-existing"
                else:
                    provenance = "unknown"
                dup_details.append({
                    "token_length": f.get("token_length", 0),
                    "copies": len(f.get("occurrences", [])),
                    "provenance": provenance,
                    "hint": f.get("hint", ""),
                })
            pillar_data["details"] = dup_details
        by_pillar[pillar] = pillar_data
        total_new += new_count * weight
        total_existing += existing_count * weight
        total_inherent += inherent_count * weight

    severity = _extract_severity_metrics(payload)

    return {
        "quality_regression_score": total_new + total_existing,
        "new_quality_score": total_new,
        "existing_quality_score": total_existing,
        "inherent_quality_score": total_inherent,
        "by_pillar": by_pillar,
        "severity": severity,
    }


def _extract_severity_metrics(payload: dict[str, Any]) -> dict[str, Any]:
    """Extract granular severity metrics for comparison across variants."""
    severity: dict[str, Any] = {}

    duplication = payload.get("duplication") or []
    if duplication:
        new_dups = [d for d in duplication if not _is_inherent_duplication(d) and not _is_pre_existing_duplication(d)]
        inherent_dups = [d for d in duplication if _is_inherent_duplication(d)]
        clone_sizes = [d.get("token_length", 0) for d in duplication]
        severity["clone_total_tokens"] = sum(clone_sizes)
        severity["clone_max_tokens"] = max(clone_sizes) if clone_sizes else 0
        severity["clone_avg_tokens"] = (
            round(sum(clone_sizes) / len(clone_sizes), 1) if clone_sizes else 0
        )
        severity["clone_new_tokens"] = sum(
            d.get("token_length", 0) for d in new_dups
        )
        severity["clone_inherent_tokens"] = sum(
            d.get("token_length", 0) for d in inherent_dups
        )
        all_copies = sum(len(d.get("occurrences", [])) for d in duplication)
        severity["clone_total_copies"] = all_copies

    smells = payload.get("smells") or []
    complexity_metrics = [
        s.get("metric", 0)
        for s in smells
        if s.get("kind") == "high_cognitive_complexity"
    ]
    if complexity_metrics:
        severity["complexity_max"] = max(complexity_metrics)
        severity["complexity_avg"] = round(
            sum(complexity_metrics) / len(complexity_metrics), 1
        )
        severity["complexity_count"] = len(complexity_metrics)

    mutated_params = [
        s for s in smells if s.get("kind") == "mutated_parameter"
    ]
    if mutated_params:
        severity["mutated_param_count"] = len(mutated_params)

    return severity


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("sensez_diff_json", type=Path)
    args = parser.parse_args()
    scan = json.loads(args.sensez_diff_json.read_text())
    print(json.dumps(score_payload(scan.get("json") or scan), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
