"""Typed access to historical evaluation metric rows."""

from __future__ import annotations

from dataclasses import dataclass

from models import (
    DiffStats,
    FindingCounts,
    JsonFields,
    PillarScore,
    QualityScore,
    SeverityMetrics,
)

@dataclass(frozen=True)
class MetricRow:
    fields: JsonFields
    recomputed_quality: QualityScore | None = None

    @classmethod
    def parse(cls, value: object) -> MetricRow:
        return cls(JsonFields(value))

    def with_quality(self, quality: QualityScore) -> MetricRow:
        return MetricRow(self.fields, quality)

    @property
    def task_id(self) -> str:
        return self.fields.text("task_id")

    @property
    def variant(self) -> str:
        return self.fields.text("variant")

    @property
    def run(self) -> int:
        return self.fields.integer("run")

    @property
    def agent_returncode(self) -> int:
        return self.fields.integer("agent_returncode")

    @property
    def test_returncode(self) -> int | None:
        value = self.fields.get("test_returncode")
        return value if isinstance(value, int) else None

    @property
    def agent_elapsed_seconds(self) -> float:
        return self.fields.number("agent_elapsed_seconds")

    def integer(self, name: str) -> int:
        if self.recomputed_quality:
            quality_values = {
                "quality_regression_score": self.recomputed_quality.quality_regression_score,
                "new_quality_score": self.recomputed_quality.new_quality_score,
                "existing_quality_score": self.recomputed_quality.existing_quality_score,
                "inherent_quality_score": self.recomputed_quality.inherent_quality_score,
            }
            if name in quality_values:
                return quality_values[name]
        return self.fields.integer(name)

    def counts(self, name: str) -> FindingCounts:
        return FindingCounts.parse(self.fields.get(name))

    @property
    def diff_stats(self) -> DiffStats:
        return DiffStats.parse(self.fields.get("diff_stats"))

    @property
    def severity(self) -> SeverityMetrics:
        if self.recomputed_quality:
            return self.recomputed_quality.severity
        return SeverityMetrics.parse(self.fields.get("quality_severity"))

    def pillar(self, name: str) -> PillarScore:
        if self.recomputed_quality:
            return self.recomputed_quality.by_pillar.get(
                name, empty_pillar()
            )
        all_pillars = JsonFields(self.fields.get("quality_by_pillar"))
        return PillarScore.parse(all_pillars.get(name))


def empty_pillar() -> PillarScore:
    return PillarScore(0, 0, 0, 0, 0, 0, 0, 0)


@dataclass(frozen=True)
class GroupSummary:
    runs: int
    agent_success_rate: float
    test_success_rate: float
    avg_elapsed_seconds: float
    avg_diff_total: float
    avg_qual_score: float
    avg_new_qual_score: float
    avg_existing_qual_score: float
    avg_inherent_qual_score: float
    avg_delta_total: float
    avg_after_findings: float
    avg_tool_calls: float
    avg_input_tokens: float
    avg_output_tokens: float
    avg_total_tokens: float
    avg_tokens_per_line: float
    avg_files_touched: float
    avg_lines_added: float
    avg_lines_deleted: float
    avg_clone_tokens: float
    avg_clone_new_tokens: float
    avg_complexity_max: float
