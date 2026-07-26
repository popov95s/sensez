"""Typed domain models shared by the Sensez evaluation scripts."""

from __future__ import annotations

from dataclasses import dataclass


PILLARS = ("cycles", "dead_code", "boundaries", "duplication", "smells")
MISSING_TEXT = ""


@dataclass(frozen=True)
class JsonFields:
    value: object

    def get(self, key: str) -> object:
        if isinstance(self.value, dict):
            return self.value.get(key)
        return None

    def integer(self, key: str, default: int = 0) -> int:
        return _integer(self.get(key), default)

    def number(self, key: str, default: float = 0.0) -> float:
        value = self.get(key)
        return float(value) if isinstance(value, (int, float)) else default

    def text(self, key: str, default: str = MISSING_TEXT) -> str:
        return _text(self.get(key), default)


def _mapping(value: object) -> JsonFields:
    return JsonFields(value)


def _integer(value: object, default: int = 0) -> int:
    return value if isinstance(value, int) else default


def _text(value: object, default: str = MISSING_TEXT) -> str:
    return value if isinstance(value, str) else default


@dataclass(frozen=True)
class Occurrence:
    file: str
    start_row: int

    @classmethod
    def parse(cls, value: object) -> Occurrence:
        data = _mapping(value)
        return cls(_text(data.get("file")), _integer(data.get("start_row")))


@dataclass(frozen=True)
class Finding:
    reason: str = ""
    hint: str = ""
    token_length: int = 0
    kind: str = ""
    metric: int = 0
    occurrences: tuple[Occurrence, ...] = ()

    @classmethod
    def parse(cls, value: object) -> Finding:
        data = _mapping(value)
        occurrences = data.get("occurrences")
        return cls(
            reason=_text(data.get("reason")),
            hint=_text(data.get("hint")),
            token_length=_integer(data.get("token_length")),
            kind=_text(data.get("kind")),
            metric=_integer(data.get("metric")),
            occurrences=tuple(Occurrence.parse(item) for item in occurrences)
            if isinstance(occurrences, list)
            else (),
        )


@dataclass(frozen=True)
class ScanPayload:
    cycles: tuple[Finding, ...] = ()
    dead_code: tuple[Finding, ...] = ()
    boundaries: tuple[Finding, ...] = ()
    duplication: tuple[Finding, ...] = ()
    smells: tuple[Finding, ...] = ()

    @classmethod
    def parse(cls, value: object) -> ScanPayload:
        data = _mapping(value)

        def findings(name: str) -> tuple[Finding, ...]:
            raw = data.get(name)
            return tuple(Finding.parse(item) for item in raw) if isinstance(raw, list) else ()

        return cls(*(findings(name) for name in PILLARS))

    def for_pillar(self, pillar: str) -> tuple[Finding, ...]:
        return getattr(self, pillar)


@dataclass(frozen=True)
class DuplicationDetail:
    token_length: int
    copies: int
    provenance: str
    hint: str

    @classmethod
    def parse(cls, value: object) -> DuplicationDetail:
        fields = JsonFields(value)
        return cls(
            fields.integer("token_length"),
            fields.integer("copies"),
            fields.text("provenance"),
            fields.text("hint"),
        )


@dataclass(frozen=True)
class PillarScore:
    total: int
    new: int
    existing: int
    inherent: int
    weight: int
    new_score: int
    existing_score: int
    inherent_score: int
    details: tuple[DuplicationDetail, ...] = ()

    @classmethod
    def parse(cls, value: object) -> PillarScore:
        fields = JsonFields(value)
        details = fields.get("details")
        return cls(
            total=fields.integer("total"),
            new=fields.integer("new"),
            existing=fields.integer("existing"),
            inherent=fields.integer("inherent"),
            weight=fields.integer("weight"),
            new_score=fields.integer("new_score"),
            existing_score=fields.integer("existing_score"),
            inherent_score=fields.integer("inherent_score"),
            details=tuple(DuplicationDetail.parse(item) for item in details)
            if isinstance(details, list)
            else (),
        )


@dataclass(frozen=True)
class SeverityMetrics:
    clone_total_tokens: int = 0
    clone_max_tokens: int = 0
    clone_avg_tokens: float = 0.0
    clone_new_tokens: int = 0
    clone_inherent_tokens: int = 0
    clone_total_copies: int = 0
    complexity_max: int = 0
    complexity_avg: float = 0.0
    complexity_count: int = 0
    mutated_param_count: int = 0

    @classmethod
    def parse(cls, value: object) -> SeverityMetrics:
        fields = JsonFields(value)
        return cls(
            clone_total_tokens=fields.integer("clone_total_tokens"),
            clone_max_tokens=fields.integer("clone_max_tokens"),
            clone_avg_tokens=fields.number("clone_avg_tokens"),
            clone_new_tokens=fields.integer("clone_new_tokens"),
            clone_inherent_tokens=fields.integer("clone_inherent_tokens"),
            clone_total_copies=fields.integer("clone_total_copies"),
            complexity_max=fields.integer("complexity_max"),
            complexity_avg=fields.number("complexity_avg"),
            complexity_count=fields.integer("complexity_count"),
            mutated_param_count=fields.integer("mutated_param_count"),
        )


@dataclass(frozen=True)
class QualityScore:
    quality_regression_score: int
    new_quality_score: int
    existing_quality_score: int
    inherent_quality_score: int
    by_pillar: dict[str, PillarScore]
    severity: SeverityMetrics


@dataclass(frozen=True)
class FindingCounts:
    cycles: int = 0
    dead_code: int = 0
    boundaries: int = 0
    duplication: int = 0
    smells: int = 0

    @property
    def total(self) -> int:
        return sum(getattr(self, pillar) for pillar in PILLARS)

    @classmethod
    def from_scan(cls, scan: ScanPayload) -> FindingCounts:
        return cls(*(len(scan.for_pillar(pillar)) for pillar in PILLARS))

    @classmethod
    def parse(cls, value: object) -> FindingCounts:
        fields = JsonFields(value)
        return cls(*(fields.integer(pillar) for pillar in PILLARS))


@dataclass(frozen=True)
class DiffStats:
    files: tuple[str, ...]
    files_touched: int
    lines_added: int
    lines_deleted: int

    @classmethod
    def parse(cls, value: object) -> DiffStats:
        fields = JsonFields(value)
        files = fields.get("files")
        return cls(
            tuple(item for item in files if isinstance(item, str))
            if isinstance(files, list)
            else (),
            fields.integer("files_touched"),
            fields.integer("lines_added"),
            fields.integer("lines_deleted"),
        )


@dataclass(frozen=True)
class TokenUsage:
    input: int = 0
    output: int = 0
    reasoning: int = 0
    total: int = 0


@dataclass(frozen=True)
class TaskSpec:
    id: str
    repo: str
    base_commit: str
    category: str
    summary: str
    test_command: str = ""

    @classmethod
    def parse(cls, value: object) -> TaskSpec:
        data = _mapping(value)
        return cls(
            id=_text(data.get("id")),
            repo=_text(data.get("repo")),
            base_commit=_text(data.get("base_commit")),
            category=_text(data.get("category")),
            summary=_text(data.get("summary")),
            test_command=_text(data.get("test_command")),
        )
