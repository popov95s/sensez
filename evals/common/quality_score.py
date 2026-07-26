#!/usr/bin/env python3
"""Compute typed quality-regression metrics from Sensez diff JSON."""

from __future__ import annotations

import argparse
import json
from dataclasses import asdict
from pathlib import Path

from models import (
    DuplicationDetail,
    Finding,
    PillarScore,
    QualityScore,
    ScanPayload,
    SeverityMetrics,
)


WEIGHTS = {
    "boundaries": 5,
    "cycles": 4,
    "duplication": 3,
    "dead_code": 2,
    "smells": 1,
}


def _is_new_dead_code(finding: Finding) -> bool:
    return finding.reason == "added_unreferenced"


def _is_new_duplication(finding: Finding) -> bool:
    return "copies written in this change" in finding.hint


def _is_pre_existing_duplication(finding: Finding) -> bool:
    return "clone of existing code" in finding.hint


def _is_inherent_duplication(finding: Finding) -> bool:
    """Identify distant same-file mirror implementations."""
    if _is_pre_existing_duplication(finding) or len(finding.occurrences) != 2:
        return False
    if len({occurrence.file for occurrence in finding.occurrences}) != 1:
        return False
    lines = sorted(occurrence.start_row for occurrence in finding.occurrences)
    return lines[1] - lines[0] > 200


def _provenance(finding: Finding) -> str:
    if _is_inherent_duplication(finding):
        return "inherent"
    if _is_new_duplication(finding):
        return "new"
    if _is_pre_existing_duplication(finding):
        return "pre-existing"
    return "unknown"


def _score_pillar(pillar: str, items: tuple[Finding, ...], weight: int) -> PillarScore:
    new_count = inherent_count = 0
    details = []
    for finding in items:
        if pillar == "dead_code":
            new_count += _is_new_dead_code(finding)
        elif pillar == "duplication":
            provenance = _provenance(finding)
            new_count += provenance == "new"
            inherent_count += provenance == "inherent"
            details.append(
                DuplicationDetail(
                    token_length=finding.token_length,
                    copies=len(finding.occurrences),
                    provenance=provenance,
                    hint=finding.hint,
                )
            )
    existing_count = len(items) - new_count - inherent_count
    return PillarScore(
        total=len(items),
        new=new_count,
        existing=existing_count,
        inherent=inherent_count,
        weight=weight,
        new_score=new_count * weight,
        existing_score=existing_count * weight,
        inherent_score=inherent_count * weight,
        details=tuple(details),
    )


def score_payload(payload: ScanPayload) -> QualityScore:
    by_pillar = {
        pillar: _score_pillar(pillar, payload.for_pillar(pillar), weight)
        for pillar, weight in WEIGHTS.items()
    }
    total_new = sum(score.new_score for score in by_pillar.values())
    total_existing = sum(score.existing_score for score in by_pillar.values())
    total_inherent = sum(score.inherent_score for score in by_pillar.values())
    return QualityScore(
        quality_regression_score=total_new + total_existing,
        new_quality_score=total_new,
        existing_quality_score=total_existing,
        inherent_quality_score=total_inherent,
        by_pillar=by_pillar,
        severity=_extract_severity_metrics(payload),
    )


def _extract_severity_metrics(payload: ScanPayload) -> SeverityMetrics:
    clone_total = clone_max = clone_copies = 0
    new_tokens = inherent_tokens = 0
    for finding in payload.duplication:
        clone_total += finding.token_length
        clone_max = max(clone_max, finding.token_length)
        clone_copies += len(finding.occurrences)
        provenance = _provenance(finding)
        if provenance == "new":
            new_tokens += finding.token_length
        elif provenance == "inherent":
            inherent_tokens += finding.token_length

    complexity_total = complexity_max = complexity_count = mutated_count = 0
    for finding in payload.smells:
        if finding.kind == "high_cognitive_complexity":
            complexity_total += finding.metric
            complexity_max = max(complexity_max, finding.metric)
            complexity_count += 1
        elif finding.kind == "mutated_parameter":
            mutated_count += 1
    return SeverityMetrics(
        clone_total_tokens=clone_total,
        clone_max_tokens=clone_max,
        clone_avg_tokens=round(clone_total / len(payload.duplication), 1)
        if payload.duplication
        else 0.0,
        clone_new_tokens=new_tokens,
        clone_inherent_tokens=inherent_tokens,
        clone_total_copies=clone_copies,
        complexity_max=complexity_max,
        complexity_avg=round(complexity_total / complexity_count, 1)
        if complexity_count
        else 0.0,
        complexity_count=complexity_count,
        mutated_param_count=mutated_count,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("sensez_diff_json", type=Path)
    args = parser.parse_args()
    raw = json.loads(args.sensez_diff_json.read_text())
    envelope = raw.get("json") if isinstance(raw, dict) else None
    payload = ScanPayload.parse(envelope if envelope is not None else raw)
    print(json.dumps(asdict(score_payload(payload)), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
